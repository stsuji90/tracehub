//! 試験用の合成データ(kevcsv の CSV)を作る。traceprinter との比較に使う。
//!
//! | 場面 | 内容 |
//! |---|---|
//! | catalog | args.rs と def.rs の全イベント(定義の無い番号・クラスを含む)。payloads の語数・文字列・値の名前を変えた変種 |
//! | short_words | 語が足りないイベント(S・CB だけ・CB CC で途切れた組) |
//! | combine_window | 組み立て中の組の後ろにたまる項目の数の境目(窓) |
//! | orphan_lookahead | 多数のイベントの後の orphan(ORPHAN 行の位置) |
//! | slog2 | SLOG2 のレコードの規則と printf の変換 |
//! | sched_wrap | 4 CPU・下位 32 ビットの一周(TIME あり)・CPU 間の同時刻 |
//! | sched_wrap_inferred | 同じ列で、一周のときの TIME が無い |

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use super::{args, def, internal_class, internal_ids};
use crate::semantic_id::{ClassId, EventId, kercall, process, system, thread};
use crate::tp_format::{Spec, Usage};

/// 試験用の場面: (名前, CSV)
pub fn scenarios() -> Vec<(&'static str, String)> {
    let mut out = vec![
        ("catalog", to_csv(1, &pack(&catalog_events(), MAX_BUFFER_SLOTS, true))),
        ("short_words", to_csv(1, &short_words())),
        ("combine_window", to_csv(2, &combine_window())),
        ("orphan_lookahead", to_csv(1, &orphan_lookahead())),
        ("slog2", to_csv(1, &pack(&slog2_events(), MAX_BUFFER_SLOTS, true))),
    ];
    let sched = sched_events(4, 3000);
    out.push(("sched_wrap", to_csv(4, &pack(&sched, MAX_BUFFER_SLOTS, true))));
    out.push(("sched_wrap_inferred", to_csv(4, &pack(&sched, MAX_BUFFER_SLOTS, false))));
    out
}

/// 1 トレースバッファのスロット数の上限(TIME・BUFFER・BUFFER_END を含む。_TRACELEMENTS)
const MAX_BUFFER_SLOTS: usize = 1011;

type Slot = [u32; 4];

fn header(structure: u32, cpu: u8, class: u8, event: u16) -> u32 {
    (structure << def::STRUCT_SHIFT)
        | ((cpu as u32) << def::CPU_SHIFT)
        | ((class as u32) << def::CLASS_SHIFT)
        | ((event as u32) << def::EVENT_SHIFT)
}

/// 語数が 2 以下なら S、それ以外は CB, CC…, CE
fn event_slots(cpu: u8, class: u8, event: u16, low: u32, words: &[u32]) -> Vec<Slot> {
    let g = |i: usize| words.get(i).copied().unwrap_or(0);
    if words.len() <= 2 {
        return vec![[header(def::STRUCT_S, cpu, class, event), low, g(0), g(1)]];
    }
    let n = words.len().div_ceil(2);
    (0..n)
        .map(|p| {
            let st = match p {
                0 => def::STRUCT_CB,
                _ if p == n - 1 => def::STRUCT_CE,
                _ => def::STRUCT_CC,
            };
            [header(st, cpu, class, event), low, g(2 * p), g(2 * p + 1)]
        })
        .collect()
}

fn control(cpu: u8, event: u16, low: u32, words: [u32; 2]) -> Slot {
    [
        header(def::STRUCT_S, cpu, def::CLASS_CONTROL, event),
        low,
        words[0],
        words[1],
    ]
}

fn time_slot(cpu: u8, t: u64) -> Slot {
    let mut w = [0u32; 2];
    w[def::TIME_MSB_WORD] = (t >> 32) as u32;
    w[def::TIME_LSB_WORD] = t as u32;
    control(cpu, def::CONTROL_TIME, t as u32, w)
}

/// TIME・BUFFER(スロット数は後で直す)
fn begin_buffer(out: &mut Vec<Slot>, cpu: u8, t: u64, seq: u32) {
    out.push(time_slot(cpu, t));
    let mut w = [0u32; 2];
    w[def::BUFFER_SEQ_WORD] = seq;
    out.push(control(cpu, def::CONTROL_BUFFER, t as u32, w));
}

fn end_buffer(out: &mut Vec<Slot>, cpu: u8, low: u32) {
    out.push(control(cpu, def::CONTROL_BUFFER_END, low, [def::BUFFER_COOKIE; 2]));
}

/// BUFFER のスロット数を、直前の TIME から同じ CPU の次の BUFFER_END までの数にする
fn fix_counts(slots: &mut [Slot]) {
    let is = |s: &Slot, ev: u16| {
        let b = super::header_bits(s[0]);
        b.class == def::CLASS_CONTROL && b.event == ev && b.structure == def::STRUCT_S
    };
    let cpu = |s: &Slot| super::header_bits(s[0]).cpu;
    for i in 0..slots.len() {
        if !is(&slots[i], def::CONTROL_BUFFER) {
            continue;
        }
        let c = cpu(&slots[i]);
        let mut n = 1 + slots[..i]
            .iter()
            .rev()
            .find(|s| cpu(s) == c)
            .is_some_and(|s| is(s, def::CONTROL_TIME)) as u32;
        for s in &slots[i + 1..] {
            if cpu(s) != c {
                continue;
            }
            n += 1;
            if is(s, def::CONTROL_BUFFER_END) {
                break;
            }
        }
        slots[i][2 + def::BUFFER_COUNT_WORD] = n;
    }
}

/// 合成イベント
struct SynEvent {
    cpu: u8,
    cycle: u64,
    class: u8,
    event: u16,
    words: Vec<u32>,
}

