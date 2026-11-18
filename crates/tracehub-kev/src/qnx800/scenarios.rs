//! 試験用の合成データ(kevcsv の CSV)を作る。traceprinter との比較に使う。
//!
//! | 場面 | 内容 |
//! |---|---|
//! | catalog | args.rs と def.rs の全イベント(定義の無い番号・クラスを含む)。payloads の語数・文字列・値の名前を変えた変種 |
//! | short_words | 語が足りないイベント(S・CB だけ・CB CC で途切れた組) |
//! | combine_window | 組み立て中の組の後ろにたまる項目の数の境目(窓) |
//! | orphan_lookahead | 多数のイベントの後の orphan(ORPHAN 行の位置) |
//! | sched_wrap | 4 CPU・下位 32 ビットの一周(TIME あり)・CPU 間の同時刻 |
//! | sched_wrap_inferred | 同じ列で、一周のときの TIME が無い |

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use super::{args, class_code, def, event_code};
use crate::tp_format::{Spec, Usage};

/// 試験用の場面: (名前, CSV)
pub fn scenarios() -> Vec<(&'static str, String)> {
    let mut out = vec![
        ("catalog", to_csv(1, &pack(&catalog_events(), MAX_BUFFER_SLOTS, true))),
        ("short_words", to_csv(1, &short_words())),
        ("combine_window", to_csv(2, &combine_window())),
        ("orphan_lookahead", to_csv(1, &orphan_lookahead())),
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
        let c = class_code(e.class).unwrap();
        let u = usage(e.fast, e.wide, e.min_words as usize);
        if e.event == "*" {
            wild.insert(c, u);
        } else {
            ev.insert((c, event_code(c, e.event, e.kind, e.is64).unwrap()), u);
        }
    }
    (ev, wild)
}

fn thread_usage(code: u16) -> Usage {
    let mut u = Usage::default();
    let name = def::THREAD_STATES.iter().find(|r| r.0 == code).map(|r| r.1);
    match args::THREAD_OVERRIDES.iter().find(|r| Some(r.0) == name) {
        Some((_, items)) => u.add(items),
        None => u.add(args::THREAD_BODY),
    }
    u
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
    // スレッドの状態(全番号)
    for code in 0..def::PR_TH_PROCESS_BASE {
        for w in variants(&thread_usage(code)) {
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

fn code(class: &str, event: &str) -> (u8, u16) {
    let c = class_code(class).unwrap();
    (c, event_code(c, event, "", false).unwrap())
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
    let (ev, _) = usages();
    let (pr_th, running) = code("PR_TH", "RUNNING");
    let g = |v: &[u32], i: usize| v.get(i).copied().unwrap_or(0);
    // (構造, クラス, イベント番号, 時刻の増分, payloads)
    type Part = (u32, u8, u16, u64, [u32; 2]);
    let mut groups: Vec<Vec<Part>> = Vec::new();
    for (&(c, code), v) in &longest {
        if !super::DEFINED_CLASSES.contains(&c) {
            continue;
        }
        let u = if c == def::CLASS_PR_TH && code < def::PR_TH_PROCESS_BASE {
            thread_usage(code)
        } else {
            match ev.get(&(c, code)) {
                Some(u) => u.clone(),
                None => continue,
            }
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
    let (pr_th, name_t) = code("PR_TH", "NAME_T");
    let (_, running) = code("PR_TH", "RUNNING");
    let ker = def::CLASS_KER_CALL;
    let sendv = event_code(ker, "MSG_SENDV", "ENTER", false).unwrap();
    let ab = u32::from_le_bytes(*b"ab  ");
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
    let (pr_th, name_t) = code("PR_TH", "NAME_T");
    let (_, running) = code("PR_TH", "RUNNING");
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

/// 複数 CPU・下位 32 ビットの一周・CPU 間の同時刻を含む、スケジューリングのような列
fn sched_events(cpus: u8, per_cpu: usize) -> Vec<SynEvent> {
    let (pr_th, running) = code("PR_TH", "RUNNING");
    let (_, ready) = code("PR_TH", "READY");
    let ker = def::CLASS_KER_CALL;
    let yield_num = event_code(ker, "SCHED_YIELD", "ENTER", false).unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DecodeOptions, DiagKind, MissingStyle};

    /// CSV(この生成器の形)を kev のバイト列にする
    fn kev_bytes(csv: &str) -> Vec<u8> {
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
            for missing in [MissingStyle::Traceprinter, MissingStyle::Marked] {
                for ev in &kev.events {
                    let _ = crate::decode(kev.header.version, ev, &DecodeOptions { missing });
                }
            }
        }
    }
}
