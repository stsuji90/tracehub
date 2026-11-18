# tracehub 設計書

## 1. 目的と範囲

tracehub は、いろいろな形式のトレース情報を読み、分析しやすい形式に変換して出力するプログラムである。

第 1 段階では、QNX のカーネルトレース(kev ファイル)だけを扱う。

| 項目 | 第 1 段階 | 後の段階 |
|---|---|---|
| 入力 | kev(QNX 8.0) | ほかの QNX の版、ほかのトレース形式 |
| ライブラリの出力 | `KevFile`(§5)、`decode()` による文字列(§7) | 別アプリ向けのデータ形式 |
| CLI の出力 | traceprinter 互換のテキスト(`-n` 相当) | 人が読むための出力など |

### 前提

kev の解釈は公式に公開されていない。tracehub の解釈は、traceprinter(QNX 公式のツール)の出力を正として、その振る舞いから決めたものである。

- traceprinter が出力するものは、tracehub も出力する(traceprinter が出すのに tracehub が出さない、を作らない)。
- traceprinter が失敗する入力(異常終了・内部エラー)での振る舞いは決めない(実装の成り行きでよい)。

---

## 2. 用語

| 用語 | 意味 |
|---|---|
| スロット(slot) | kev のイベント部の単位。u32 × 4 = 16 バイト |
| word | スロットの u32 の 1 つ。意味を問わないときの呼び名 |
| header | スロットの word[0]。構造・CPU・フラグ・内部クラス・内部イベント番号のビット列 |
| timestamp_cycles_low32 | スロットの word[1]。時刻(クロック値)の下位 32 ビット |
| payloads | 解釈する前の u32 の列。スロットでは word[2]・word[3]。組では各スロットの payloads をつないだもの |
| args | payloads を解釈して意味を持たせたもの(名前と値、文字列) |
| 構造(struct) | header のビット 30〜31。S(単独)/ CB(組の始まり)/ CC(組の途中)/ CE(組の終わり) |
| 組(複合イベント) | CB, CC…, CE の複数スロットで 1 つのイベントになるもの |
| トレースバッファ | CPU ごとのバッファ。kev では、同じ CPU のスロットが連続した範囲で、TIME・BUFFER で始まり BUFFER_END で終わる |
| CPU 別スロット列 | ある CPU のスロットを、ファイルの順に並べたもの |
| reconstruct_timestamp | CPU 別スロット列で、timestamp_cycles_low32 に上位 32 ビットを補って 64 ビットの時刻にする処理(§6.3) |
| merge | CPU 別スロット列を、時刻の順に 1 列にまとめる処理(§6.4) |
| combine | merge した列のスロットから、組を組み立てる処理(§6.5) |
| 内部 ID | kev に書かれている番号。内部クラス ID(`int_class_id`)と内部イベント ID(`int_event_id`)。QNX の版に依存する |
| セマンティック ID | tracehub が決める、QNX の版に依存しない番号。クラス ID(`class_id`)とイベント ID(`event_id`)(§8) |
| 診断(DiagInfo) | 解析の途中で見つかった、組み立ての異常とトレースバッファの不整合(§5.4) |
| traceprinter 互換出力 | `traceprinter -n -f X.kev -o OUT` が OUT に書く内容と、バイト単位で同じ出力(§9) |

---

## 3. 構成

```
tracehub/
  Cargo.toml                 ワークスペース
  crates/
    tracehub-kev/            kev に関するすべての処理(ライブラリ)
      src/
        lib.rs               公開 API
        model.rs             KevFile・TraceEvent・DiagInfo などの型
        ids.rs               セマンティック ID(版に依存しない)
        kev_header.rs        kev のヘッダの読み取り(版に共通)
        kev_syspage.rs       syspage の読み飛ばし(版に共通)
        qnx800/              QNX 8.0 に依存する部分
          mod.rs
          def.rs             内部 ID の定義、内部 ID → セマンティック ID の対応、表示名
          args.rs            イベントごとの payloads の解釈と表示(traceprinter の書式)
          event_parser.rs    スロットの読み取りからイベントの列まで(§6)
          event_decoder.rs   TraceEvent → クラス・イベント・args の文字列(§7)
          traceprinter.rs    traceprinter 互換出力(§9)
    tracehub-cli/            CLI(tracehub コマンド)
  tools/
    kevedit/                 試験用の kev を CSV から作る道具(kevedit.html、kevcsv.py)
  docs/
    design.md                この文書
    qnx-version-guide.md     QNX の版の定義をそろえる手順(別の作業者向け)
```