/// イベントを CPU ごとのトレースバッファに詰め、トレースバッファの最後の時刻の順に並べる
fn pack(events: &[SynEvent], max_slots: usize, time_on_wrap: bool) -> Vec<Slot> {
    let mut by_cpu: BTreeMap<u8, Vec<&SynEvent>> = BTreeMap::new();
    for e in events {
        by_cpu.entry(e.cpu).or_default().push(e);
    }
    let mut bufs: Vec<(u64, u8, Vec<Slot>)> = Vec::new();
    for (&cpu, evs) in &by_cpu {
        let mut seq = 1;
        let mut i = 0;
        while i < evs.len() {
            let mut out = Vec::new();
            begin_buffer(&mut out, cpu, evs[i].cycle, seq);
            let mut msb = evs[i].cycle >> 32;
            let mut j = i;
            while j < evs.len() {
                let e = evs[j];
                let n = event_slots(cpu, e.class, e.event, 0, &e.words).len();
                let wrap = time_on_wrap && e.cycle >> 32 != msb;
                if j > i && out.len() + n + wrap as usize + 1 > max_slots {
                    break;
                }
                if wrap {
                    out.push(time_slot(cpu, e.cycle));
                }
                msb = e.cycle >> 32;
                out.extend(event_slots(cpu, e.class, e.event, e.cycle as u32, &e.words));
                j += 1;
            }
            let last = evs[j - 1].cycle;
            end_buffer(&mut out, cpu, last as u32);
            bufs.push((last, cpu, out));
            seq += 1;
            i = j;
        }
    }
    bufs.sort_by_key(|b| (b.0, b.1));
    let mut slots: Vec<Slot> = bufs.into_iter().flat_map(|b| b.2).collect();
    fix_counts(&mut slots);
    slots
}

/// 識別しやすい非印字の並び(文字列として読まれても "" になる)
fn pattern(n: usize) -> Vec<u32> {
    (0..n).map(|i| 0xF0F0_F080 + i as u32).collect()
}

/// 語 start 以降に NUL で終わる文字列を詰めた並び
fn with_string(n: usize, start: usize, s: &str) -> Vec<u32> {
    let mut w = pattern(n);
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    b.resize(b.len().next_multiple_of(4), 0);
    for (i, c) in b.chunks(4).enumerate() {
        if start + i < n {
            w[start + i] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        }
    }
    w
}

/// 書式が使う語
fn usage(fast: Spec, wide: Spec, min_words: usize) -> Usage {
    let mut u = Usage::default();
    u.add_spec(fast);
    u.add_spec(wide);
    u.min_required = u.min_required.max(min_words);
    u
}

/// 1 種類のイベントの変種(語数・内容)
fn variants(u: &Usage) -> Vec<Vec<u32>> {
    let mut out = Vec::new();
    // S(2 語)。書式が 3 語目以降を無条件に読むなら作らない(traceprinter がスロットの外を読む)
    if u.min_required <= 2 && !u.raw_str {
        out.push(pattern(2));
    }
    let need = u.max_arg.map_or(0, |m| m + 1).max(u.min_required);
    let full = need.max(4).next_multiple_of(2);
    let mut sizes: BTreeSet<usize> = [4, full, full + 2].into_iter().collect();
    sizes.retain(|&n| n >= u.min_required.max(4));
    for n in sizes {
        let mut w = pattern(n);
        if u.raw_str {
            for &sw in &u.str_words {
                for x in w.iter_mut().skip(sw) {
                    *x = 0x0000_4141;
                }
            }
        }
        out.push(w);
    }
    for &sw in &u.str_words {
        out.push(with_string(full + 4, sw, "tracehub_str"));
        out.push(with_string(full + 4, sw, "ab"));
    }
    // 値の名前の表: 1 ビットずつ立てたものと 0
    for &tw in &u.table_words {
        for bit in 0..=32u32 {
            let mut w = pattern(full);
            w[tw] = if bit == 32 { 0 } else { 1u32 << bit };
            if tw + 1 < full {
                w[tw + 1] = 0;
            }
            out.push(w);
        }
    }
    if u.dump {
        out.push(with_string(2, 0, "ab"));
        out.push(with_string(6, 0, "hello_user"));
        out.push(with_string(6, 0, "hello\u{1}"));
    }
    out
}

/// 内部 ID → 書式の使う語(args.rs に無ければ None)
fn usages() -> (BTreeMap<(u8, u16), Usage>, BTreeMap<u8, Usage>) {
    let mut ev = BTreeMap::new();
    let mut wild = BTreeMap::new();
    for e in args::EVENTS {
        let u = usage(e.fast, e.wide, e.min_words as usize);
        match e.event {
            Some(id) => {
                ev.insert(internal_ids(e.class, id).unwrap(), u);
            }
            None => {
                wild.insert(internal_class(e.class).unwrap(), u);
            }
        }
    }
    (ev, wild)
}

/// 全イベントの目録: (内部クラス, 内部イベント番号, payloads)
fn catalog() -> Vec<(u8, u16, Vec<u32>)> {
    let (ev, wild) = usages();
    let mut out = Vec::new();
    for (&(c, code), u) in &ev {
        if c == def::CLASS_CONTROL {
            continue; // CONTROL はトレースバッファの枠として作る
        }
        for w in variants(u) {
            out.push((c, code, w));
        }
    }
    // カーネルコール: 失敗した exit、定義の無い番号、exit と int の両方のビット
    for num in 0..=def::KERCALL_NUM_MASK {
        for kind in [
            0,
            def::KERCALL_EXIT,
            def::KERCALL_INT,
            def::KERCALL_EXIT | def::KERCALL_INT,
        ] {
            for b64 in [0, def::BIT64] {
                let code = num | kind | b64;
                if kind == def::KERCALL_EXIT {
                    out.push((def::CLASS_KER_CALL, code, vec![0xffff_ffff, 22]));
                    out.push((
                        def::CLASS_KER_CALL,
                        code,
                        vec![0xffff_ffff, 22, 0xF0F0_F082, 0xF0F0_F083],
                    ));
                }
                if !ev.contains_key(&(def::CLASS_KER_CALL, code)) {
                    out.push((def::CLASS_KER_CALL, code, pattern(2)));
                    out.push((def::CLASS_KER_CALL, code, pattern(4)));
                }
            }
        }
    }
    // スレッドの状態の定義の無い番号
    for code in 0..def::PR_TH_PROCESS_BASE {
        if ev.contains_key(&(def::CLASS_PR_TH, code)) {
            continue;
        }
        for w in variants(&wild[&def::CLASS_PR_TH]) {
            out.push((def::CLASS_PR_TH, code, w));
        }
    }
    // そのほかのクラスの定義の無い番号
    for &c in super::DEFINED_CLASSES {
        if c == def::CLASS_KER_CALL || c == def::CLASS_CONTROL {
            continue;
        }
        let known: Vec<u16> = match c {
            def::CLASS_PR_TH => def::PR_TH_PROCESS
                .iter()
                .chain(def::PR_TH_VTHREAD)
                .map(|r| r.0)
                .collect(),
            _ => super::event_table(c).iter().map(|r| r.0).collect(),
        };
        let mut codes: BTreeSet<u16> = match known.iter().max() {
            Some(&m) if c == def::CLASS_PR_TH => (def::PR_TH_PROCESS_BASE..=m + 70).collect(),
            Some(&m) => (0..=m + 3).collect(),
            None => [0, 1, 5, 1023].into(),
        };
        if c == def::CLASS_SYSTEM {
            let base: Vec<u16> = codes.iter().copied().collect();
            codes.extend(base.into_iter().map(|x| x | def::BIT64));
        }
        for code in codes {
            if ev.contains_key(&(c, code)) {
                continue;
            }
            for w in variants(wild.get(&c).unwrap_or(&Usage::default())) {
                out.push((c, code, w));
            }
        }
    }
    // 定義の無いクラス
    for c in 0..=def::CLASS_MASK as u8 {
        if super::DEFINED_CLASSES.contains(&c) {
            continue;
        }
        for code in [1u16, 7, 0x3ff] {
            out.push((c, code, vec![0x1111_0000 + c as u32, code as u32]));
            out.push((c, code, vec![1, 2, 3, 4]));
        }
    }
    out
}

