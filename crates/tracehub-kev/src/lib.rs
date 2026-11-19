//! QNX のカーネルトレース(kev ファイル)の読み取り。
//!
//! - [`parse`]: kev のバイト列 → [`KevFile`](ヘッダ・イベントの列・異常)
//! - [`decode`]: イベント 1 件の表示文字列(traceprinter -n と同じ書式)
//! - [`class_name`] / [`event_name`]: セマンティック ID の表示文字列
//! - [`traceprinter::write`]: traceprinter 互換出力
//!
//! kev の解釈は公式に公開されていないので、traceprinter の出力を正として決めている。
//! ヘッダの値は印字可能な ASCII。イベントの文字列(args など)は payloads のバイトを 1 バイト 1 文字にしたもの
//! (traceprinter はバイトをそのまま出力するので、文字コードは決まっていない)。

pub mod ids;
mod kev_header;
mod kev_syspage;
pub mod model;
pub mod qnx800;
pub mod tp_format;

pub use ids::{ClassId, EventId};
pub use model::*;

use tp_format::Missing;

/// kev を読む。ヘッダの誤り(traceprinter が読まないもの)と、対応していない版はエラー
pub fn parse(bytes: &[u8]) -> Result<KevFile, KevError> {
    let (header, end) = kev_header::parse(bytes)?;
    let start = kev_syspage::skip(bytes, end, header.syspage_len)?;
    let (events, diag) = match header.version {
        QnxVersion::V800 => qnx800::event_parser::parse(&bytes[start..]),
    };
    Ok(KevFile { header, events, diag })
}

/// イベント 1 件の表示文字列(traceprinter -n と同じ書式)。
///
/// traceprinter が行を出さないイベント(Silent・Unknown・Orphan)は、
/// 名前が決まればその名前、決まらなければ `UNKNOWN(内部クラス)` と内部イベント番号を返し、args は payloads の 16 進の並び。
pub fn decode(version: QnxVersion, event: &TraceEvent, opts: &DecodeOptions) -> DecodedEvent {
    let missing = match opts.missing {
        MissingStyle::Traceprinter => Missing::Omit,
        MissingStyle::Marked if event.combine_status == CombineStatus::Truncated => Missing::Mark("(truncated)"),
        MissingStyle::Marked => Missing::Mark("(missing)"),
    };
    let words = &event.payloads;
    let (class, code) = (event.int_class_id, event.int_event_id);
    match version {
        QnxVersion::V800 => {
            if event.combine_status != CombineStatus::Orphan
                && let Some(l) = qnx800::event_decoder::format(class, code, words, missing)
            {
                return DecodedEvent {
                    class_name: l.class,
                    event_name: l.name,
                    args: l.args,
                };
            }
            let (c, n) = qnx800::event_decoder::names(class, code);
            let mut args = String::new();
            tp_format::hex_words(words, &mut args);
            DecodedEvent {
                class_name: c.unwrap_or_else(|| format!("UNKNOWN({class})")),
                event_name: n.filter(|n| !n.is_empty()).unwrap_or_else(|| code.to_string()),
                args,
            }
        }
    }
}

/// セマンティックなクラス ID の表示文字列(traceprinter が表示するクラス名)
pub fn class_name(version: QnxVersion, class_id: ClassId) -> Option<&'static str> {
    match version {
        QnxVersion::V800 => qnx800::class_display(class_id),
    }
}

/// セマンティックなイベント ID の表示文字列(traceprinter が表示するイベント名)
pub fn event_name(version: QnxVersion, class_id: ClassId, event_id: EventId) -> Option<String> {
    match version {
        QnxVersion::V800 => {
            let (c, e) = qnx800::internal_ids(class_id, event_id)?;
            qnx800::event_decoder::names(c, e).1.filter(|n| !n.is_empty())
        }
    }
}

/// traceprinter 互換出力
pub mod traceprinter {
    use crate::{KevFile, QnxVersion, qnx800};

    /// `traceprinter -n -f X.kev -o OUT` が OUT に書く内容と同じものを書く
    pub fn write(kev: &KevFile, out: &mut impl std::io::Write) -> std::io::Result<()> {
        match kev.header.version {
            QnxVersion::V800 => qnx800::traceprinter::write(kev, out),
        }
    }
}