### 版に依存する部分と依存しない部分

| 版に共通(と仮定する) | 版に依存する |
|---|---|
| ヘッダの書式、syspage の位置、スロットの大きさ、header のビット配置、トレースバッファの枠(TIME・BUFFER・BUFFER_END)、merge と combine の規則 | 内部 ID の値、内部 ID → セマンティック ID の対応、payloads の解釈、traceprinter の表示文字列 |

- 定義(def.rs・args.rs)とロジックを分ける。
- 版を足すときは、`qnxNNN/` を足して、版の enum(§4)に 1 つ加える。
- event_parser.rs のうち版に共通な規則は、版を足すときに共通のモジュールへ移してよい(第 1 段階では qnx800 の中に置く)。

---

## 4. 公開 API

```rust
/// kev を読む。ヘッダの誤りと、対応していない版はエラー
pub fn parse(bytes: &[u8]) -> Result<KevFile, KevError>;

/// TraceEvent 1 件の表示文字列(traceprinter -n と同じ書式)
pub fn decode(version: QnxVersion, event: &TraceEvent, opts: &DecodeOptions) -> DecodedEvent;

/// セマンティック ID の表示文字列(traceprinter が表示するクラス名・イベント名)
pub fn class_name(version: QnxVersion, class_id: ClassId) -> Option<&'static str>;
pub fn event_name(version: QnxVersion, class_id: ClassId, event_id: EventId) -> Option<String>;

/// traceprinter 互換出力
pub mod traceprinter {
    pub fn write(kev: &KevFile, out: &mut impl std::io::Write) -> std::io::Result<()>;
}
```

| 型 | 内容 |
|---|---|
| `QnxVersion` | 対応する QNX の版の enum。第 1 段階は `V800` だけ。`KevHeader` の `TRACE_SYS_RELEASE` から決まる |
| `DecodeOptions` | `missing`: 欠けた payloads の表し方(§7.2) |
| `DecodedEvent` | `class_name`・`event_name`・`args`(いずれも文字列) |
| `KevError` | 読めない理由(§10) |

- `event_name` が `String` を返すのは、KER_CALL の表示名(`MSG_SENDV/11` など)が番号を含むため。
- ファイルを読む関数は持たない(呼ぶ側がバイト列を渡す)。

---

## 5. データモデル

### 5.1 KevFile

```rust
pub struct KevFile {
    pub header: KevHeader,
    pub events: Vec<TraceEvent>,   // traceprinter の並び(merge と combine の結果の順)
    pub diag: Vec<DiagInfo>,       // 見つかった順
}
```

### 5.2 KevHeader

```rust
pub struct KevHeader {
    pub version: QnxVersion,
    pub cycles_per_sec: u64,
    pub cpu_num: u32,
    pub syspage_len: u64,
    pub keywords: Vec<(String, String)>,  // キー(TRACE_ を含み :: を含まない)と値。kev に書かれた順
}
```

- keywords は kev に書かれた順で持つ。traceprinter 互換出力のヘッダ部は、この順に書く。
- 同じキーが 2 回以上あれば、すべて持つ。値を引く関数は最後のものを返す。
- 値は latin-1 のバイト列を、そのまま 1 文字ずつ文字にしたもの。
- fast / wide の区別は持たない。traceprinter は `TRACE_TRACELOGGER_ARGS` を見ず、イベントの構造で書式を選ぶ(§7.1)。

### 5.3 TraceEvent

