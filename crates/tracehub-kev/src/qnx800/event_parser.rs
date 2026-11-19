//! スロットの読み取りからイベントの列まで(設計書 §6)。
//!
//! 1. スロットを読む
//! 2. CPU 別スロット列ごとに reconstruct_timestamp(CONTROL の TIME だけで上位 32 ビットを補う)
//! 3. CONTROL の点検(トレースバッファの通番・スロット数・cookie・DROPPED・途中の別 CPU・外のスロット)
//! 4. merge: CPU 別スロット列の先頭のうち (時刻, CPU 番号) が最小のものを順に取り出す
//! 5. combine: merge した順に組を組み立て、項目を先頭スロットの順に TraceEvent にする
//!
//! 後ろの節に、header の各部の取り出しと、内部 ID ⇔ セマンティック ID の変換がある。

use std::collections::{HashMap, VecDeque};

use super::{args, def, event_decoder};
use crate::model::{CombineStatus, DiagInfo, DiagKind, DisplayStatus, OrphanStructure, TraceEvent, TruncateReason};
use crate::semantic_id::{ClassId, EventId};
use crate::tp_format::Missing;

/// スロットの word の数
pub const SLOT_WORDS_NUM: usize = 4;
const SLOT_BYTES: usize = SLOT_WORDS_NUM * 4;
/// TIME が無いのに下位 32 ビットがこれより大きく減ったら、一周したとみなす(半周)
const WRAP_THRESHOLD: u32 = 0x8000_0000;

/// スロット 1 つ
#[derive(Debug, Clone, Copy)]
struct Slot {
    timestamp_cycles: u64,
    words: [u32; SLOT_WORDS_NUM],
}

impl Slot {
    fn cpu(&self) -> u8 {
        header_bits(self.words[0]).cpu
    }
    fn is_control(&self, event: u16) -> bool {
        let b = header_bits(self.words[0]);
        b.class == def::CLASS_CONTROL && b.event == event && b.structure == def::STRUCT_S
    }
    fn payload(&self, i: usize) -> u32 {
        self.words[2 + i]
    }
}

/// イベント部(syspage の後ろ)を読んで、イベントの列と異常を返す
pub(crate) fn parse(data: &[u8]) -> (Vec<TraceEvent>, Vec<DiagInfo>) {
    let mut diag = Vec::new();
    let mut slots = read_slots(data);
    let tail = data.len() % SLOT_BYTES;
    if tail != 0 {
        diag.push(DiagInfo {
            timestamp_cycles: 0,
            cpu_id: 0,
            kind: DiagKind::TrailingBytes { bytes: tail as u64 },
        });
    }
    let streams = split_cpus(&slots);
    for list in streams.values() {
        reconstruct_timestamp(&mut slots, list, &mut diag);
    }
    check_buffers(&slots, &mut diag);
    let order = merge(&slots, &streams);
    let events = combine(&slots, &order, &mut diag);
    diag.sort_by_key(|d| d.timestamp_cycles);
    (events, diag)
}

/// スロットを読む(リトルエンディアン)
fn read_slots(data: &[u8]) -> Vec<Slot> {
    let rd = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    data.chunks_exact(SLOT_BYTES)
        .map(|c| Slot {
            timestamp_cycles: 0,
            words: [rd(&c[0..4]), rd(&c[4..8]), rd(&c[8..12]), rd(&c[12..16])],
        })
        .collect()
}

/// CPU 別スロット列(スロットの番号をファイルの順に並べたもの)
fn split_cpus(slots: &[Slot]) -> std::collections::BTreeMap<u8, Vec<usize>> {
    let mut m: std::collections::BTreeMap<u8, Vec<usize>> = Default::default();
    for (i, s) in slots.iter().enumerate() {
        m.entry(s.cpu()).or_default().push(i);
    }
    m
}

