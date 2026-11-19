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

mod kev_header;
mod kev_syspage;
pub mod model;
pub mod semantic_id;
pub mod tp_format;

pub use model::*;
pub use semantic_id::{ClassId, EventId};

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
        QnxVersion::V800 => qnx800::event_decoder::class_display(class_id),
    }
}

/// セマンティックなイベント ID の表示文字列(traceprinter が表示するイベント名)
pub fn event_name(version: QnxVersion, class_id: ClassId, event_id: EventId) -> Option<String> {
    match version {
        QnxVersion::V800 => {
            let (c, e) = qnx800::event_parser::internal_ids(class_id, event_id)?;
            qnx800::event_decoder::names(c, e).1.filter(|n| !n.is_empty())
        }
    }
}

/// QNX 8.0 に依存する部分(src/qnx800/)
pub mod qnx800 {
    /// 内部 ID の定義(生成ファイル。tests/kev/tools/gen_qnx_def.py の出力のまま)
    #[rustfmt::skip]
    pub mod def;
    /// traceprinter の表示の定義(手で保守する)
    pub mod args;
    /// イベントの表示文字列(SLOG2 を含む)
    pub(crate) mod event_decoder;
    /// スロットの読み取りからイベントの列まで。内部 ID ⇔ セマンティック ID の変換
    pub(crate) mod event_parser;
    /// 試験・確認用のコード(ライブラリの機能ではない)
    #[doc(hidden)]
    pub mod test_support;
    /// traceprinter 互換出力
    pub(crate) mod traceprinter;
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

/// docs/semantic-id.md の一覧(生成部分)が、semantic_id.rs と版ごとの定義に合っているかの試験。
/// 合っていなければ失敗する。`TRACEHUB_UPDATE_DOCS=1 cargo test` で書き直す。版を足すときは VERSIONS・class_define・event_define に足す。
#[cfg(test)]
mod semantic_id_doc {
    use super::*;

    const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/semantic-id.md");
    const BEGIN: &str =
        "<!-- 生成部分ここから(cargo test が確かめる。書き直すときは TRACEHUB_UPDATE_DOCS=1 cargo test) -->\n";
    const END: &str = "<!-- 生成部分ここまで -->";
    const VERSIONS: &[(QnxVersion, &str)] = &[(QnxVersion::V800, "8.0")];

    /// その版のヘッダの定義名(内部クラス・イベント)
    fn class_define(v: QnxVersion, c: ClassId) -> Option<String> {
        match v {
            QnxVersion::V800 => qnx800::event_parser::class_define_name(c).map(String::from),
        }
    }
    fn event_define(v: QnxVersion, c: ClassId, e: EventId) -> Option<String> {
        match v {
            QnxVersion::V800 => qnx800::event_parser::event_define_name(c, e),
        }
    }

    /// semantic_id.rs のイベントのモジュール: (モジュール名, 説明, [(定数名, 番号)])
    type Module = (String, String, Vec<(String, u16)>);

    fn modules() -> Vec<Module> {
        let mut out: Vec<Module> = Vec::new();
        let mut doc = String::new();
        for line in include_str!("semantic_id.rs").lines() {
            let t = line.trim();
            if let Some(d) = t.strip_prefix("/// ") {
                doc = d.to_string();
            } else if let Some(m) = t.strip_prefix("pub mod ").and_then(|s| s.strip_suffix(" {")) {
                out.push((m.to_string(), std::mem::take(&mut doc), Vec::new()));
            } else if let (Some(m), Some((name, n))) = (
                out.last_mut(),
                t.strip_prefix("pub const ")
                    .and_then(|r| r.split_once(": EventId = EventId(")),
            ) {
                m.2.push((name.to_string(), n.trim_end_matches(");").parse().unwrap()));
            }
        }
        out
    }

    fn classes_of(m: &str) -> &'static [ClassId] {
        match m {
            "control" => &[ClassId::CONTROL],
            "kercall" => &[ClassId::KER_CALL_ENTER, ClassId::KER_CALL_EXIT, ClassId::KER_CALL_INT],
            "int" => &[ClassId::INT],
            "thread" => &[ClassId::THREAD],
            "process" => &[ClassId::PROCESS],
            "system" => &[ClassId::SYSTEM],
            "comm" => &[ClassId::COMM],
            "sec" => &[ClassId::SEC],
            "qvm" => &[ClassId::QVM],
            _ => panic!("semantic_id.rs の {m} のクラスを classes_of に足す"),
        }
    }

