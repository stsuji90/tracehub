//! 公開するデータの型(設計書 §5)。

use crate::ids::{ClassId, EventId};

/// 対応している QNX の版
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QnxVersion {
    V800,
}

impl QnxVersion {
    /// `TRACE_SYS_RELEASE` の値(例: "8.0.0")から版を決める
    pub fn from_release(release: &str) -> Option<QnxVersion> {
        let mut it = release.trim().split('.');
        match (it.next(), it.next()) {
            (Some("8"), Some("0")) => Some(QnxVersion::V800),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            QnxVersion::V800 => "8.0",
        }
    }
}

/// kev ファイル全体
#[derive(Debug, Clone)]
pub struct KevFile {
    pub header: KevHeader,
    /// traceprinter の並び(merge と combine の結果の順)
    pub events: Vec<TraceEvent>,
    /// 見つかった異常(時刻の順)
    pub diag: Vec<DiagInfo>,
}

/// kev のヘッダ
#[derive(Debug, Clone)]
pub struct KevHeader {
    pub version: QnxVersion,
    /// `TRACE_CYCLES_PER_SEC`(無い・数値でなければ 0)
    pub cycles_per_sec: u64,
    /// `TRACE_CPU_NUM`(無い・数値でなければ 0)
    pub cpu_num: u32,
    /// `TRACE_SYSPAGE_LEN`(無い・空なら 0)
    pub syspage_len: u64,
    /// スロットのバイト順(`TRACE_LITTLE_ENDIAN` があればリトル、無く `TRACE_BIG_ENDIAN` があればビッグ)
    pub little_endian: bool,
    /// キー(`TRACE_` を含み `::` を含まない)と値。kev に書かれた順。同じキーが複数あればすべて持つ
    pub keywords: Vec<(String, String)>,
}

impl KevHeader {
    /// キーの値(同じキーが複数あれば最後のもの)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keywords
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// イベント 1 件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    /// CONTROL の TIME だけで上位 32 ビットを補った時刻。組は CB の時刻
    pub timestamp_cycles: u64,
    pub cpu_id: u8,
    /// セマンティック ID
    pub class_id: ClassId,
    pub event_id: EventId,
    /// kev に書かれた内部クラス
    pub int_class_id: u8,
    /// kev に書かれた内部イベント番号
    pub int_event_id: u16,
    pub combine_status: CombineStatus,
    pub display_status: DisplayStatus,
    /// Complete: 各スロットの payloads(word[2]・word[3])をつないだもの。
    /// Truncated・Orphan: 各スロットの生の 4 words(header を含む)をつないだもの
    pub payloads: Vec<u32>,
}

impl TraceEvent {
    /// 解釈に使う payloads(Truncated・Orphan では生の words から word[2]・word[3] を取り出す)
    pub fn payload_words(&self) -> Vec<u32> {
        match self.combine_status {
            CombineStatus::Complete => self.payloads.clone(),
            _ => self
                .payloads
                .chunks(4)
                .flat_map(|s| s.iter().skip(2).copied())
                .collect(),
        }
    }
}

/// 組み立ての結果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombineStatus {
    /// S のスロット 1 つのイベント、または CB から CE までそろった組
    Complete,
    /// CE が来ないまま途切れた組
    Truncated,
    /// 組み立て中の組が無いのに来た CC / CE のスロット 1 つ
    Orphan,
}

/// traceprinter が何かを出力するか
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayStatus {
    /// イベントの行、または診断の行(TRUNC・ORPHAN・UNKNOW)を出力する
    Display,
    /// 何も出力しない
    Silent,
}

/// 見つかった異常 1 件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagInfo {
    /// 異常が確定したスロットの時刻(CONTROL の TIME だけで補った値)
    pub timestamp_cycles: u64,
    pub cpu_id: u8,
    pub kind: DiagKind,
}