/// CPU 別スロット列の時刻を補う。上位 32 ビットは TIME から。TIME の無い一周は記録だけして補わない
fn reconstruct_timestamp(slots: &mut [Slot], list: &[usize], diag: &mut Vec<DiagInfo>) {
    let mut msb = 0u64;
    let mut last: Option<u32> = None;
    for &i in list {
        let s = &mut slots[i];
        let low = s.words[1];
        if s.is_control(def::CONTROL_TIME) {
            msb = s.payload(def::TIME_MSB_WORD) as u64;
        } else if let Some(prev) = last
            && low < prev
            && prev - low > WRAP_THRESHOLD
        {
            diag.push(DiagInfo {
                timestamp_cycles: (msb << 32) | low as u64,
                cpu_id: s.cpu(),
                kind: DiagKind::WrapWithoutTime {
                    prev_low32: prev,
                    low32: low,
                },
            });
        }
        last = Some(low);
        s.timestamp_cycles = (msb << 32) | low as u64;
    }
}

/// 開いているトレースバッファ
#[derive(Debug, Clone, Copy)]
struct OpenBuffer {
    seq: u32,
    declared: u32,
    /// TIME から数えたスロット数
    actual: u32,
    last_ts: u64,
    interleaved: bool,
}

#[derive(Debug, Default)]
struct CpuState {
    open: Option<OpenBuffer>,
    last_seq: Option<u32>,
    /// トレースバッファの外にある、直前の TIME(次が BUFFER ならトレースバッファの先頭)
    pending_time: Option<u64>,
    /// 続いているトレースバッファの外のスロット: (最初の時刻, 数)
    outside: Option<(u64, u64)>,
}

/// CONTROL の点検(ファイルの順に、CPU ごとの状態を持って見る)
fn check_buffers(slots: &[Slot], diag: &mut Vec<DiagInfo>) {
    let mut st: HashMap<u8, CpuState> = HashMap::new();
    let push = |diag: &mut Vec<DiagInfo>, ts, cpu, kind| {
        diag.push(DiagInfo {
            timestamp_cycles: ts,
            cpu_id: cpu,
            kind,
        })
    };
    let close_outside = |c: &mut CpuState, cpu: u8, diag: &mut Vec<DiagInfo>| {
        if let Some((ts, n)) = c.outside.take() {
            push(diag, ts, cpu, DiagKind::OutsideBuffer { slots: n });
        }
    };
    let add_outside = |c: &mut CpuState, ts: u64| {
        let o = c.outside.get_or_insert((ts, 0));
        o.1 += 1;
    };
    for s in slots {
        let cpu = s.cpu();
        let ts = s.timestamp_cycles;
        for (&oc, o) in st.iter_mut() {
            if oc != cpu
                && let Some(b) = o.open.as_mut()
                && !b.interleaved
            {
                b.interleaved = true;
                push(
                    diag,
                    b.last_ts,
                    oc,
                    DiagKind::Interleaved {
                        seq: b.seq,
                        other_cpu: cpu,
                    },
                );
            }
        }
        let c = st.entry(cpu).or_default();
        let pending = c.pending_time.take();
        if s.is_control(def::CONTROL_DROPPED) {
            push(
                diag,
                ts,
                cpu,
                DiagKind::Dropped {
                    count: s.payload(def::DROPPED_COUNT_WORD),
                },
            );
        }
        if s.is_control(def::CONTROL_BUFFER) {
            if let Some(b) = c.open.take() {
                push(diag, ts, cpu, DiagKind::MissingBufferEnd { seq: b.seq });
            }
            if pending.is_none() {
                close_outside(c, cpu, diag);
            } else if let Some(o) = c.outside.as_mut() {
                // 直前の TIME はトレースバッファの先頭なので、外のスロットから除く
                o.1 -= 1;
                if o.1 == 0 {
                    c.outside = None;
                }
            }
            close_outside(c, cpu, diag);
            let seq = s.payload(def::BUFFER_SEQ_WORD);
            if let Some(last) = c.last_seq
                && seq != last.wrapping_add(1)
            {
                push(diag, ts, cpu, DiagKind::SeqGap { last_seq: last, seq });
            }
            c.last_seq = Some(seq);
            c.open = Some(OpenBuffer {
                seq,
                declared: s.payload(def::BUFFER_COUNT_WORD),
                actual: if pending.is_some() { 2 } else { 1 },
                last_ts: ts,
                interleaved: false,
            });
        } else if let Some(b) = c.open.as_mut() {
            b.actual += 1;
            b.last_ts = ts;
            if s.is_control(def::CONTROL_BUFFER_END) {
                let values = [s.payload(0), s.payload(1)];
                if values != [def::BUFFER_COOKIE; 2] {
                    push(diag, ts, cpu, DiagKind::CookieMismatch { seq: b.seq, values });
                }
                if b.declared != b.actual {
                    push(
                        diag,
                        ts,
                        cpu,
                        DiagKind::CountMismatch {
                            seq: b.seq,
                            declared: b.declared,
                            actual: b.actual,
                        },
                    );
                }
                c.open = None;
            }
        } else {
            add_outside(c, ts);
            if s.is_control(def::CONTROL_TIME) {
                c.pending_time = Some(ts);
            }
        }
    }
    let mut cpus: Vec<u8> = st.keys().copied().collect();
    cpus.sort_unstable();
    for cpu in cpus {
        let c = st.get_mut(&cpu).unwrap();
        if let Some(b) = c.open.take() {
            push(diag, b.last_ts, cpu, DiagKind::MissingBufferEnd { seq: b.seq });
        }
        close_outside(c, cpu, diag);
    }
}

