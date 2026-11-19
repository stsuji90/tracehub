//! 確認用のイベント一覧(セマンティック ID ごとの定義・表示・合成データでの出力の例)。
//!
//! tests/kev/tools/event_catalog.py が examples/event_catalog.rs を通して使い、TableOfEvents・
//! traceprinter との比較結果・実機の kev と並べた一覧を作る。

use std::collections::BTreeMap;

use super::{args, def, event_decoder, internal_ids, scenarios};
use crate::model::{CombineStatus, DisplayStatus, KevFile};
use crate::semantic_id::{ClassId, EventId};
use crate::tp_format::{Ev, Item, Op, Spec};

/// 1 つのセマンティック ID の一覧の行
#[derive(Debug, Clone)]
pub struct Entry {
    pub class_id: ClassId,
    pub event_id: EventId,
    /// クラス ID の定数名(例: "KER_CALL_ENTER")
    pub class: &'static str,
    /// イベント ID の定数名(例: "kercall::MSG_SENDV_64")
    pub event: String,
    pub int_class: u8,
    pub int_event: u16,
    /// traceprinter の表示クラス名・表示イベント名
    pub display_class: String,
    pub display_name: String,
    /// args.rs の書式の説明(fast / wide)
    pub fast: String,
    pub wide: String,
    /// label 付きの要素のラベル(fast / wide)
    pub fast_labels: Vec<String>,
    pub wide_labels: Vec<String>,
    /// 合成データ(全イベント網羅の catalog)での件数と、traceprinter が行を出す件数
    pub synthetic: usize,
    pub synthetic_displayed: usize,
    /// 合成データでの出力の例(S の 2 語 / 組)
    pub example_fast: String,
    pub example_wide: String,
}

/// クラス ID の定数名
pub fn class_name(c: ClassId) -> &'static str {
    match c {
        ClassId::CONTROL => "CONTROL",
        ClassId::KER_CALL_ENTER => "KER_CALL_ENTER",
        ClassId::KER_CALL_EXIT => "KER_CALL_EXIT",
        ClassId::KER_CALL_INT => "KER_CALL_INT",
        ClassId::INT => "INT",
        ClassId::THREAD => "THREAD",
        ClassId::PROCESS => "PROCESS",
        ClassId::SYSTEM => "SYSTEM",
        ClassId::USER => "USER",
        ClassId::COMM => "COMM",
        ClassId::QUIP => "QUIP",
        ClassId::SEC => "SEC",
        ClassId::QVM => "QVM",
        _ => "UNKNOWN",
    }
}

/// def.rs の表から、セマンティック ID の全組み合わせと定数名(USER・QUIP の任意番号を除く)
pub fn all_ids() -> Vec<(ClassId, EventId, String)> {
    let mut out = Vec::new();
    let mut add = |c: ClassId, module: &str, suffix: &str, t: &[(u16, &str, EventId)]| {
        out.extend(t.iter().map(|r| (c, r.2, format!("{module}::{}{suffix}", r.1))));
    };
    add(ClassId::CONTROL, "control", "", def::CONTROL_EVENTS);
    for c in [ClassId::KER_CALL_ENTER, ClassId::KER_CALL_EXIT, ClassId::KER_CALL_INT] {
        add(c, "kercall", "", def::KERCALLS);
        add(c, "kercall", "_64", def::KERCALLS_64);
    }
    add(ClassId::INT, "int", "", def::INT_EVENTS);
    add(ClassId::THREAD, "thread", "", def::THREAD_STATES);
    add(ClassId::THREAD, "thread", "", def::PR_TH_VTHREAD);
    add(ClassId::PROCESS, "process", "", def::PR_TH_PROCESS);
    add(ClassId::SYSTEM, "system", "", def::SYSTEM_EVENTS);
    add(ClassId::SYSTEM, "system", "_64", def::SYSTEM_EVENTS_64);
    add(ClassId::COMM, "comm", "", def::COMM_EVENTS);
    add(ClassId::SEC, "sec", "", def::SEC_EVENTS);
    add(ClassId::QVM, "qvm", "", def::QVM_EVENTS);
    out
}

/// custom の要素のラベル: 語をそろえた payloads で書かせ、`label:` と `label =` を拾う
fn custom_labels(it: &Item) -> Vec<String> {
    let w: Vec<u32> = (0..16).map(|i| 0xF0F0_F080 + i).collect();
    let mut out = String::new();
    crate::tp_format::render(std::slice::from_ref(it), &w, crate::tp_format::Missing::Omit, &mut out);
    let toks: Vec<&str> = out.split_whitespace().collect();
    let ident = |t: &str| t.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let mut labels = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if let Some((l, _)) = t.split_once(':')
            && ident(l)
        {
            labels.push(l.to_string());
        } else if toks.get(i + 1) == Some(&"=") && ident(t) {
            labels.push(t.to_string());
        }
    }
    labels
}