/// 目録を 1 CPU の時系列にする(最後に DROPPED を 1 件)
fn catalog_events() -> Vec<SynEvent> {
    let mut cycle: u64 = (4u64 << 32) | 0x1000_0000;
    let mut out = Vec::new();
    for (class, event, words) in catalog() {
        cycle += 0x40;
        out.push(SynEvent {
            cpu: 0,
            cycle,
            class,
            event,
            words,
        });
    }
    cycle += 0x40;
    let mut w = vec![0u32; 2];
    w[def::DROPPED_COUNT_WORD] = 5;
    out.push(SynEvent {
        cpu: 0,
        cycle,
        class: def::CLASS_CONTROL,
        event: def::CONTROL_DROPPED,
        words: w,
    });
    out
}

fn code(class: ClassId, event: EventId) -> (u8, u16) {
    internal_ids(class, event).unwrap()
}

/// 語が足りないイベント: S(2 語)・CB だけ・CB CC で途切れた組
fn short_words() -> Vec<Slot> {
    let mut longest: BTreeMap<(u8, u16), Vec<u32>> = BTreeMap::new();
    for (c, code, w) in catalog() {
        let e = longest.entry((c, code)).or_default();
        if w.len() > e.len() {
            *e = w;
        }
    }
    let (ev, wild) = usages();
    let (pr_th, running) = code(ClassId::THREAD, thread::RUNNING);
    let g = |v: &[u32], i: usize| v.get(i).copied().unwrap_or(0);
    // (構造, クラス, イベント番号, 時刻の増分, payloads)
    type Part = (u32, u8, u16, u64, [u32; 2]);
    let mut groups: Vec<Vec<Part>> = Vec::new();
    for (&(c, code), v) in &longest {
        if !super::DEFINED_CLASSES.contains(&c) {
            continue;
        }
        let u = match ev.get(&(c, code)) {
            Some(u) => u.clone(),
            None if c == def::CLASS_PR_TH && code < def::PR_TH_PROCESS_BASE => wild[&def::CLASS_PR_TH].clone(),
            None => continue,
        };
        let req = if u.raw_str { usize::MAX } else { u.min_required };
        let run: Part = (def::STRUCT_S, pr_th, running, 0x10, [1, 1]);
        if req <= 2 && u.str_words.is_empty() {
            groups.push(vec![(def::STRUCT_S, c, code, 0x10, [g(v, 0), g(v, 1)])]);
        }
        if req <= 2 {
            groups.push(vec![(def::STRUCT_CB, c, code, 0x10, [g(v, 0), g(v, 1)]), run]);
        }
        if v.len() >= 6 && req <= 4 {
            groups.push(vec![
                (def::STRUCT_CB, c, code, 0x10, [g(v, 0), g(v, 1)]),
                (def::STRUCT_CC, c, code, 0, [g(v, 2), g(v, 3)]),
                run,
            ]);
        }
    }
    let mut out = Vec::new();
    let mut t: u64 = (5u64 << 32) | 0x100;
    let mut seq = 1;
    let mut i = 0;
    while i < groups.len() {
        let start = out.len();
        begin_buffer(&mut out, 0, t, seq);
        while i < groups.len() && out.len() - start + groups[i].len() < MAX_BUFFER_SLOTS {
            for &(st, c, code, dt, d) in &groups[i] {
                t += dt;
                out.push([header(st, 0, c, code), t as u32, d[0], d[1]]);
            }
            i += 1;
        }
        t += 0x10;
        end_buffer(&mut out, 0, t as u32);
        t += 0x10;
        seq += 1;
    }
    fix_counts(&mut out);
    out
}