/// merge: CPU 別スロット列の先頭のうち (時刻, CPU 番号) が最小のものを順に取り出す。CPU の中では並べ替えない
fn merge(slots: &[Slot], streams: &std::collections::BTreeMap<u8, Vec<usize>>) -> Vec<usize> {
    let lists: Vec<&Vec<usize>> = streams.values().collect(); // CPU 番号の順
    let mut pos = vec![0usize; lists.len()];
    let mut out = Vec::with_capacity(slots.len());
    loop {
        let mut best: Option<usize> = None;
        for (k, l) in lists.iter().enumerate() {
            let Some(&i) = l.get(pos[k]) else { continue };
            if best.is_none_or(|b| slots[i].timestamp_cycles < slots[lists[b][pos[b]]].timestamp_cycles) {
                best = Some(k);
            }
        }
        let Some(b) = best else { return out };
        out.push(lists[b][pos[b]]);
        pos[b] += 1;
    }
}

/// 組み立ての項目
#[derive(Debug)]
struct Item {
    cpu: u8,
    flags: u8,
    class: u8,
    event: u16,
    timestamp_cycles: u64,
    /// merge した順のスロットの通し番号(1 から)
    slot_number: u64,
    status: CombineStatus,
    /// 組み立て中(CE 待ち)
    open: bool,
    /// 各スロットの payloads
    payloads: Vec<u32>,
    /// orphan のスロットの構造
    orphan: Option<OrphanStructure>,
    /// 途切れた理由と、途切れた時刻
    truncated: Option<(TruncateReason, u64)>,
}

/// 組の鍵: (CPU, 内部クラス, 内部イベント番号, 時刻の下位 32 ビット)
type Key = (u8, u8, u16, u32);

fn key_of(it: &Item) -> Key {
    (it.cpu, it.class, it.event, it.timestamp_cycles as u32)
}

struct Combiner<'a> {
    q: VecDeque<Item>,
    /// q[0] の通し番号
    base: u64,
    open: HashMap<Key, u64>,
    events: Vec<TraceEvent>,
    diag: &'a mut Vec<DiagInfo>,
    /// orphan を除くイベントの数と、その events の番号
    numbered: Vec<usize>,
    /// ORPHAN 行の位置(orphan を除くイベントの何件目の前か。1 から)の決まっていない orphan の diag の番号
    orphan_pos: Vec<(usize, u64)>,
}