```rust
pub struct TraceEvent {
    pub timestamp_cycles: u64,       // TIME だけで補った 64 ビットの時刻(§6.3)
    pub cpu_id: u8,
    pub class_id: ClassId,           // セマンティック ID(§8)
    pub event_id: EventId,
    pub int_class_id: u8,            // header の内部クラス
    pub int_event_id: u16,           // header の内部イベント番号
    pub flags: u8,                   // header のフラグ(ビット 16〜23)
    pub combine_status: CombineStatus,
    pub display_status: DisplayStatus,
    pub payloads: Vec<u32>,          // 組では各スロットの payloads をつないだもの
}

pub enum CombineStatus { Single, Complete, Truncated, Orphan }
pub enum DisplayStatus { Display, Silent }
```

| combine_status | 意味 | payloads |
|---|---|---|
| `Single` | S のスロット 1 つのイベント | 2 語 |
| `Complete` | CB から CE までそろった組 | 2 × スロット数 |
| `Truncated` | CE が来ないまま途切れた組(§6.5) | それまでに集まった分 |
| `Orphan` | 組み立て中の組が無いのに来た CC / CE のスロット 1 つ | 2 語 |

| display_status | 意味 |
|---|---|
| `Display` | traceprinter が何かを出力するもの(イベントの行、ORPHAN 行、UNKNOW 行) |
| `Silent` | traceprinter が何も出力しないもの(header が 0 のスロット、表示しないと決まっているイベントなど。一覧は版の定義が持つ) |

- `flags` は第 1 段階の要件に無かったが、traceprinter の診断行が header をそのまま表示するため加えた。
- 時刻は組では CB(Orphan では そのスロット)の時刻。
- 組の 2 つ目以降のスロットの構造は、Orphan のときだけ必要になるので、DiagInfo が持つ。

### 5.4 DiagInfo

```rust
pub struct DiagInfo {
    pub timestamp_cycles: u64,   // その異常が確定した時点の、TIME だけで補った時刻(§6.3)
    pub cpu_id: u8,
    pub kind: DiagKind,
}
```

| DiagKind | 確定するとき | 持つ情報 |
|---|---|---|
| `Orphan` | CC / CE に組み立て中の組が無かった | TraceEvent の番号、スロットの構造(CC / CE)、traceprinter が表示する番号、表示する位置(§9.3) |
| `Truncated` | 組が途切れた | TraceEvent の番号、途切れた理由(同じ鍵の CB / 窓 / 列の終わり)、traceprinter が表示する番号 |
| `SeqGap` | BUFFER の通番が、同じ CPU の前の BUFFER の通番 + 1 でなかった | 前の通番、今の通番 |
| `CountMismatch` | BUFFER_END で、BUFFER に書かれたスロット数と実際の数が違った | 通番、BUFFER の数、実際の数 |
| `Dropped` | DROPPED があった | 失われたイベントの数 |
| `MissingBufferEnd` | BUFFER_END が来ないまま次の BUFFER が来た、または列が終わった | 通番 |
| `CookieMismatch` | BUFFER_END の payloads が cookie(`0xe6e6e6e6`)でなかった | 通番、2 つの値 |
| `WrapWithoutTime` | TIME が無いのに timestamp_cycles_low32 が一周した(半周以上減った) | 前と今の timestamp_cycles_low32 |
| `Interleaved` | トレースバッファの途中に、別の CPU のスロットがあった | トレースバッファの通番、別の CPU の番号 |
| `OutsideBuffer` | トレースバッファの外(BUFFER の前、BUFFER_END の後)にスロットがあった | 続いた範囲の最初の時点で 1 件。スロット数 |
| `TrailingBytes` | ファイルの末尾に 16 バイトに満たない端数があった | バイト数 |

- 時刻(`timestamp_cycles`)は、確定したスロットの時刻。TrailingBytes は 0 とする。
- 時刻の範囲は持たない。欠けた範囲は、前後の値(SeqGap の前の通番と今の通番など)で分かる。
- payloads は持たない。必要なら TraceEvent の番号から引く。

---

## 6. parse() の処理

