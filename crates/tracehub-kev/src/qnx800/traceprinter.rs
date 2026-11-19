//! traceprinter 互換出力(`traceprinter -n -f X.kev -o OUT` が OUT に書く内容と同じ。設計書 §9)。
//!
//! イベントの行は event_decoder の文字列、診断の行(TRUNC・ORPHAN・UNKNOW)は TraceEvent の生の words と
//! DiagInfo の番号・位置から作る。

use std::collections::HashMap;
use std::io::{self, Write};

use super::{HeaderBits, args, def, event_decoder, header_bits, make_header};
use crate::model::{CombineStatus, DiagKind, KevFile, OrphanStructure, TraceEvent};
use crate::tp_format::Missing;

pub(crate) fn write(kev: &KevFile, out: &mut impl Write) -> io::Result<()> {
    let le = args::LINE_END.as_bytes();
    for b in args::BANNER {
        out.write_all(b.as_bytes())?;
        out.write_all(le)?;
    }
    out.write_all(args::HEADER_TITLE.as_bytes())?;
    out.write_all(le)?;
    for (k, v) in &kev.header.keywords {
        if args::HEADER_SKIP.contains(&k.as_str()) {
            continue;
        }
        let line = format!("{k:>w$}:: {v}", w = args::HEADER_KEY_WIDTH);
        out.write_all(&latin1(&line))?;
        out.write_all(le)?;
    }
    out.write_all(args::EVENTS_TITLE.as_bytes())?;
    out.write_all(le)?;

    // 診断の情報を events の番号で引けるようにする
    let mut trunc: HashMap<usize, u64> = HashMap::new();
    let mut orphans: Vec<(usize, usize, u64, u32)> = Vec::new(); // (表示位置, events の番号, スロットの番号, 構造)
    for d in &kev.diag {
        match d.kind {
            DiagKind::Truncated {
                event_index,
                event_number,
                ..
            } => {
                trunc.insert(event_index, event_number);
            }
            DiagKind::Orphan {
                event_index,
                slot_number,
                structure,
                display_before,
            } => {
                let st = match structure {
                    OrphanStructure::CC => def::STRUCT_CC,
                    OrphanStructure::CE => def::STRUCT_CE,
                };
                orphans.push((display_before, event_index, slot_number, st))
            }
            _ => {}
        }
    }
    orphans.sort_unstable();
    let mut next_orphan = 0;
    let mut write_orphans = |out: &mut dyn Write, upto: usize| -> io::Result<()> {
        while let Some(&(at, idx, n, st)) = orphans.get(next_orphan)
            && at <= upto
        {
            let ev = &kev.events[idx];
            out.write_all(diag_line(args::DIAG_ORPHAN, n, ev, &first_slot(ev, st)).as_bytes())?;
            out.write_all(le)?;
            next_orphan += 1;
        }
        Ok(())
    };

    for (i, ev) in kev.events.iter().enumerate() {
        write_orphans(out, i)?;
        if ev.combine_status == CombineStatus::Orphan {
            continue;
        }
        let raw = first_slot(ev, event_structure(ev));
        if let Some(&n) = trunc.get(&i) {
            out.write_all(diag_line(args::DIAG_TRUNC, n, ev, &raw).as_bytes())?;
            out.write_all(le)?;
        }
        if super::event_parser::unknown_line(ev.int_class_id, ev.int_event_id) {
            out.write_all(diag_line(args::DIAG_UNKNOWN, 0, ev, &raw).as_bytes())?;
            out.write_all(le)?;
        }
        let words = &ev.payloads;
        if let Some(l) = event_decoder::format(ev.int_class_id, ev.int_event_id, words, Missing::Omit) {
            let line = format!(
                "t:0x{:08x} CPU:{:02} {:<cw$}:{:<nw$}{}",
                ev.timestamp_cycles as u32,
                ev.cpu_id,
                l.class,
                l.name,
                l.args,
                cw = args::CLASS_WIDTH,
                nw = args::NAME_WIDTH
            );
            out.write_all(&latin1(&line))?;
            out.write_all(le)?;
        }
        if event_decoder::blank_after(ev.int_class_id, ev.int_event_id) {
            out.write_all(le)?;
        }
    }
    write_orphans(out, kev.events.len())?;
    out.flush()
}

/// イベントの先頭スロットの 4 words を TraceEvent から組み直す
fn first_slot(ev: &TraceEvent, structure: u32) -> [u32; 4] {
    let h = make_header(HeaderBits {
        structure,
        cpu: ev.cpu_id,
        flags: ev.flags,
        class: ev.int_class_id,
        event: ev.int_event_id,
    });
    let p = |i: usize| ev.payloads.get(i).copied().unwrap_or(0);
    [h, ev.timestamp_cycles as u32, p(0), p(1)]
}

/// orphan でないイベントの先頭スロットの構造(組は CB、それ以外は S)
fn event_structure(ev: &TraceEvent) -> u32 {
    if ev.combine_status == CombineStatus::Truncated || ev.payloads.len() > 2 {
        def::STRUCT_CB
    } else {
        def::STRUCT_S
    }
}

/// 診断の行を作る
fn diag_line(tpl: &str, n: u64, ev: &TraceEvent, raw: &[u32; 4]) -> String {
    let class = ev.int_class_id;
    let code = ev.int_event_id;
    let class_name = match args::DIAG_CLASS_NAMES.iter().find(|r| r.0 == class) {
        Some(r) => r.1.to_string(),
        None => args::DIAG_UNKNOWN_CLASS.replace("{num}", &class.to_string()),
    };
    let (ktype, func) = if class == def::CLASS_KER_CALL {
        let shift = def::KERCALL_EXIT.trailing_zeros();
        let t = (code & !def::KERCALL_NUM_MASK) >> shift;
        let name = args::DIAG_KERCALL_TYPES
            .iter()
            .find(|r| r.0 == t)
            .map_or(args::DIAG_KERCALL_TYPE_UNKNOWN, |r| r.1);
        (
            args::DIAG_KERCALL_TYPE.replace("{type}", name),
            code & def::KERCALL_NUM_MASK,
        )
    } else {
        (String::new(), code)
    };
    let structure = header_bits(raw[0]).structure;
    let kind = args::DIAG_STRUCT_NAMES
        .iter()
        .find(|r| r.0 == structure)
        .map_or("?", |r| r.1);
    let hex = |w: &[u32]| w.iter().map(|x| format!("{x:08x}")).collect::<Vec<_>>().join(" ");
    tpl.replace("{n}", &n.to_string())
        .replace("{kind}", kind)
        .replace("{cpu}", &format!("{:02}", ev.cpu_id))
        .replace("{class}", &class_name)
        .replace("{ktype}", &ktype)
        .replace("{func}", &func.to_string())
        .replace("{raw2}", &hex(&raw[..2]))
        .replace("{raw}", &hex(raw))
}

/// 1 文字 1 バイト(latin-1)のバイト列にする。行の中の改行(SLOG2 の文字列など)は、traceprinter が
/// テキストモードで書くのと同じく LINE_END にする
fn latin1(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        if c == '\n' {
            out.extend_from_slice(args::LINE_END.as_bytes());
        } else {
            out.push(c as u32 as u8);
        }
    }
    out
}