impl Combiner<'_> {
    fn truncate(&mut self, seq: u64, reason: TruncateReason, ts: u64) {
        let it = &mut self.q[(seq - self.base) as usize];
        if it.open {
            it.open = false;
            it.status = CombineStatus::Truncated;
            it.truncated = Some((reason, ts));
            let key = key_of(it);
            self.open.remove(&key);
        }
    }

    /// 先頭の組み立て中の組の後ろに窓の数だけたまっていれば途切れさせる
    fn check_window(&mut self, ts: u64) {
        if self.q.front().is_some_and(|f| f.open) && self.q.len() > args::COMBINE_WINDOW {
            self.truncate(self.base, TruncateReason::Window, ts);
        }
    }

    fn push(&mut self, it: Item, ts: u64) {
        let seq = self.base + self.q.len() as u64;
        if it.open {
            self.open.insert(key_of(&it), seq);
        }
        self.q.push_back(it);
        self.check_window(ts);
    }

    /// 先頭から閉じた項目を TraceEvent にする
    fn drain(&mut self, ts: u64) {
        while self.q.front().is_some_and(|f| !f.open) {
            let it = self.q.pop_front().unwrap();
            self.base += 1;
            self.check_window(ts);
            self.emit(it);
        }
    }

    fn emit(&mut self, it: Item) {
        let index = self.events.len();
        let (class_id, event_id) = semantic_ids(it.class, it.event);
        let mut display = DisplayStatus::Silent;
        match it.status {
            CombineStatus::Orphan => {
                display = DisplayStatus::Display;
                let written = (self.numbered.len() as u64).saturating_sub(args::COMBINE_WINDOW as u64);
                self.orphan_pos.push((self.diag.len(), written + 1));
                self.diag.push(DiagInfo {
                    timestamp_cycles: it.timestamp_cycles,
                    cpu_id: it.cpu,
                    kind: DiagKind::Orphan {
                        event_index: index,
                        slot_number: it.slot_number,
                        structure: it.orphan.unwrap_or(OrphanStructure::CC),
                        display_before: 0,
                    },
                });
            }
            _ => {
                self.numbered.push(index);
                if let Some((reason, ts)) = it.truncated {
                    display = DisplayStatus::Display;
                    self.diag.push(DiagInfo {
                        timestamp_cycles: ts,
                        cpu_id: it.cpu,
                        kind: DiagKind::Truncated {
                            event_index: index,
                            event_number: self.numbered.len() as u64,
                            reason,
                        },
                    });
                }
                // イベントの行 > 定義の無いイベント > TRUNC の行(上で Display) > Silent
                if event_decoder::format(it.class, it.event, &it.payloads, Missing::Omit).is_some() {
                    display = DisplayStatus::Display;
                } else if (class_id == ClassId::UNKNOWN || event_id == EventId::UNKNOWN)
                    && !args::DIAG_UNKNOWN_SILENT.contains(&(it.class, it.event))
                {
                    display = DisplayStatus::Unknown;
                }
            }
        }
        self.events.push(TraceEvent {
            timestamp_cycles: it.timestamp_cycles,
            cpu_id: it.cpu,
            class_id,
            event_id,
            int_class_id: it.class,
            int_event_id: it.event,
            flags: it.flags,
            combine_status: it.status,
            display_status: display,
            payloads: it.payloads,
        });
    }
}

/// traceprinter が UNKNOW 行を出すか(定義の無い内部クラス)
pub(crate) fn unknown_line(class: u8, event: u16) -> bool {
    DEFINED_CLASSES.iter().all(|&c| c != class) && !args::DIAG_UNKNOWN_SILENT.contains(&(class, event))
}

/// combine: merge した順にスロットを処理する
fn combine(slots: &[Slot], order: &[usize], diag: &mut Vec<DiagInfo>) -> Vec<TraceEvent> {
    let mut c = Combiner {
        q: VecDeque::new(),
        base: 0,
        open: HashMap::new(),
        events: Vec::with_capacity(order.len()),
        diag,
        numbered: Vec::new(),
        orphan_pos: Vec::new(),
    };
    let mut last_ts = 0;
    for (n, &i) in order.iter().enumerate() {
        let s = &slots[i];
        let b = header_bits(s.words[0]);
        let ts = s.timestamp_cycles;
        last_ts = ts;
        let item = |status, open, orphan| Item {
            cpu: b.cpu,
            flags: b.flags,
            class: b.class,
            event: b.event,
            timestamp_cycles: ts,
            slot_number: n as u64 + 1,
            status,
            open,
            payloads: vec![s.words[2], s.words[3]],
            orphan,
            truncated: None,
        };
        let key: Key = (b.cpu, b.class, b.event, s.words[1]);
        match b.structure {
            def::STRUCT_S => c.push(item(CombineStatus::Complete, false, None), ts),
            def::STRUCT_CB => {
                if let Some(&old) = c.open.get(&key) {
                    c.truncate(old, TruncateReason::SameKeyCb, ts);
                }
                c.push(item(CombineStatus::Complete, true, None), ts);
            }
            _ => match c.open.get(&key).copied() {
                Some(seq) => {
                    let it = &mut c.q[(seq - c.base) as usize];
                    it.payloads.extend_from_slice(&s.words[2..]);
                    if b.structure == def::STRUCT_CE {
                        it.open = false;
                        c.open.remove(&key);
                    }
                }
                None => {
                    let st = if b.structure == def::STRUCT_CE {
                        OrphanStructure::CE
                    } else {
                        OrphanStructure::CC
                    };
                    c.push(item(CombineStatus::Orphan, false, Some(st)), ts)
                }
            },
        }
        c.drain(ts);
    }
    // 列の終わり: 組み立て中の組はすべて途切れる
    let mut open: Vec<u64> = c.open.values().copied().collect();
    open.sort_unstable();
    for seq in open {
        c.truncate(seq, TruncateReason::EndOfStream, last_ts);
    }
    c.drain(last_ts);

    // ORPHAN 行の位置を events の番号にする
    let Combiner {
        events,
        numbered,
        orphan_pos,
        diag,
        ..
    } = c;
    for (d, k) in orphan_pos {
        let at = numbered.get(k as usize - 1).copied().unwrap_or(events.len());
        if let DiagKind::Orphan { display_before, .. } = &mut diag[d].kind {
            *display_before = at;
        }
    }
    events
}