```
バイト列
  → kev_header::parse          ヘッダ → KevHeader、版の決定
  → kev_syspage::skip          syspage を読み飛ばす
  → event_parser::parse        (版ごと)
       1. スロットの読み取り          → Vec<Slot>
       2. CPU 別スロット列に分ける
       3. reconstruct_timestamp と CONTROL の点検(CPU ごと)
       4. merge                       → 1 列のスロット
       5. combine                     → Vec<TraceEvent>、DiagInfo
       6. セマンティック ID と display_status を決める
  → KevFile
```

### 6.1 ヘッダ

- `TRACE_<英大文字・数字・_>::` をキーとし、次のキーの直前までを値とする。`TRACE_HEADER_END::` で終わる。
- 誤りとする基準は traceprinter に合わせる(§10)。

### 6.2 スロットの読み取り

```rust
struct Slot {
    timestamp_cycles: u64,          // 3 で決まる
    words: [u32; SLOT_WORDS_NUM],   // SLOT_WORDS_NUM = 4
}
```

- syspage の後ろから 16 バイトずつ読む。バイト順はヘッダ(`TRACE_LITTLE_ENDIAN` / `TRACE_BIG_ENDIAN`)に従う。
- 16 バイトに満たない末尾は読まず、TrailingBytes を記録する。

### 6.3 reconstruct_timestamp と CONTROL の点検

CPU 別スロット列ごとに、ファイルの順に 1 回だけ見る。

**時刻**
- TIME(S のスロット)が来たら、その payloads[0] を上位 32 ビットとする。
- 各スロットの `timestamp_cycles = (上位 32 ビット << 32) | timestamp_cycles_low32`。
- TIME が無いのに timestamp_cycles_low32 が半周以上減ったときは、上位を進めずに WrapWithoutTime を記録する。traceprinter と同じ並びにするため。

**CONTROL の点検**
- BUFFER: 通番の連続を確かめる(SeqGap)。前のトレースバッファが閉じていなければ MissingBufferEnd。
- BUFFER_END: スロット数(直前の TIME から BUFFER_END まで)と cookie を確かめる(CountMismatch、CookieMismatch)。
- DROPPED: Dropped を記録する。
- 列の終わりでトレースバッファが閉じていなければ MissingBufferEnd。
- Interleaved と OutsideBuffer は、ファイルの順で見て判定する(CPU 別にする前の列で調べる)。

### 6.4 merge

- 各 CPU 別スロット列の先頭のうち、`(timestamp_cycles, CPU 番号)` が最小のものを順に取り出す。
- CPU の中では並べ替えない(時刻が戻ってもファイルの順のまま)。
- 取り出した順の 1 から数えた番号を、スロットの通し番号とする(ORPHAN 行の番号)。

### 6.5 combine

merge した順にスロットを処理し、項目(S のイベント・組・orphan)を待ち行列に入れて、先頭から順に TraceEvent にする。

| 規則 | 内容 |
|---|---|
| 組の鍵 | (CPU, 内部クラス, 内部イベント番号, timestamp_cycles_low32)。鍵の違う組は同時に組み立て中になれる |
| S | 項目として待ち行列に入れる。組には影響しない |
| CB | 同じ鍵の組が組み立て中なら、それを Truncated にする。新しい組を始める |
| CC / CE | 同じ鍵の組につなぐ。CE なら閉じる(Complete)。組が無ければ Orphan |
| 窓 | 先頭の項目が組み立て中のまま、後ろに 1022 項目たまったら、その組を Truncated にする。項目は全 CPU の merge の順で数え、S・組・orphan をそれぞれ 1 つと数える |
| 列の終わり | 組み立て中の組を Truncated にする |
| 出力の順 | 項目の先頭スロットの順(組の後ろに来た S は、組の後に出る) |
| header が 0 のスロット | 内部クラス 0・イベント 0 の S として数え、Silent にする |

窓の大きさ 1022 はヘッダに定義が無く、traceprinter の振る舞いから決めた値である。版の定義(def.rs)に置く。

### 6.6 セマンティック ID と display_status

- 版の定義の表で、(内部クラス, 内部イベント番号) → (class_id, event_id) を引く。無ければ UNKNOWN(§8)。
- display_status は版の定義の規則で決める(表示しないイベント、表示に足りない語数など)。

---

## 7. decode()

### 7.1 書式