/// 組み立て中の組の後ろにたまる項目の数の境目(窓 - 1 / 窓。項目は S か組)と、
/// 別の CPU の項目が merge の順で間に入る場合(2 CPU)
fn combine_window() -> Vec<Slot> {
    let (pr_th, name_t) = code(ClassId::PROCESS, process::NAME_T);
    let (_, running) = code(ClassId::THREAD, thread::RUNNING);
    let ker = def::CLASS_KER_CALL;
    let sendv = code(ClassId::KER_CALL_ENTER, kercall::MSG_SENDV).1;
    let ab = u32::from_le_bytes(*b"ab\0\0");
    let win = args::COMBINE_WINDOW;
    let per = 1000usize; // 1 トレースバッファの本体のスロット数の上限
    let mut out = Vec::new();
    let mut t: u64 = (1u64 << 32) | 0x100;
    let mut seq = [1u32; 2];
    let buffer = |out: &mut Vec<Slot>, cpu: u8, t: u64, body: Vec<Slot>, end: u32, seq: &mut [u32; 2]| {
        begin_buffer(out, cpu, t, seq[cpu as usize]);
        seq[cpu as usize] += 1;
        out.extend(body);
        end_buffer(out, cpu, end);
    };
    for (n, as_combine) in [(win - 1, false), (win, false), (win - 1, true), (win, true)] {
        let tc = t as u32 + 0x10;
        let mut tt = t + 0x20;
        // トレースバッファの境目の TIME・BUFFER・BUFFER_END も項目に数えるので、境目 1 つにつき項目を 3 つ減らす
        let mut bodies: Vec<Vec<Slot>> = vec![vec![[header(def::STRUCT_CB, 0, pr_th, name_t), tc, 1, 1]]];
        let mut items = 0usize;
        while items < n {
            let size = if as_combine { 2 } else { 1 };
            if bodies.last().unwrap().len() + size > per {
                bodies.push(Vec::new());
                items += 3;
                continue;
            }
            tt += 0x10;
            let b = bodies.last_mut().unwrap();
            if as_combine {
                b.push([header(def::STRUCT_CB, 0, ker, sendv), tt as u32, 1, 1]);
                b.push([header(def::STRUCT_CE, 0, ker, sendv), tt as u32, 2, 2]);
            } else {
                b.push([header(def::STRUCT_S, 0, pr_th, running), tt as u32, 1, items as u32]);
            }
            items += 1;
        }
        let last = bodies.last_mut().unwrap();
        last.push([header(def::STRUCT_CC, 0, pr_th, name_t), tc, ab, 0]);
        last.push([header(def::STRUCT_CE, 0, pr_th, name_t), tc, 0, 0]);
        for b in bodies {
            buffer(&mut out, 0, t, b, tt as u32 + 0x10, &mut seq);
        }
        t = tt + 0x1000;
    }
    // CPU 0: CB、時刻の遅い S、CC CE。CPU 1: その間の時刻に n 個(少数なら組は閉じる、3 × 窓なら途切れる)。
    // 最後に CPU 1 の項目を CPU 0 のすべてより後の時刻に置く(どの組にも影響しない)
    let mut cpu1: Vec<(u64, usize)> = Vec::new();
    for n in [10usize, 3 * win] {
        let tc = t + 0x10;
        let body = vec![
            [header(def::STRUCT_CB, 0, pr_th, name_t), tc as u32, 1, 1],
            [header(def::STRUCT_S, 0, pr_th, running), (tc + 0x10_0000) as u32, 1, 1],
            [header(def::STRUCT_CC, 0, pr_th, name_t), tc as u32, ab, 0],
            [header(def::STRUCT_CE, 0, pr_th, name_t), tc as u32, 0, 0],
        ];
        buffer(&mut out, 0, t, body, (tc + 0x10_0010) as u32, &mut seq);
        cpu1.push((tc + 0x10, n));
        t = tc + 0x20_0000;
    }
    cpu1.push((t + 0x100_0000, 3 * win));
    for (t1, total) in cpu1 {
        let mut n = 0usize;
        while n < total {
            let mut body = Vec::new();
            while n < total && body.len() + 3 < per {
                body.push([header(def::STRUCT_S, 1, pr_th, running), t1 as u32 + 0x10, 2, n as u32]);
                n += 1;
            }
            buffer(&mut out, 1, t1, body, t1 as u32 + 0x20, &mut seq);
        }
    }
    fix_counts(&mut out);
    out
}

/// 多数のイベントの後の orphan(区切りごとに CC と CE)
fn orphan_lookahead() -> Vec<Slot> {
    let (pr_th, name_t) = code(ClassId::PROCESS, process::NAME_T);
    let (_, running) = code(ClassId::THREAD, thread::RUNNING);
    let mut out = Vec::new();
    let mut t: u64 = (1u64 << 32) | 0x100;
    let mut seq = 1;
    let mut n = 0usize;
    let plan = [10usize, 1500, 1030, 1022, 1023, 5];
    for (pi, &count) in plan.iter().enumerate() {
        let mut left = count;
        while left > 0 {
            let start = out.len();
            begin_buffer(&mut out, 0, t, seq);
            seq += 1;
            while left > 0 && out.len() - start < 1000 {
                t += 0x10;
                if n.is_multiple_of(97) {
                    let w = [1, 2, u32::from_le_bytes(*b"la\0\0"), 0];
                    out.extend(event_slots(0, pr_th, name_t, t as u32, &w));
                } else {
                    out.push([header(def::STRUCT_S, 0, pr_th, running), t as u32, 1, n as u32]);
                }
                n += 1;
                left -= 1;
            }
            if left == 0 && pi + 1 < plan.len() {
                t += 0x10;
                let st = if pi % 2 == 0 { def::STRUCT_CC } else { def::STRUCT_CE };
                out.push([header(st, 0, pr_th, name_t), t as u32, 0xAAAA_0000 + pi as u32, 0]);
            }
            t += 0x10;
            end_buffer(&mut out, 0, t as u32);
        }
    }
    fix_counts(&mut out);
    out
}