// ---------------------------------------------------------------- header の各部

/// header の各部
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeaderBits {
    pub structure: u32,
    pub cpu: u8,
    pub flags: u8,
    pub class: u8,
    pub event: u16,
}

pub(crate) fn header_bits(h: u32) -> HeaderBits {
    HeaderBits {
        structure: (h >> def::STRUCT_SHIFT) & def::STRUCT_MASK,
        cpu: ((h >> def::CPU_SHIFT) & def::CPU_MASK) as u8,
        flags: ((h >> def::FLAGS_SHIFT) & def::FLAGS_MASK) as u8,
        class: ((h >> def::CLASS_SHIFT) & def::CLASS_MASK) as u8,
        event: ((h >> def::EVENT_SHIFT) & def::EVENT_MASK) as u16,
    }
}

/// header を組み立てる(header_bits の逆)
pub(crate) fn make_header(b: HeaderBits) -> u32 {
    ((b.structure & def::STRUCT_MASK) << def::STRUCT_SHIFT)
        | ((b.cpu as u32 & def::CPU_MASK) << def::CPU_SHIFT)
        | ((b.flags as u32 & def::FLAGS_MASK) << def::FLAGS_SHIFT)
        | ((b.class as u32 & def::CLASS_MASK) << def::CLASS_SHIFT)
        | ((b.event as u32 & def::EVENT_MASK) << def::EVENT_SHIFT)
}

// ---------------------------------------------------------------- 内部 ID ⇔ セマンティック ID

/// 定義のある内部クラス(これ以外のクラスのイベントには traceprinter が UNKNOW 行を出す)
pub(crate) const DEFINED_CLASSES: &[u8] = &[
    def::CLASS_CONTROL,
    def::CLASS_KER_CALL,
    def::CLASS_INT,
    def::CLASS_PR_TH,
    def::CLASS_SYSTEM,
    def::CLASS_USER,
    def::CLASS_COMM,
    def::CLASS_QUIP,
    def::CLASS_SEC,
    def::CLASS_QVM,
];

/// 内部クラス ID → (内部イベント番号, 名前, セマンティック ID) の表(KER_CALL・USER・QUIP 以外)
pub(crate) fn event_table(class: u8) -> &'static [(u16, &'static str, EventId)] {
    match class {
        def::CLASS_CONTROL => def::CONTROL_EVENTS,
        def::CLASS_INT => def::INT_EVENTS,
        def::CLASS_SYSTEM => def::SYSTEM_EVENTS,
        def::CLASS_COMM => def::COMM_EVENTS,
        def::CLASS_SEC => def::SEC_EVENTS,
        def::CLASS_QVM => def::QVM_EVENTS,
        _ => &[],
    }
}

/// 内部クラス ID → 64 ビット版の (BIT64 を足した内部イベント番号, 名前, セマンティック ID) の表(KER_CALL 以外)
fn event_table_64(class: u8) -> &'static [(u16, &'static str, EventId)] {
    match class {
        def::CLASS_SYSTEM => def::SYSTEM_EVENTS_64,
        _ => &[],
    }
}