fn describe(s: Spec) -> (String, Vec<String>) {
    let items = match s {
        Spec::None => return ("(なし)".into(), Vec::new()),
        Spec::Silent => return ("(出力しない)".into(), Vec::new()),
        Spec::Items(i) => i,
    };
    let mut labels = Vec::new();
    let parts: Vec<String> = items
        .iter()
        .map(|it| match *it {
            Item::Arg { op, label, word } => {
                labels.push(label.to_string());
                let w = word as usize;
                match op {
                    Op::H => format!("{label}=hex(w{w})"),
                    Op::D => format!("{label}=dec(w{w})"),
                    Op::U => format!("{label}=udec(w{w})"),
                    Op::Q => format!("{label}=hex64(w{w}-{})", w + 1),
                    Op::E => format!("{label}=dec64(w{w}-{})", w + 1),
                    Op::S(n) => format!("{label}=msg(w{w}, {n} 語)"),
                    Op::SZ => format!("{label}=str(w{w})"),
                }
            }
            Item::Flags { word, .. } => format!("flags(w{word})"),
            Item::Dump => "dump".into(),
            Item::Custom(_) => {
                labels.extend(custom_labels(it));
                "custom".into()
            }
        })
        .collect();
    (
        if parts.is_empty() {
            "(引数なし)".into()
        } else {
            parts.join(" ")
        },
        labels,
    )
}

/// 全セマンティック ID の一覧(合成データの catalog の出力の例つき)
pub fn entries() -> Vec<Entry> {
    // 合成データでの件数と出力の例
    let csv = scenarios::scenarios().into_iter().find(|s| s.0 == "catalog").unwrap().1;
    let kev = crate::parse(&scenarios::kev_bytes(&csv)).expect("catalog");
    let mut seen: BTreeMap<(ClassId, EventId), (usize, usize, String, String)> = BTreeMap::new();
    for ev in &kev.events {
        if ev.combine_status != CombineStatus::Complete {
            continue;
        }
        let e = seen.entry((ev.class_id, ev.event_id)).or_default();
        e.0 += 1;
        if let Some(l) = event_decoder::format(
            ev.int_class_id,
            ev.int_event_id,
            &ev.payloads,
            crate::tp_format::Missing::Omit,
        ) {
            e.1 += 1;
            let line = format!("{}:{}{}", l.class, l.name, l.args);
            let slot = if ev.payloads.len() <= 2 { &mut e.2 } else { &mut e.3 };
            if slot.is_empty() {
                *slot = line;
            }
        }
    }
    let rows: BTreeMap<(ClassId, EventId), &Ev> = args::EVENTS
        .iter()
        .filter_map(|e| Some(((e.class, e.event?), e)))
        .collect();
    all_ids()
        .into_iter()
        .map(|(c, e, name)| {
            let (ic, code) = internal_ids(c, e).expect("def.rs");
            let (dc, dn) = event_decoder::names(ic, code);
            let row = rows.get(&(c, e)).copied();
            // fast の書式が無ければ、FAST_FALLBACK_WIDE により wide の書式か出力しない
            let fallback = args::FAST_FALLBACK_WIDE
                .iter()
                .find(|r| r.0 == c)
                .map_or(args::FAST_FALLBACK_WIDE_DEFAULT, |r| r.1);
            let (fast, fast_labels) = row.map_or(("(行なし)".into(), Vec::new()), |r| match r.fast {
                Spec::None if fallback && !matches!(r.wide, Spec::None) => {
                    let (d, l) = describe(r.wide);
                    (format!("(wide と同じ) {d}"), l)
                }
                f => describe(f),
            });
            let (wide, wide_labels) = row.map_or(("(行なし)".into(), Vec::new()), |r| describe(r.wide));
            let mut fast = fast;
            if let Some(r) = row {
                if r.intcall {
                    fast = "INT_CALL の形(引数なし)".into();
                }
                if r.min_words > 0 {
                    fast = format!("{fast} [{} 語以上]", r.min_words);
                }
            }
            let s = seen.remove(&(c, e)).unwrap_or_default();
            Entry {
                class_id: c,
                event_id: e,
                class: class_name(c),
                event: name,
                int_class: ic,
                int_event: code,
                display_class: dc.unwrap_or_default(),
                display_name: dn.unwrap_or_default(),
                fast,
                wide,
                fast_labels,
                wide_labels,
                synthetic: s.0,
                synthetic_displayed: s.1,
                example_fast: s.2,
                example_wide: s.3,
            }
        })
        .collect()
}

/// kev の中のセマンティック ID ごとの件数と、traceprinter が行を出す件数
pub fn count(kev: &KevFile) -> BTreeMap<(ClassId, EventId), (usize, usize)> {
    let mut out: BTreeMap<(ClassId, EventId), (usize, usize)> = BTreeMap::new();
    for ev in &kev.events {
        let e = out.entry((ev.class_id, ev.event_id)).or_default();
        e.0 += 1;
        e.1 += (ev.display_status == DisplayStatus::Display) as usize;
    }
    out
}