/// SLOG2 の引数(数はバイト数と値、文字列はデータそのまま)
enum P {
    I(usize, i64),
    S(&'static [u8]),
}

/// SLOG2 のレコード(種別は書式)。レコード長は実際の長さ
fn slog2_record(fmt: &[u8], params: &[P]) -> Vec<u8> {
    let mut r = vec![0u8; args::SLOG2_BODY_OFFSET];
    r[..2].copy_from_slice(&args::SLOG2_MAGIC.to_le_bytes());
    r[4..7].copy_from_slice(&[0xb7, 0x70, 0x04]); // 読まれないバイト
    r[args::SLOG2_TYPE_OFFSET] = args::SLOG2_TYPE_FORMAT;
    r.extend_from_slice(fmt);
    r.push(0);
    for p in params {
        r.resize(r.len().next_multiple_of(args::SLOG2_ARG_ALIGN), 0);
        let (h, d) = match p {
            P::I(n, v) => (
                *n as u16,
                (0..*n).map(|k| v.to_le_bytes().get(k).copied().unwrap_or(0)).collect(),
            ),
            P::S(s) => (s.len() as u16 | args::SLOG2_ARG_STRING, s.to_vec()),
        };
        r.extend_from_slice(&h.to_le_bytes());
        r.extend_from_slice(&d);
    }
    set_len(&mut r, None);
    r
}

/// レコード長を書く(None なら実際の長さ)
fn set_len(r: &mut [u8], len: Option<usize>) {
    let n = len.unwrap_or(r.len()) as u16;
    r[args::SLOG2_LEN_OFFSET..args::SLOG2_LEN_OFFSET + 2].copy_from_slice(&n.to_le_bytes());
}

fn to_words(b: &[u8]) -> Vec<u32> {
    let mut b = b.to_vec();
    b.resize(b.len().next_multiple_of(4), 0);
    b.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
}

/// SLOG2: レコードの規則(先頭・長さ・種別・上限)と printf の変換(フラグ・幅・精度・長さ修飾子・型の不一致)
fn slog2_events() -> Vec<SynEvent> {
    use P::{I, S};
    let f = |v: f64| I(8, v.to_bits() as i64);
    let i4 = |v: i64| I(4, v);
    let i8 = |v: i64| I(8, v);
    let mut recs: Vec<Vec<u8>> = Vec::new();
    let mut add = |fmt: &[u8], p: &[P]| recs.push(slog2_record(fmt, p));
    add(
        b"%s(%d)[tid:%d]: pid %d proc=%s client %p",
        &[
            S(b"connect\0"),
            i4(62),
            i4(5),
            i4(1234),
            S(b"server 1234\0"),
            i8(0x12_3456_78d0),
        ],
    );
    add(b"hello", &[]);
    add(
        b"%d %i %u|%d %u|%x %X %o",
        &[i4(-5), i4(-6), i4(-7), i8(-5), i8(-7), i4(255), i4(255), i4(8)],
    );
    add(
        b"[%5d][%-5d][%05d][%.3s][%10s][%-10s]",
        &[i4(42), i4(42), i4(42), S(b"abcdef\0"), S(b"ab\0"), S(b"ab\0")],
    );
    add(b"%c%c 100%% %p %p", &[I(1, 65), i4(66), i8(0x1234), i4(0x5678)]);
    // 引数が足りない・型が合わない・扱わない長さ
    add(b"%s", &[i4(7)]);
    add(b"%d", &[S(b"str\0")]);
    add(b"%d %d", &[i4(1)]);
    add(b"%d", &[I(3, 0x10203)]);
    add(b"%s", &[S(b"noterm")]);
    add(b"[%s]", &[S(b"")]);
    add(b"[%s]", &[I(1, 0x41)]);
    add(b"[%s|%s|%s]", &[I(2, 0x41), i8(0x6162), I(1, 0)]);
    add(b"[%c|%p|%x|%f]", &[S(b"Q\0")]);
    add(b"[%d]", &[I(0, 0)]);
    add(b"[%d]", &[I(5, 1)]);
    add(b"[%d]", &[I(32, 2)]);
    add(b"e%d", &[i4(1), i4(2)]);
    add(b"e%d", &[i4(1), S(b"zz\0")]);
    add(b"plain", &[i4(1)]);
    // 長さ修飾子
    add(
        b"%ld %lld %lu %llx %hd %hhu",
        &[i8(-1), i8(-2), i8(3), i8(0xabc), I(2, -4), I(1, 200)],
    );
    add(b"[%ld %lld %lu]", &[i4(-1), i4(-1), i4(-1)]);
    add(b"[%hhd %hd %hhx]", &[i4(0x1ff), i4(0x1ffff), i4(0x1ff)]);
    add(
        b"[%lld %llu %llx|%lld %llu %hd %hu|%lld %llx]",
        &[
            i4(-1),
            i4(-1),
            i4(0x8000_0000),
            I(2, -1),
            I(1, -1),
            I(2, -1),
            I(2, -1),
            I(1, -1),
            I(1, 0x80),
        ],
    );
    add(
        b"[%d %u %x|%zx %zd %jx %tx %lx %Lx]",
        &[
            i4(0x8000_0000),
            i4(0x8000_0000),
            i4(0x8000_0000),
            i8(-1),
            i8(-1),
            i8(-1),
            i8(-1),
            i8(-1),
            i8(-1),
        ],
    );
    add(
        b"[%jd|%td|%zd|%ju|%tu|%zu]",
        &[i4(-1), i4(-1), i4(-1), i4(-1), i4(-1), i4(-1)],
    );
    add(
        b"[%ld|%lu|%li|%lx|%lli|%llo]",
        &[
            i8(0x1_0000_0001),
            i8(0x1_0000_0001),
            i8(0x1_0000_0001),
            i8(0x1_0000_0001),
            I(1, 0x80),
            I(2, 0x8000),
        ],
    );
    add(
        b"[%o %X][%hhu|%hu|%hhx|%hx|%ho]",
        &[i8(-1), i8(-1), i4(0x1ff), i4(0x1ffff), i8(-1), i8(-1), i8(-1)],
    );
    add(
        b"[%p %p %p][%c][%c]",
        &[I(1, 0xff), I(2, 0xffff), i8(-1), i8(0x4142), i4(0x1ff)],
    );
    // フラグ・幅・精度
    add(
        b"[%#x %+d % d %08.3f][%zu %jd %td]",
        &[i4(255), i4(5), i4(5), i8(0), i8(1), i8(2), i8(3)],
    );
    add(b"[%p %#p %10p %-20p|]", &[i4(-1), i4(1), i4(1), i4(1)]);
    add(
        b"[%+s %#o %#X % x][%2c|%-2c]",
        &[S(b"a\0"), i4(8), i4(255), i4(1), i4(0x41), i4(0x42)],
    );
    add(
        b"[%.4p|%020p|%#20p|%#p|%-p|%+p|% p|%0p|%1p|%.0p]",
        &[i8(0), i8(0), i8(0), i8(0), i8(0), i8(0), i8(0), i8(0), i8(0), i8(0)],
    );
    add(b"[%lp %hp %llp %zp]", &[i8(0x41), i8(0x41), i8(0x41), i8(0x41)]);
    add(
        b"[%#x|%#o|%#.0o|%.0d|%.0x|%#.3x|%#08x|%#.8x]",
        &[i8(0), i8(0), i8(0), i4(0), i8(0), i8(0), i8(0), i8(0)],
    );
    add(
        b"[%-05d|%+05d|% 05d|%+ d|%-+#0 8.3d|%000005d|%.10d|%0d]",
        &[i4(42), i4(42), i4(42), i4(42), i4(42), i4(42), i4(42), i4(3)],
    );
    add(
        b"[%#5c|%.3c|%05c|%-05c][%05s|%#s|%-.3s|%010s|%-010s]",
        &[
            i4(0x41),
            i4(0x41),
            i4(0x41),
            i4(0x41),
            S(b"abcdef\0"),
            S(b"abcdef\0"),
            S(b"abcdef\0"),
            S(b"ab\0"),
            S(b"ab\0"),
        ],
    );
    add(
        b"[%#5x|%-#8o|%#X|%08.3d|%-8.3x|%+u|% u][%5.2d]",
        &[i4(255), i4(8), i4(0), i4(-5), i4(10), i4(1), i4(1), i4(3)],
    );
    add(
        b"[%.0d|%+.0d|% .0d|%5.0d][%#o|%#.0o|%#5o]",
        &[i4(0), i4(0), i4(0), i4(0), i4(0), i4(0), i4(8)],
    );
    add(
        b"[% u|% llu|% 5u|%- 5u|% 05u|% hu|% hhu|%+ u|% lu|% zu|%#u|% .3u|% .0u]",
        &[
            i4(7),
            i8(-1),
            i4(7),
            i4(7),
            i4(7),
            i4(-1),
            i4(-1),
            i4(0),
            i4(-1),
            i8(-1),
            i4(7),
            i4(7),
            i4(0),
        ],
    );
    add(
        b"[% x|% o|% X|%+x|% p|% i|%+i|% i]",
        &[i4(7), i4(7), i4(7), i4(7), i4(7), i4(7), i4(0), i4(-1)],
    );
    add(b"[%99999d]", &[i4(1)]);
    add(b"[%.99999d]", &[i4(1)]);
    add(b"[%300d][%490d]", &[i4(1), i4(1)]);
    // 扱わない書き方
    for fmt in [
        &b"[%.d]"[..],
        b"[% %d]",
        b"[%5%]",
        b"[%-%]",
        b"abc %d%",
        b"abc%",
        b"[%*d]",
        b"[%1$d]",
        b"[%n%q%]",
        b"[%I64d]",
        b"[%qd]",
        b"[%Zd]",
        b"[%Cd]",
        b"[%S]",
        b"[%C]",
        b"[%hn]",
        b"[%D]",
        b"[%U]",
        b"[%O]",
        b"[%b]",
        b"[%m]",
        b"[%ls %lc]",
    ] {
        add(fmt, &[i8(0x41), i8(0x41)]);
    }
    add(
        b"[%Lf %lf %llf %hf %jf][%hs %ls %lls %hc %lc %hhs][%%%d]",
        &[
            i8(0x41),
            i8(0x41),
            i8(0x41),
            i8(0x41),
            i8(0x41),
            S(b"A\0"),
            S(b"A\0"),
            S(b"A\0"),
            i8(0x41),
            i8(0x41),
            S(b"A\0"),
            i8(0x41),
        ],
    );
    // 浮動小数点
    for v in [
        0.0,
        5e-324,
        1.0,
        100.0,
        1e-6,
        1e23,
        -0.0,
        f64::MAX,
        0.1,
        10.0,
        1e4,
        1e6,
        1e-4,
        f64::MIN_POSITIVE,
        -f64::NAN,
        f64::NEG_INFINITY,
    ] {
        add(
            b"[%f|%e|%g|%a|%.3g|%#g|%G|%.0g|%.15g|%.17g]",
            &[f(v), f(v), f(v), f(v), f(v), f(v), f(v), f(v), f(v), f(v)],
        );
    }
    add(b"[%f %e %G %E %F %A]", &[i4(1), i4(1), i4(1), i4(1), i4(1), i4(1)]);
    add(
        b"[%.2f %10.3e][%#g %#.0f %.0e %.20f %g %.3g %10.4g]",
        &[
            f(-2.0),
            f(-1.0),
            f(1.2345),
            f(1.2345),
            f(1.2345),
            f(1.2345),
            f(1.2345),
            f(1.2345),
            f(1.2345),
        ],
    );
    add(b"[%.100f]", &[f(1.2345)]);
    add(
        b"[%F %F %E %+f % f %+e]",
        &[
            f(f64::INFINITY),
            f(f64::NAN),
            f(f64::INFINITY),
            f(f64::NAN),
            f(f64::NAN),
            f(-f64::NAN),
        ],
    );
    add(
        b"[%10f|%-10.2f|%010.2f|%+.1f|% .1f|%010f]",
        &[f(1.5), f(1.5), f(1.5), f(1.5), f(1.5), f(f64::NEG_INFINITY)],
    );
    add(
        b"[%.1a|%.0a|%.3a|%10a|%#a|%.0A]",
        &[f(1.5), f(1.5), f(0.1), f(1.5), f(1.0), f(1.5)],
    );
    add(
        b"[%g|%g|%g|%.0g|%#.3g|%.2g|%G]",
        &[f(1e8), f(0.001), f(1e-4), f(0.5), f(100.0), f(99.5), f(1e-7)],
    );
    add(
        b"[%.2e|%.0f|%.0f|%.0f|%.1f]",
        &[f(99.5), f(0.5), f(1.5), f(2.5), f(0.1)],
    );
    add(b"[% f|% e|% g|% a]", &[i8(-1), i4(7), i4(-1), i4(7)]);
    // ワイド文字(%ls・%lc など)と、%s のデータの最後の NUL
    for m in ["l", "ll", "j", "z", "t", "L", "h", "hh"] {
        let fmt = format!("[%{m}s|%{m}s|%{m}c|%{m}c]");
        add(fmt.as_bytes(), &[S(b"A\0B\0\0\0"), I(2, 0x41), i4(0x41), i4(0xe9)]);
        add(format!("[%{m}c]").as_bytes(), &[S(b"A\0")]);
    }
    add(
        b"[%ls|%ls|%ls|%ls|%ls]",
        &[
            S(b"A\0\x42\x30B\0\0\0"),
            S(b"A\0\x00\x01B\0\0\0"),
            S(b"A\0\xff\x00B\0\0\0"),
            S(b"\0"),
            S(b"A\0\0"),
        ],
    );
    add(b"[%ls]%d", &[S(b"A\0"), i4(0x0043_0042)]);
    add(b"[%lls]|", &[S(b"A\0B\0")]);
    add(
        b"[%.1lls|%5lls|%-5lls]",
        &[S(b"A\0B\0\0\0"), S(b"A\0\0\0"), S(b"A\0\0\0")],
    );
    add(b"[%lc|%lc|%c|%hc]", &[i4(0x1ff), i4(0x3042), i4(0x3042), i4(0x3042)]);
    for bad in [&b"A\0\0B"[..], b"A\0B", b""] {
        add(b"[%ls]", &[S(bad)]);
    }
    add(b"[%s]", &[S(b"a\0b")]);
    add(b"[%s|%s]", &[i4(0x6300_6261), i4(0x6261)]);
    // 文字列そのもの(制御文字・改行・NUL)
    add(b"tab\there\nnl a\r\nb", &[]);
    add(b"\xe3\x81\x82 \x01\x7f\x80\xff", &[]);
    add(b"[%s][%s]", &[S(b"a\x01b\xe3\x81\x82\x7f\x80\xff\0"), S(b"ab\0cd\0")]);
    add(b"[%s][%c][%c]", &[S(b"a\nb\0"), i4(10), i4(0)]);
    // 長さの上限(書式 511 / 512 バイト、レコード 536 / 537 バイト、出力 1023 バイト)
    for n in [511usize, 512] {
        add(&vec![b'z'; n], &[]);
    }
    let text = |n: usize, c: u8| -> &'static [u8] {
        let mut v = vec![c; n];
        v.push(0);
        Box::leak(v.into_boxed_slice())
    };
    for n in [507usize, 508, 509, 510] {
        add(b"%s", &[S(text(n, b'q'))]);
    }
    add(b"%s%s", &[S(text(250, b'y')), S(text(250, b'y'))]);
    add(b"%s%s", &[S(text(300, b'y')), S(text(300, b'y'))]);