    fn generate() -> String {
        use std::fmt::Write as _;
        let dash = |s: Option<String>| s.filter(|s| !s.is_empty()).unwrap_or_else(|| "—".into());
        // 表の中の | はエスケープする
        let code = |s: Option<String>| {
            s.map(|x| format!("`{}`", x.replace('|', r"\|")))
                .unwrap_or_else(|| "—".into())
        };
        let mut s = String::new();
        let vh: String = VERSIONS
            .iter()
            .map(|(_, n)| format!(" QNX {n}: 定義名 | QNX {n}: 表示クラス名 |"))
            .collect();
        let vs: String = VERSIONS.iter().map(|_| "---|---|").collect();
        writeln!(s, "### クラス\n\n| class_id | 定数 |{vh}\n|---|---|{vs}").unwrap();
        for &(name, c) in ClassId::ALL {
            let cols: String = VERSIONS
                .iter()
                .map(|&(v, _)| {
                    format!(
                        " {} | {} |",
                        code(class_define(v, c)),
                        dash(class_name(v, c).map(String::from))
                    )
                })
                .collect();
            writeln!(s, "| {} | `{name}` |{cols}", c.0).unwrap();
        }
        let none: String = VERSIONS.iter().map(|_| " — | — |").collect();
        writeln!(s, "| {} | `UNKNOWN` |{none}", ClassId::UNKNOWN.0).unwrap();
        let mut total = 0;
        for (m, doc, consts) in modules() {
            let classes = classes_of(&m);
            total += consts.len() * classes.len();
            let cls: Vec<String> = classes.iter().map(|c| format!("`{}`", c.const_name())).collect();
            writeln!(
                s,
                "\n### `{m}`: {doc}\n\nクラス: {}。{} 個。\n",
                cls.join("・"),
                consts.len()
            )
            .unwrap();
            let mut head = "| event_id | 定数 |".to_string();
            let mut sep = "|---|---|".to_string();
            for &(v, n) in VERSIONS {
                head += &format!(" QNX {n}: 定義名 |");
                sep += "---|";
                for c in classes {
                    let d = if classes.len() == 1 {
                        "表示名".to_string()
                    } else {
                        format!("表示名({})", class_name(v, *c).unwrap_or("?"))
                    };
                    head += &format!(" QNX {n}: {d} |");
                    sep += "---|";
                }
            }
            writeln!(s, "{head}\n{sep}").unwrap();
            for (name, n) in consts {
                let e = EventId(n);
                let mut row = format!("| {n} | `{m}::{name}` |");
                for &(v, _) in VERSIONS {
                    row += &format!(" {} |", code(event_define(v, classes[0], e)));
                    for &c in classes {
                        row += &format!(" {} |", dash(event_name(v, c, e)));
                    }
                }
                writeln!(s, "{row}").unwrap();
            }
        }
        writeln!(
            s,
            "\nイベントの定数 {} 個、クラスとの組み合わせ {total} 個(USER・QUIP の任意番号を除く)。",
            modules().iter().map(|m| m.2.len()).sum::<usize>()
        )
        .unwrap();
        s
    }

    #[test]
    fn semantic_id_doc_is_up_to_date() {
        let doc = std::fs::read_to_string(DOC).expect("docs/semantic-id.md");
        let i = doc.find(BEGIN).expect("生成部分の始まりの印") + BEGIN.len();
        let j = i + doc[i..].find(END).expect("生成部分の終わりの印");
        let new = format!("{}{}{}", &doc[..i], generate(), &doc[j..]);
        if new == doc {
            return;
        }
        if std::env::var("TRACEHUB_UPDATE_DOCS").is_ok_and(|v| v == "1") {
            std::fs::write(DOC, new).unwrap();
        } else {
            panic!("docs/semantic-id.md の一覧が古い。TRACEHUB_UPDATE_DOCS=1 cargo test で書き直す");
        }
    }
}