/// 異常の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagKind {
    /// 組み立て中の組が無い CC / CE
    Orphan {
        /// `KevFile::events` の番号
        event_index: usize,
        /// traceprinter が表示する番号(merge した順のスロットの通し番号。1 から)
        slot_number: u64,
        /// traceprinter が ORPHAN 行を出す位置(この番号の `KevFile::events` の行の前。末尾なら events の長さ)
        display_before: usize,
    },
    /// 途切れた組
    Truncated {
        /// `KevFile::events` の番号
        event_index: usize,
        /// traceprinter が表示する番号(orphan を除くイベントの通し番号。1 から)
        event_number: u64,
        reason: TruncateReason,
    },
    /// BUFFER の通番が、同じ CPU の前の BUFFER の通番 + 1 でない
    SeqGap { last_seq: u32, seq: u32 },
    /// BUFFER に書かれたスロット数と、実際の数(TIME から BUFFER_END まで)が違う
    CountMismatch { seq: u32, declared: u32, actual: u32 },
    /// DROPPED(失われたイベントの数)
    Dropped { count: u32 },
    /// BUFFER_END が来ないまま次の BUFFER が来た、または列が終わった
    MissingBufferEnd { seq: u32 },
    /// BUFFER_END の payloads が cookie でない
    CookieMismatch { seq: u32, values: [u32; 2] },
    /// TIME が無いのに時刻の下位 32 ビットが一周した
    WrapWithoutTime { prev_low32: u32, low32: u32 },
    /// トレースバッファの途中に別の CPU のスロットがある
    Interleaved { seq: u32, other_cpu: u8 },
    /// トレースバッファの外のスロット(続いた範囲ごとに 1 件)
    OutsideBuffer { slots: u64 },
    /// ファイルの末尾の、16 バイトに満たない端数
    TrailingBytes { bytes: u64 },
}

/// 組が途切れた理由
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TruncateReason {
    /// 同じ鍵の CB が来た
    SameKeyCb,
    /// 先頭にあるまま、後ろに窓の数だけ項目がたまった
    Window,
    /// 列が終わった
    EndOfStream,
}

/// kev を読めない理由
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KevError {
    /// `TRACE_HEADER_END::` が無い
    NoHeaderEnd,
    /// `TRACE_VER_MAJOR` / `TRACE_VER_MINOR` が無い、または対応していない値
    BadFormatVersion { key: String, value: Option<String> },
    /// ヘッダの項目(キー::値)が長すぎる
    HeaderItemTooLong { key: String, len: usize },
    /// `TRACE_SYSPAGE_LEN` の分のバイトが無い
    SyspageTooShort { expected: u64, actual: u64 },
    /// `TRACE_SYS_RELEASE` が無い、または対応していない版
    UnsupportedVersion { release: Option<String> },
}

impl std::fmt::Display for KevError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KevError::NoHeaderEnd => write!(f, "ヘッダの終わり(TRACE_HEADER_END::)がありません"),
            KevError::BadFormatVersion { key, value: None } => write!(f, "ヘッダに {key} がありません"),
            KevError::BadFormatVersion { key, value: Some(v) } => write!(f, "{key} が対応していない値です: {v}"),
            KevError::HeaderItemTooLong { key, len } => write!(f, "ヘッダの項目 {key} が長すぎます({len} バイト)"),
            KevError::SyspageTooShort { expected, actual } => {
                write!(f, "syspage が足りません({expected} バイトのはずが {actual} バイト)")
            }
            KevError::UnsupportedVersion { release: None } => write!(f, "ヘッダに TRACE_SYS_RELEASE がありません"),
            KevError::UnsupportedVersion { release: Some(r) } => write!(f, "対応していない QNX の版です: {r}"),
        }
    }
}

impl std::error::Error for KevError {}

/// decode() の設定
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DecodeOptions {
    pub missing: MissingStyle,
}

/// 欠けた payloads の表し方
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MissingStyle {
    /// traceprinter と同じ
    #[default]
    Traceprinter,
    /// 欠けた引数を `(truncated)` / `(missing)` と書く
    Marked,
}

/// decode() の結果(traceprinter -n と同じ書式の文字列)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedEvent {
    /// 表示クラス名(例: "KER_EXIT")
    pub class_name: String,
    /// 表示イベント名(例: "MSG_SENDV/11")
    pub event_name: String,
    /// 名前より後ろ(先頭の区切りの空白を含む)
    pub args: String,
}