    // レコードの規則
    let base = slog2_record(b"L%d", &[i4(7)]);
    let mut raw = |r: Vec<u8>| recs.push(r);
    for t in [0u8, 1, 2, 3, 0x10, 0x81, 0xff] {
        let mut r = base.clone();
        r[args::SLOG2_TYPE_OFFSET] = t;
        raw(r);
    }
    // 先頭・レコード長・種別のほかのバイトは読まれない
    for k in [4usize, 5, 6, 8, 12, 16, 19] {
        let mut r = base.clone();
        r[k] ^= 0x55;
        raw(r);
    }
    for magic in [0xcfbcu16, 0xcfbf, 0xcebd, 0x4fbd, 0x0000, 0xffff] {
        let mut r = base.clone();
        r[..2].copy_from_slice(&magic.to_le_bytes());
        raw(r);
    }
    for len in [0usize, 4, 19, 20, 21, 22, 23, 24, 26, 28, 29, 31, 32, 34, 130] {
        let mut r = base.clone();
        set_len(&mut r, Some(len));
        raw(r);
    }
    let mut text = base.clone();
    text[args::SLOG2_TYPE_OFFSET] = args::SLOG2_TYPE_STRING;
    text.truncate(args::SLOG2_BODY_OFFSET);
    text.extend_from_slice(b"plain%d\nx\0");
    for len in [None, Some(22), Some(21), Some(20), Some(0), Some(40)] {
        let mut r = text.clone();
        set_len(&mut r, len);
        raw(r);
    }
    for total in [535usize, 536, 537] {
        let mut r = text.clone();
        r.truncate(args::SLOG2_BODY_OFFSET);
        r.resize(total - 1, b'r');
        r.push(0);
        set_len(&mut r, None);
        raw(r);
    }
    // 引数のヘッダの揃え
    let mut odd = base.clone();
    odd.truncate(args::SLOG2_BODY_OFFSET);
    odd.extend_from_slice(b"ab%d\0");
    for pad in [0usize, 1, 3] {
        let mut r = odd.clone();
        r.resize(r.len() + pad, 0);
        r.extend_from_slice(&4u16.to_le_bytes());
        r.extend_from_slice(&5i32.to_le_bytes());
        set_len(&mut r, None);
        raw(r);
    }
    // payloads が短い(S・2 語・4 語)
    raw(base[..8].to_vec());
    raw(base[..16].to_vec());