- 文字列は traceprinter `-n` と同じ書式にする。
  - `class_name`: 表示クラス名(`KER_CALL`、`KER_EXIT`、`THREAD` など)
  - `event_name`: 表示イベント名(`MSG_SENDV/11`、`THRUNNING` など)
  - `args`: 名前より後ろの部分
- fast / wide の書式は、イベントの構造で選ぶ(traceprinter の振る舞い)。
  - S と、CB だけで途切れた組 → fast の書式
  - それ以外の組 → wide の書式
- 引数の名前は QNX の名前(traceprinter の表示)のまま使う。
- 解釈できないイベント(UNKNOWN)は、payloads を 16 進で並べる。

### 7.2 欠けた payloads の表し方(`DecodeOptions::missing`)

| 値 | 内容 |
|---|---|
| `Traceprinter`(既定) | traceprinter と同じ(欠けた引数は出さない、など) |
| `Marked` | 欠けた引数を `(truncated)` / `(missing)` と書く |

- Truncated の組で、来なかったスロットの分の引数は `(truncated)` と書く。
- 書式が求める語より payloads が少ないときは `(missing)` と書く。
- CLI は `Traceprinter` だけを使う。`Marked` は API からだけ使う。

---

## 8. セマンティック ID

```rust
pub struct ClassId(pub u16);
pub struct EventId(pub u16);
pub const UNKNOWN: u16 = 0xFFFF;
```

- ids.rs に定数として定義する。版に依存しない。
- 一度決めた番号は変えない。追加だけする。対応表は手で保守する。

### クラス

| class_id | 名前 | 内容 |
|---|---|---|
| 1 | CONTROL | TIME・BUFFER・BUFFER_END・DROPPED |
| 2 | KER_CALL_ENTER | カーネルコールの呼び出し |
| 3 | KER_CALL_EXIT | カーネルコールの戻り |
| 4 | KER_CALL_INT | カーネルコールの割り込まれ |
| 5 | INT | 割り込み |
| 6 | THREAD | スレッドの状態の遷移・生成・破棄 |
| 7 | PROCESS | プロセスの生成・破棄・名前 |
| 8 | SYSTEM | システムのイベント |
| 9 | USER | ユーザーのイベント |
| 10 | COMM | メッセージ・パルス |
| 11 | QUIP | QUIP |
| 12 | SEC | セキュリティ |
| 13 | QVM | 仮想マシン |
| 0xFFFF | UNKNOWN | 対応表に無い内部クラス(クラス 0 と未定義の番号) |

番号は確定ではない。実装時に表を確定して ids.rs に書く。

### イベント

- クラスごとに、意味(QNX の名前)に番号を割り当てる。
  - 例: KER_CALL_ENTER の MSG_SENDV。カーネルコールの番号とは別の番号にする。
- 64 ビット版(KER_CALL・SYSTEM の `_64`)は別の event_id にする。
- 番号が任意のイベント(USER・QUIP・INT の割り込み番号など)は、内部イベント番号をそのまま event_id にする。
- 対応表に無いものは UNKNOWN(`0xFFFF`)。

### 表示文字列

- `class_name()` / `event_name()` は、traceprinter が表示するクラス名・イベント名を返す。
- 表示名は版で変わりうる(カーネルコールの番号など)ので、版を引数にとる。

---

## 9. traceprinter 互換出力

`traceprinter -n -f X.kev -o OUT` が OUT に書く内容と、バイト単位で同じにする。標準出力・標準エラーに出るもの(末尾の集計 `TRACEPARSER: … cnt` など)は再現しない。

### 9.1 全体

```
TRACEPRINTER version 1.02
TRACEPARSER LIBRARY version 1.02
 -- HEADER FILE INFORMATION -- 
<ヘッダの行>
 -- KERNEL EVENTS -- 
<イベントの行・診断の行>
```

- 改行は CRLF。
- ヘッダの行: keywords を kev の順に `{キー:>22}:: {値}` の形で書く。`TRACE_HEADER_BEGIN` と `TRACE_HEADER_END` は書かない。

### 9.2 イベントの行