/// 内部 ID → セマンティック ID
pub(crate) fn semantic_ids(class: u8, code: u16) -> (ClassId, EventId) {
    let find = |t: &[(u16, &str, EventId)], c: u16| t.iter().find(|r| r.0 == c).map(|r| r.2);
    let unknown = (ClassId::UNKNOWN, EventId::UNKNOWN);
    match class {
        def::CLASS_KER_CALL => {
            let cls = match code & (def::KERCALL_EXIT | def::KERCALL_INT) {
                0 => ClassId::KER_CALL_ENTER,
                def::KERCALL_EXIT => ClassId::KER_CALL_EXIT,
                def::KERCALL_INT => ClassId::KER_CALL_INT,
                _ => return unknown,
            };
            let table = if code & def::BIT64 != 0 {
                def::KERCALLS_64
            } else {
                def::KERCALLS
            };
            (
                cls,
                find(table, code & def::KERCALL_NUM_MASK).unwrap_or(EventId::UNKNOWN),
            )
        }
        def::CLASS_PR_TH => {
            if let Some(e) = find(def::PR_TH_PROCESS, code) {
                (ClassId::PROCESS, e)
            } else {
                let e = find(def::THREAD_STATES, code).or_else(|| find(def::PR_TH_VTHREAD, code));
                (ClassId::THREAD, e.unwrap_or(EventId::UNKNOWN))
            }
        }
        def::CLASS_USER => (ClassId::USER, EventId(code)),
        def::CLASS_QUIP => (ClassId::QUIP, EventId(code)),
        _ => {
            let cls = match class {
                def::CLASS_CONTROL => ClassId::CONTROL,
                def::CLASS_INT => ClassId::INT,
                def::CLASS_SYSTEM => ClassId::SYSTEM,
                def::CLASS_COMM => ClassId::COMM,
                def::CLASS_SEC => ClassId::SEC,
                def::CLASS_QVM => ClassId::QVM,
                _ => return unknown,
            };
            let e = find(event_table(class), code).or_else(|| find(event_table_64(class), code));
            (cls, e.unwrap_or(EventId::UNKNOWN))
        }
    }
}

/// セマンティックなクラス ID → 内部クラス ID(KER_CALL の種別、THREAD と PROCESS は同じ内部クラス)
pub(crate) fn internal_class(class: ClassId) -> Option<u8> {
    Some(match class {
        ClassId::CONTROL => def::CLASS_CONTROL,
        ClassId::KER_CALL_ENTER | ClassId::KER_CALL_EXIT | ClassId::KER_CALL_INT => def::CLASS_KER_CALL,
        ClassId::INT => def::CLASS_INT,
        ClassId::THREAD | ClassId::PROCESS => def::CLASS_PR_TH,
        ClassId::SYSTEM => def::CLASS_SYSTEM,
        ClassId::USER => def::CLASS_USER,
        ClassId::COMM => def::CLASS_COMM,
        ClassId::QUIP => def::CLASS_QUIP,
        ClassId::SEC => def::CLASS_SEC,
        ClassId::QVM => def::CLASS_QVM,
        _ => return None,
    })
}