    let (system, slog) = code(ClassId::SYSTEM, system::SLOG);
    let mut out: Vec<SynEvent> = recs
        .iter()
        .enumerate()
        .map(|(i, r)| SynEvent {
            cpu: 0,
            cycle: 0x100 + 0x10 * i as u64,
            class: system,
            event: slog,
            words: to_words(r),
        })
        .collect();
    // 64 ビット版の番号は SLOG2 として読まない(行を出さない)
    out.push(SynEvent {
        cpu: 0,
        cycle: 0x100 + 0x10 * out.len() as u64,
        class: system,
        event: slog | def::BIT64,
        words: to_words(&base),
    });
    out
}

/// 複数 CPU・下位 32 ビットの一周・CPU 間の同時刻を含む、スケジューリングのような列
fn sched_events(cpus: u8, per_cpu: usize) -> Vec<SynEvent> {
    let (pr_th, running) = code(ClassId::THREAD, thread::RUNNING);
    let (_, ready) = code(ClassId::THREAD, thread::READY);
    let ker = def::CLASS_KER_CALL;
    let yield_num = code(ClassId::KER_CALL_ENTER, kercall::SCHED_YIELD).1;
    let mut out = Vec::new();
    let start: u64 = (7u64 << 32) | 0xFFF0_0000;
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    for cpu in 0..cpus {
        let mut cycle = start + cpu as u64 * 3;
        for i in 0..per_cpu {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let step = if i % 17 == 0 { 0x400 } else { 0x100 + (seed % 0x1800) };
            cycle += step;
            if i % 17 == 0 {
                cycle = (cycle + 0x3ff) & !0x3ff; // CPU 間で同時刻になるように
            }
            let pid = 1000 + cpu as u32;
            let tid = 1 + (i % 5) as u32;
            let (class, event, words) = match i % 4 {
                0 => (pr_th, running, vec![pid, tid, 10, 2]),
                1 => (ker, yield_num, vec![0, 0]),
                2 => (ker, yield_num | def::KERCALL_EXIT, vec![0, 0]),
                _ => (pr_th, ready, vec![pid, tid, 10, 2]),
            };
            out.push(SynEvent {
                cpu,
                cycle,
                class,
                event,
                words,
            });
        }
    }
    out
}

/// kevcsv の CSV にする(ヘッダは traceprinter が読む最小限、syspage は無し)
fn to_csv(cpus: u8, slots: &[Slot]) -> String {
    let mut s = String::from("[header],key,value\r\n");
    for (k, v) in [
        ("TRACE_HEADER_BEGIN::", String::new()),
        ("TRACE_VER_MAJOR::", "2".into()),
        ("TRACE_VER_MINOR::", "01".into()),
        ("TRACE_LITTLE_ENDIAN::", "TRUE".into()),
        ("TRACE_ENCODING::", "16 byte events".into()),
        ("TRACE_CYCLES_PER_SEC::", "19200000".into()),
        ("TRACE_CPU_NUM::", cpus.to_string()),
        ("TRACE_SYS_RELEASE::", "8.0.0".into()),
        ("TRACE_SYSPAGE_LEN::", "0".into()),
        ("TRACE_HEADER_END::", String::new()),
    ] {
        let _ = write!(s, "on,{k},{v}\r\n");
    }
    s.push_str("[syspage],offset,bytes\r\n[slots],header,stamp,payload0,payload1\r\n");
    for w in slots {
        let _ = write!(s, "on,0x{:08x},0x{:08x},0x{:08x},0x{:08x}\r\n", w[0], w[1], w[2], w[3]);
    }
    s
}