```
t:0x{timestamp_cycles_low32:08x} CPU:{cpu:02} {class_name:<16}:{event_name:<19}{args}
```

- 桁や空白の細部、イベントの後の空行(BUFFER などの後)は、版の定義(args.rs)が持つ。
- Silent のイベントは何も書かない。

### 9.3 診断の行

| 行 | 位置 | 内容 |
|---|---|---|
| `TRUNC` | 途切れた組のイベントの行の直前 | イベントの通し番号、先頭スロットの 4 語 |
| `ORPHAN` | 最大 1022 項目前(待ち行列の先頭にあった項目の直前) | スロットの通し番号、構造(CC / CE)、スロットの 4 語 |
| `UNKNOW` | 定義の無いクラスのイベントの行の直前(途切れた組なら TRUNC の後)。クラス 0・イベント 0 には出さない | header など |

- 行の書式は版の定義が持つ。
- 番号と位置は parse() が DiagInfo に入れる。traceprinter.rs は、TraceEvent と DiagInfo から行を作る。

---

## 10. エラー

| 状況 | 扱い |
|---|---|
| `TRACE_HEADER_END::` が無い | エラー |
| `TRACE_VER_MAJOR` / `TRACE_VER_MINOR` が無い | エラー(traceprinter が読まない) |
| ヘッダの項目(キー::値)が 2048 バイト以上 | エラー(traceprinter が読まない) |
| `TRACE_SYSPAGE_LEN` が無い・空 | 0 とする |
| `TRACE_SYSPAGE_LEN` の分のバイトが無い | エラー(traceprinter が読まない) |
| `TRACE_SYS_RELEASE` が対応していない版 | エラー |
| イベント部の異常(トレースバッファの不整合、組の異常など) | エラーにしない。DiagInfo に記録して続ける |

traceprinter が読まない入力の細部(ヘッダ全体の長さの上限が無いこと、など)は、実装の前に traceprinter で確かめて表を足す。

---

## 11. 定義の作り方

- 版ごとの def.rs・args.rs は、QNX の公開資料から作る。
  - QNX SDP のヘッダ(`sys/trace.h`、`sys/kercalls.h` など)
  - イベントごとの引数の一覧(TableOfEvents)
  - traceprinter の出力(表示名・書式の確認)
- 作業は AI が行い、Python のスクリプトを補助に使う。生成した `.rs` はリポジトリに入れる。
- QNX の配布物(ヘッダ、TableOfEvents)はリポジトリに入れない。
- 手順は docs/qnx-version-guide.md に書く。版に固有の情報は入れず、何を・どの資料から・どの形で作るかを書く。

---

## 12. 依存とロギング

- ライブラリ(tracehub-kev)は標準ライブラリだけを使う。
- CLI も標準ライブラリだけを使う(引数は自分で解析する)。
- ライブラリは何も出力しない。気づいたことは DiagInfo で返す。ロギング(`log` クレート)は必要になってから加える。

---

## 13. CLI

```
tracehub traceprinter <kev ファイル> <出力ファイル>
```

- traceprinter 互換出力(`-n` 相当)を出力ファイルに書く。
- エラーのときは、理由を標準エラーに書いて終了コード 1 で終わる。

---

## 14. 試験

| 種類 | 内容 |
|---|---|
| 合成 kev | tools/kevedit/kevcsv.py で CSV から作る。版の定義にある全イベント(S・組・途切れ・orphan・未定義)を網羅する。traceprinter の出力とバイト単位で比べる |
| 実機の kev | 実機でカーネルコールなどを呼ぶプログラムを動かし、fast と wide の 2 種類を取る。traceprinter の出力とバイト単位で比べる。リポジトリには入れず、置き場所は環境変数で指定する |
| 単体試験 | ヘッダ・merge・combine・CONTROL の点検・decode を、小さな入力で確かめる |

- 実機で取りにくいイベントは、合成 kev で確かめる。
- traceprinter と比べる試験は、手元に traceprinter があるときだけ行う(場所は環境変数で指定)。
- traceprinter の出力は、試験の期待値としてリポジトリに入れる(合成 kev の分だけ)。