/// セマンティック ID → 内部 ID(semantic_ids の逆)
pub(crate) fn internal_ids(class: ClassId, event: EventId) -> Option<(u8, u16)> {
    if event == EventId::UNKNOWN {
        return None;
    }
    let find = |t: &[(u16, &str, EventId)]| t.iter().find(|r| r.2 == event).map(|r| r.0);
    match class {
        ClassId::KER_CALL_ENTER | ClassId::KER_CALL_EXIT | ClassId::KER_CALL_INT => {
            let kind = match class {
                ClassId::KER_CALL_EXIT => def::KERCALL_EXIT,
                ClassId::KER_CALL_INT => def::KERCALL_INT,
                _ => 0,
            };
            let num = find(def::KERCALLS).or_else(|| find(def::KERCALLS_64).map(|n| n | def::BIT64))?;
            Some((def::CLASS_KER_CALL, num | kind))
        }
        ClassId::THREAD => Some((
            def::CLASS_PR_TH,
            find(def::THREAD_STATES).or_else(|| find(def::PR_TH_VTHREAD))?,
        )),
        ClassId::PROCESS => Some((def::CLASS_PR_TH, find(def::PR_TH_PROCESS)?)),
        ClassId::USER => Some((def::CLASS_USER, event.0)),
        ClassId::QUIP => Some((def::CLASS_QUIP, event.0)),
        _ => {
            let class = match class {
                ClassId::CONTROL => def::CLASS_CONTROL,
                ClassId::INT => def::CLASS_INT,
                ClassId::SYSTEM => def::CLASS_SYSTEM,
                ClassId::COMM => def::CLASS_COMM,
                ClassId::SEC => def::CLASS_SEC,
                ClassId::QVM => def::CLASS_QVM,
                _ => return None,
            };
            Some((class, find(event_table(class)).or_else(|| find(event_table_64(class)))?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_id;

    fn hw(st: u32, cpu: u32, class: u32, event: u32) -> u32 {
        (st << def::STRUCT_SHIFT) | (cpu << def::CPU_SHIFT) | (class << def::CLASS_SHIFT) | (event << def::EVENT_SHIFT)
    }
    fn bytes(slots: &[[u32; 4]]) -> Vec<u8> {
        slots
            .iter()
            .flat_map(|s| s.iter().flat_map(|w| w.to_le_bytes()))
            .collect()
    }
    const C: u32 = def::CLASS_CONTROL as u32;
    const PR: u32 = def::CLASS_PR_TH as u32;
    const COOKIE: u32 = def::BUFFER_COOKIE;

    #[test]
    fn combine_and_merge() {
        let s = [
            [hw(0, 1, C, 1), 0x100, 1, 0x100],     // CPU1 TIME
            [hw(0, 0, C, 1), 0x100, 1, 0x100],     // CPU0 TIME
            [hw(1, 0, PR, 192), 0x110, 1, 2],      // CB
            [hw(0, 1, PR, 1), 0x105, 1, 6],        // CPU1 の S(時刻が早いので先に出る)
            [hw(3, 0, PR, 192), 0x110, 0x6261, 0], // CE
            [hw(2, 0, PR, 320), 0x120, 1, 2],      // orphan の CC
            [hw(1, 0, PR, 320), 0x130, 1, 2],      // 閉じない CB
        ];
        let (ev, diag) = parse(&bytes(&s));
        let st: Vec<_> = ev
            .iter()
            .map(|e| (e.cpu_id, e.int_event_id, e.combine_status))
            .collect();
        assert_eq!(
            st,
            vec![
                (0, 1, CombineStatus::Complete),
                (1, 1, CombineStatus::Complete),
                (1, 1, CombineStatus::Complete),
                (0, 192, CombineStatus::Complete),
                (0, 320, CombineStatus::Orphan),
                (0, 320, CombineStatus::Truncated),
            ]
        );
        assert_eq!(ev[3].payloads, vec![1, 2, 0x6261, 0]);
        assert_eq!(ev[4].payloads, s[5][2..].to_vec());
        assert_eq!(ev[5].payloads, s[6][2..].to_vec());
        assert_eq!(ev[3].timestamp_cycles, (1 << 32) | 0x110);
        assert!(diag.iter().any(|d| matches!(
            d.kind,
            DiagKind::Orphan {
                slot_number: 6,
                event_index: 4,
                structure: OrphanStructure::CC,
                display_before: 0
            }
        )));
        assert!(diag.iter().any(|d| matches!(
            d.kind,
            DiagKind::Truncated {
                event_index: 5,
                event_number: 5,
                reason: TruncateReason::EndOfStream
            }
        )));
    }

    #[test]
    fn buffer_checks() {
        let s = [
            [hw(0, 0, C, 1), 0x100, 0, 0x100],
            [hw(0, 0, C, 2), 0x100, 1, 9], // スロット数の食い違い(実際は 4)
            [hw(0, 0, PR, 1), 0x110, 1, 6],
            [hw(0, 0, C, 4), 0x120, COOKIE, 1], // cookie の誤り
            [hw(0, 0, C, 1), 0x200, 0, 0x200],
            [hw(0, 0, C, 2), 0x200, 3, 5],  // 通番の飛び
            [hw(0, 1, PR, 1), 0x210, 1, 6], // 途中に別の CPU
            [hw(0, 0, C, 8), 0x220, 25, 0], // DROPPED
            [hw(0, 0, PR, 1), 0x230, 1, 6],
            // BUFFER_END が無い
        ];
        let (_, diag) = parse(&bytes(&s));
        let kinds: Vec<_> = diag.iter().map(|d| d.kind.clone()).collect();
        for k in [
            DiagKind::CountMismatch {
                seq: 1,
                declared: 9,
                actual: 4,
            },
            DiagKind::CookieMismatch {
                seq: 1,
                values: [COOKIE, 1],
            },
            DiagKind::SeqGap { last_seq: 1, seq: 3 },
            DiagKind::Interleaved { seq: 3, other_cpu: 1 },
            DiagKind::Dropped { count: 25 },
            DiagKind::MissingBufferEnd { seq: 3 },
            DiagKind::OutsideBuffer { slots: 1 },
        ] {
            assert!(kinds.contains(&k), "{k:?} not found: {kinds:?}");
        }
    }

    #[test]
    fn window_truncates() {
        let mut s = vec![[hw(1, 0, PR, 192), 0x100, 1, 2]];
        for i in 0..args::COMBINE_WINDOW as u32 {
            s.push([hw(0, 0, PR, 1), 0x101 + i, 1, 6]);
        }
        s.push([hw(3, 0, PR, 192), 0x100, 0, 0]);
        let (ev, _) = parse(&bytes(&s));
        assert_eq!(ev[0].combine_status, CombineStatus::Truncated);
        assert_eq!(ev.last().unwrap().combine_status, CombineStatus::Orphan);
    }

    #[test]
    fn semantic_round_trip() {
        let cases = [
            (
                def::CLASS_KER_CALL,
                11,
                ClassId::KER_CALL_ENTER,
                semantic_id::kercall::MSG_SENDV,
            ),
            (
                def::CLASS_KER_CALL,
                11 | def::KERCALL_EXIT | def::BIT64,
                ClassId::KER_CALL_EXIT,
                semantic_id::kercall::MSG_SENDV_64,
            ),
            (def::CLASS_PR_TH, 1, ClassId::THREAD, semantic_id::thread::RUNNING),
            (
                def::CLASS_PR_TH,
                192,
                ClassId::PROCESS,
                semantic_id::process::CREATE_P_NAME,
            ),
            (def::CLASS_SYSTEM, 12, ClassId::SYSTEM, semantic_id::system::SLOG),
            (
                def::CLASS_SYSTEM,
                16 | def::BIT64,
                ClassId::SYSTEM,
                semantic_id::system::IPI_64,
            ),
            (def::CLASS_USER, 77, ClassId::USER, EventId(77)),
        ];
        for (c, e, sc, se) in cases {
            assert_eq!(semantic_ids(c, e), (sc, se), "{c} {e}");
            assert_eq!(internal_ids(sc, se), Some((c, e)));
        }
        assert_eq!(semantic_ids(0, 0), (ClassId::UNKNOWN, EventId::UNKNOWN));
        // KER_CALL は全コールに 64 ビット版がある。定義の無いコール番号は UNKNOWN
        assert_eq!(
            semantic_ids(def::CLASS_KER_CALL, def::BIT64),
            (ClassId::KER_CALL_ENTER, semantic_id::kercall::NOP_64)
        );
        assert_eq!(
            semantic_ids(def::CLASS_KER_CALL, 120 | def::BIT64),
            (ClassId::KER_CALL_ENTER, EventId::UNKNOWN)
        );
        // SYSTEM の 64 ビット版の一覧に無いもの(SLOG の 64 ビット版)は UNKNOWN
        assert_eq!(
            semantic_ids(def::CLASS_SYSTEM, 12 | def::BIT64),
            (ClassId::SYSTEM, EventId::UNKNOWN)
        );
        assert_eq!(semantic_ids(def::CLASS_SYSTEM, 1000).1, EventId::UNKNOWN);
        assert_eq!(
            semantic_ids(def::CLASS_KER_CALL, def::KERCALL_EXIT | def::KERCALL_INT).0,
            ClassId::UNKNOWN
        );
    }
}