/// CSV(この生成器の形)を kev のバイト列にする
pub fn kev_bytes(csv: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut sec = "";
    for line in csv.lines() {
        let cells: Vec<&str> = line.split(',').collect();
        if cells[0].starts_with('[') {
            sec = cells[0];
        } else if sec == "[header]" {
            out.extend_from_slice(cells[1].as_bytes());
            out.extend_from_slice(cells[2].as_bytes());
        } else if sec == "[slots]" {
            for c in &cells[1..5] {
                out.extend_from_slice(&u32::from_str_radix(&c[2..], 16).unwrap().to_le_bytes());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DecodeOptions, DiagKind, MissingStyle};

    /// 定義の無いイベントは Unknown(SEC のように traceprinter が行を出すものは Display)
    #[test]
    fn unknown_events_have_unknown_display_status() {
        use crate::model::DisplayStatus;
        use crate::semantic_id::EventId;
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/kev/data/unknown_event.csv");
        let kev = crate::parse(&kev_bytes(&std::fs::read_to_string(path).unwrap())).unwrap();
        let status = |class: u8, event: u16| {
            let ev = kev
                .events
                .iter()
                .find(|e| e.int_class_id == class && e.int_event_id == event)
                .unwrap();
            (ev.event_id == EventId::UNKNOWN, ev.display_status)
        };
        for (class, event) in [
            (def::CLASS_SYSTEM, 0x20),
            (def::CLASS_SYSTEM, 0x220),
            (def::CLASS_CONTROL, 0x10),
            (def::CLASS_QVM, 0x1ff),
        ] {
            assert_eq!(
                status(class, event),
                (true, DisplayStatus::Unknown),
                "{class} {event:#x}"
            );
        }
        assert_eq!(status(def::CLASS_SEC, 0x3ff), (true, DisplayStatus::Display));
        assert_eq!(status(def::CLASS_PR_TH, 0x3f), (true, DisplayStatus::Unknown));
        let (pr_th, running) = code(ClassId::THREAD, thread::RUNNING);
        assert_eq!(status(pr_th, running), (false, DisplayStatus::Display));
    }

    #[test]
    fn scenarios_are_well_framed_and_decodable() {
        for (name, csv) in scenarios() {
            let kev = crate::parse(&kev_bytes(&csv)).unwrap();
            for d in &kev.diag {
                assert!(
                    !matches!(
                        d.kind,
                        DiagKind::CountMismatch { .. }
                            | DiagKind::SeqGap { .. }
                            | DiagKind::MissingBufferEnd { .. }
                            | DiagKind::CookieMismatch { .. }
                            | DiagKind::Interleaved { .. }
                            | DiagKind::OutsideBuffer { .. }
                    ),
                    "{name}: {d:?}"
                );
            }
            // traceprinter が行を出す 64 ビット版には、セマンティック ID がある(def.rs の 64 ビット版の表に漏れが無い)
            for ev in &kev.events {
                let is64 = matches!(ev.int_class_id, def::CLASS_KER_CALL | def::CLASS_SYSTEM)
                    && ev.int_event_id & def::BIT64 != 0;
                if is64
                    && ev.combine_status == crate::model::CombineStatus::Complete
                    && crate::qnx800::event_decoder::format(
                        ev.int_class_id,
                        ev.int_event_id,
                        &ev.payloads,
                        crate::tp_format::Missing::Omit,
                    )
                    .is_some()
                {
                    assert_ne!(
                        ev.event_id,
                        crate::semantic_id::EventId::UNKNOWN,
                        "{name}: {} {:#x}",
                        ev.int_class_id,
                        ev.int_event_id
                    );
                }
            }
            for missing in [MissingStyle::Traceprinter, MissingStyle::Marked] {
                for ev in &kev.events {
                    let _ = crate::decode(kev.header.version, ev, &DecodeOptions { missing });
                }
            }
        }
    }
}

#[cfg(test)]
mod coverage {
    use super::*;
    use crate::model::{CombineStatus, DisplayStatus};
    use crate::semantic_id::{ClassId, EventId};
    use std::collections::{BTreeMap, BTreeSet};

    /// def.rs の表から、セマンティック ID の全組み合わせ(USER・QUIP の任意番号を除く)
    fn all_ids() -> BTreeSet<(ClassId, EventId)> {
        crate::qnx800::catalog::all_ids()
            .into_iter()
            .map(|(c, e, _)| (c, e))
            .collect()
    }

    /// args.rs の EVENTS は、セマンティック ID の全組み合わせを 1 行ずつ持つ(重複は spec の組み立てで panic)
    #[test]
    fn args_rows_cover_all_semantic_ids() {
        let all = all_ids();
        let rows: BTreeSet<_> = crate::qnx800::event_decoder::defined_ids().into_iter().collect();
        let missing: Vec<_> = all.difference(&rows).collect();
        let extra: Vec<_> = rows.difference(&all).collect();
        assert!(missing.is_empty(), "args.rs に行が無い: {missing:?}");
        assert!(extra.is_empty(), "def.rs に無い行: {extra:?}");
        assert_eq!(args::EVENTS.iter().filter(|e| e.event.is_some()).count(), all.len());
    }

    /// 全イベント網羅の合成 kev(catalog)に、セマンティック ID の全組み合わせがある
    #[test]
    fn catalog_covers_all_semantic_ids() {
        let csv = scenarios().into_iter().find(|s| s.0 == "catalog").unwrap().1;
        let kev = crate::parse(&super::kev_bytes(&csv)).unwrap();
        let all = all_ids();
        let mut seen: BTreeMap<(ClassId, EventId), bool> = BTreeMap::new(); // 組み合わせ → 表示されたか
        for ev in &kev.events {
            if ev.combine_status == CombineStatus::Complete {
                let d = seen.entry((ev.class_id, ev.event_id)).or_default();
                *d |= ev.display_status == DisplayStatus::Display;
            }
        }
        let missing: Vec<_> = all.iter().filter(|k| !seen.contains_key(k)).collect();
        assert!(missing.is_empty(), "catalog に無い組み合わせ: {missing:?}");
        let displayed = all.iter().filter(|k| seen[k]).count();
        let mut per_class: BTreeMap<u16, (usize, usize)> = BTreeMap::new();
        for k in &all {
            let e = per_class.entry(k.0.0).or_default();
            e.0 += 1;
            e.1 += seen[k] as usize;
        }
        println!(
            "セマンティック ID の組み合わせ {} 種類(表示されるもの {})",
            all.len(),
            displayed
        );
        for (c, (n, d)) in per_class {
            println!("  クラス {c}: {n} 種類(表示 {d})");
        }
        println!("catalog のイベント {} 件", kev.events.len());
    }
}
