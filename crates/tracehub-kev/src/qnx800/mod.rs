//! QNX 8.0 に依存する部分。
//!
//! - def.rs: 内部 ID の定義(生成ファイル)
//! - args.rs: traceprinter の表示の定義(手で保守する)
//! - event_parser.rs: スロットの読み取りからイベントの列まで
//! - event_decoder.rs: イベントの表示文字列
//! - slog2.rs: SLOG2(SLOG の payloads の slog2 のレコード)の表示文字列
//! - traceprinter.rs: traceprinter 互換出力

pub mod args;
#[doc(hidden)]
pub mod catalog;
#[rustfmt::skip] // 生成ファイル(tests/kev/tools/gen_qnx_def.py の出力のまま)
pub mod def;
pub(crate) mod event_decoder;
pub(crate) mod event_parser;
#[doc(hidden)]
pub mod scenarios;
pub(crate) mod slog2;
pub(crate) mod traceprinter;

use crate::semantic_id::{ClassId, EventId};

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

/// セマンティックなクラス ID → traceprinter の表示クラス名
pub(crate) fn class_display(class: ClassId) -> Option<&'static str> {
    args::CLASS_DISPLAY.iter().find(|r| r.0 == class).map(|r| r.1)
}

/// 内部クラス ID → (内部イベント番号, 名前, セマンティック ID) の表(KER_CALL・USER・QUIP 以外)
fn event_table(class: u8) -> &'static [(u16, &'static str, EventId)] {
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
