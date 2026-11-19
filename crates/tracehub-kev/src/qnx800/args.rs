//! QNX 8.0 の traceprinter(1.02)の表示の定義。
//!
//! traceprinter の出力とバイト単位で同じにするための、表示名・引数の書式・出力しない条件。
//! 手で保守する。イベントはセマンティック ID で指す。EVENTS はセマンティック ID のすべてを 1 行ずつ持つ(試験で確かめる)。
//! 書式の要素とテンプレートの書き方は crate::tp_format を参照。

use crate::semantic_id::{ClassId, comm, control, int, kercall, process, qvm, sec, system, thread};
use crate::tp_format::*;

// ---------------------------------------------------------------- 出力の枠

pub const LINE_END: &str = "\r\n";
pub const BANNER: &[&str] = &["TRACEPRINTER version 1.02", "TRACEPARSER LIBRARY version 1.02"];
pub const HEADER_TITLE: &str = " -- HEADER FILE INFORMATION -- ";
pub const EVENTS_TITLE: &str = " -- KERNEL EVENTS -- ";
/// ヘッダの行に書かないキー
pub const HEADER_SKIP: &[&str] = &["TRACE_HEADER_BEGIN", "TRACE_HEADER_END"];
/// ヘッダの行: "%22s:: %s"
pub const HEADER_KEY_WIDTH: usize = 22;
/// イベントの行: "t:0x%08x CPU:%02d %-16s:%-18s<args>"
pub const CLASS_WIDTH: usize = 16;
pub const NAME_WIDTH: usize = 18;

// ---------------------------------------------------------------- 組み立て

/// 組が先頭にあるまま後ろにこの数だけ項目がたまったら途切れさせる。ORPHAN 行もこの数だけ前に出る。
/// ヘッダに定義は無く、traceprinter の振る舞いから決めた値
pub const COMBINE_WINDOW: usize = 1022;

// ---------------------------------------------------------------- 診断の行
// {n} 番号、{cpu} CPU(2 桁)、{class} 内部クラスの名前、{ktype} KER_CALL の種別、{func} イベント番号(KER_CALL はコール番号)、
// {kind} 構造の名前、{raw} スロットの 4 words、{raw2} その先頭 2 words(16 進 8 桁)

/// 途切れた組(イベントの行の前。n = orphan を除くイベントの通し番号)
pub const DIAG_TRUNC: &str =
    "TRACEPARSER: do_entry.1382 e:{n}  TRUNC CB CPU{cpu} class:{class}{ktype} func:{func} {raw}";
/// 組み立て中の組が無い CC / CE(n = merge した順のスロットの通し番号)
pub const DIAG_ORPHAN: &str =
    "TRACEPARSER: handle_entry.1483 e:{n} ORPHAN {kind} CPU{cpu} class:{class}{ktype} func:{func} {raw}";
/// 定義の無いクラスのイベント(イベントの行の前)
pub const DIAG_UNKNOWN: &str = "TRACEPARSER: decode.1016 UNKNOW CPU{cpu} class:{class} func:{func} {raw2}";
/// DIAG_CLASS_NAMES に無い内部クラスの名前
pub const DIAG_UNKNOWN_CLASS: &str = "UNKNOWN({num})";
/// UNKNOW 行を出さない (内部クラス, 内部イベント番号)。クラス 0・イベント 0 は空のスロット
pub const DIAG_UNKNOWN_SILENT: &[(u8, u16)] = &[(0, 0)];
pub const DIAG_KERCALL_TYPE: &str = " type:{type}";
pub const DIAG_KERCALL_TYPE_UNKNOWN: &str = "unknown";
/// KER_CALL の種別(exit のビットの位置からの 2 ビット)の名前
pub const DIAG_KERCALL_TYPES: &[(u16, &str)] = &[(0, "entry"), (1, "exit"), (2, "int")];
/// 内部クラスの名前(traceparser の名前)
pub const DIAG_CLASS_NAMES: &[(u8, &str)] = &[
    (0, "_TRACE_EMPTY_C"),
    (1, "_TRACE_CONTROL_C"),
    (2, "_TRACE_KER_CALL_C"),
    (3, "_TRACE_INT_C"),
    (4, "_TRACE_PR_TH_C"),
    (5, "_TRACE_CONTAINER_C"),
    (6, "_TRACE_USER_C"),
    (7, "_TRACE_COMM_C"),
    (8, "_TRACE_QUIP_C"),
    (9, "_TRACE_SEC_C"),
    (10, "_TRACE_QVM_C"),
];
/// 構造の名前
pub const DIAG_STRUCT_NAMES: &[(u32, &str)] = &[(0, "S"), (1, "CB"), (2, "CC"), (3, "CE")];

// ---------------------------------------------------------------- 表示クラス名
/// セマンティックなクラス ID → 表示クラス名
pub const CLASS_DISPLAY: &[(ClassId, &str)] = &[
    (ClassId::CONTROL, "CONTROL"),
    (ClassId::KER_CALL_ENTER, "KER_CALL"),
    (ClassId::KER_CALL_EXIT, "KER_EXIT"),
    (ClassId::KER_CALL_INT, "KER_INT"),
    (ClassId::INT, "INTERRUPT"),
    (ClassId::THREAD, "THREAD"),
    (ClassId::PROCESS, "PROCESS"),
    (ClassId::SYSTEM, "SYSTEM"),
    (ClassId::USER, "USREVENT"),
    (ClassId::COMM, "COMM"),
    (ClassId::QUIP, "QUIP"),
    (ClassId::SEC, "SEC"),
    (ClassId::QVM, "QVM"),
];
/// KER_CALL を INT_CALL の形(Ev::intcall)で表示するときの表示クラス名
pub const DISPLAY_INT_CALL: &str = "INT_CALL";

// ---------------------------------------------------------------- fast の書式が無いとき
/// true: wide の書式で表示する、false: 表示しない
pub const FAST_FALLBACK_WIDE: &[(ClassId, bool)] = &[
    (ClassId::KER_CALL_ENTER, false),
    (ClassId::KER_CALL_EXIT, false),
    (ClassId::KER_CALL_INT, false),
];
pub const FAST_FALLBACK_WIDE_DEFAULT: bool = true;

// ---------------------------------------------------------------- 表示イベント名
// EVENTS の name が無ければ、def.rs の名前(64 ビット版も 64 ビットでない版の名前)。KER_CALL とスレッドの状態は下の規則。

// ---------------------------------------------------------------- カーネルコール
pub const KERCALL_NAME: &str = "{call}/{num:02}";
pub const KERCALL_UNKNOWN_CALL: &str = "KER_{num:02}";
/// INT_CALL の形(引数なし)
pub const INTCALL_NAME: &str = "KER_{call}/{num}";
pub const INTCALL_UNKNOWN_CALL: &str = "{num:02}";
/// 失敗した exit(表示する exit に限る)
pub const KERCALL_FAILURE_WHEN: &str = "u0 == 0xffffffff && words >= 2";
pub const KERCALL_FAILURE: &[Item] = &[t(" ret_val:-1, errno:{u1}")];
/// コール名の表示の違い(enter / exit 共通)
pub const KERCALL_NAMES: &[(&str, &str)] = &[("SIGNAL_KILL_SIGVAL", "SIGNAL_KILL")];
/// exit だけの表示の違い
pub const KERCALL_NAMES_EXIT: &[(&str, &str)] = &[("SYNC_CONDVAR_SIGNAL", "SYNC_CONDVAR_SIG")];

// ---------------------------------------------------------------- スレッドの状態(PR_TH の PR_TH_PROCESS_BASE より小さい番号)
pub const THREAD_NAME: &str = "TH{name}";
/// 定義の無い状態の番号は、これ以下なら表示する(EVENTS の any(ClassId::THREAD))
pub const THREAD_MAX_PRINTED: u16 = 29;
pub const THREAD_BODY: &[Item] = &[
    t(" pid:{d0} tid:{d1} "),
    t("priority:{d2} policy:{d3}").when("words >= 4"),
    t(" partition:{d4}").when("words >= 5"),
    t(" sched_flags:0x{x5:08}").when("words >= 6"),
];

// ---------------------------------------------------------------- 値の名前の表
pub const TABLE_IPI_CMD: Table = &[
    (0x1, "RESCHED"),
    (0x2, "TLB_FLUSH"),
    (0x4, "SET_SAFE_ASPACE"),
    (0x8, "RESCHED_PREEMPT"),
    (0x10, "ACK"),
    (0x20, "HALT"),
    (0x40, "HYP_VMEXIT"),
    (0x80, "INPROGRESS"),
    (0x100, "TRACE_START"),
    (0x200, "TRACE_STOP"),
    (0x400, "TRACE_FLUSH"),
    (0x800, "CLOCK_RUN_IST_ASAP"),
    (0x1000, "CLOCK_RUN_IST_ON_NEXT_TICK"),
];
pub const TABLE_TIMER_FLAGS: Table = &[
    (0x80, "TOLERANT"),
    (0x1, "ACTIVE"),
    (0x2, "ABSOLUTE"),
    (0x4, "EXPIRED"),
    (0x8, "TOD_BASED"),
    (0x10, "TARGET_PROCESS"),
    (0x40, "PRECISE"),
    (0x100, "WAKEUP"),
];
pub const TABLE_SEC_PATH_STATUS: Table = &[(0x0, "OK"), (0x1, "NO_RULE"), (0x2, "CONFLICT"), (0x3, "BAD_OP")];
pub const TABLE_SEC_ABLE: Table = &[
    (0x0, "SPAWN_SETUID"),
    (0x1, "SPAWN_SETGID"),
    (0x2, "SETUID"),
    (0x3, "SETGID"),
    (0x4, "GETID"),
    (0x5, "PATHSPACE"),
    (0x6, "REBOOT"),
    (0x7, "CPUMODE"),
    (0x8, "RUNSTATE"),
    (0x9, "CONFSET"),
    (0xa, "RSRCDBMGR"),
    (0xb, "SESSION"),
    (0xc, "UMASK"),
    (0xd, "EVENT"),
    (0xe, "RLIMIT"),
    (0xf, "MEM_ADD"),
    (0x10, "MEM_PHYS"),
    (0x11, "MEM_SPECIAL"),
    (0x12, "MEM_GLOBAL"),
    (0x13, "MEM_PEER"),
    (0x14, "MEM_LOCK"),
    (0x15, "SPAWN"),
    (0x16, "FORK"),
    (0x17, "PROT_EXEC"),
    (0x18, "WAIT"),
    (0x19, "(null)"),
    (0x1a, "MSG_QUEUE"),
    (0x1b, "CLOCKSET"),
    (0x1c, "(null)"),
    (0x1d, "INTERRUPT"),
    (0x1e, "(null)"),
    (0x1f, "IO"),
    (0x20, "TRACE"),
    (0x21, "PRIORITY"),
    (0x22, "CONNECTION"),
    (0x23, "SCHEDULE"),
    (0x24, "SIGNAL"),
    (0x25, "TIMER"),
    (0x26, "PGRP"),
    (0x27, "MAP_FIXED"),
    (0x28, "PATH_TRUST"),
    (0x29, "SWAP"),
    (0x2a, "RCONSTRAINT"),
    (0x2b, "CHILD_NEWAPP"),
    (0x2c, "PUBLIC_CHANNEL"),
    (0x2d, "APS_ROOT"),
    (0x2e, "ABLE_CREATE"),
    (0x2f, "RUNSTATE_BURST"),
    (0x30, "DEFAULT_TIMER_TOLERANCE"),
    (0x31, "(null)"),
    (0x32, "XPROCESS_QUERY"),
    (0x33, "CHROOT"),
    (0x34, "POWER"),
    (0x35, "PROT_WRITE_AND_EXEC"),
    (0x36, "SRANDOM"),
    (0x37, "(null)"),
    (0x38, "HYP"),
    (0x39, "RLIMIT_PEER"),
    (0x3a, "CHANNEL_CONNECT"),
    (0x3b, "MAC_POLICY"),
    (0x3c, "SETTYPEID"),
    (0x3d, "ABLE_PRIV"),
    (0x3e, "XPROCESS_ABLE"),
    (0x3f, "(null)"),
    (0x40, "XPROCESS_DEBUG"),
    (0x41, "XPROCESS_MEM_READ"),
    (0x42, "HIGH_RESOLUTION_TIMER"),
    (0x43, "UNTRUSTED_EXEC"),
    (0x44, "SERVER_MONITOR"),
    (0x45, "SIGEV_THREAD"),
    (0x46, "PRIVREG"),
    (0x47, "MOUNTIFS"),
];
pub const TABLES: &[(&str, Table)] = &[
    ("ipi_cmd", TABLE_IPI_CMD),
    ("timer_flags", TABLE_TIMER_FLAGS),
    ("sec_path_status", TABLE_SEC_PATH_STATUS),
    ("sec_able", TABLE_SEC_ABLE),
];

// ---------------------------------------------------------------- SLOG2
// SLOG の payloads が slog2 のレコードなら、traceprinter は SLOG2 という名前でレコードの文字列を出す。
// 値はすべて traceprinter の振る舞いから決めた(ヘッダに定義は無い)。オフセットはレコード(payloads)の先頭からのバイト位置。
// レコード: [0..2] SLOG2_MAGIC、[2..4] レコード長、[7] 種別、[20..] 本文。
// 種別が書式なら、本文は NUL で終わる printf の書式、その後ろに引数(2 バイト揃えの u16 のヘッダ + データ)が並ぶ。

/// SLOG2 として読むイベント
pub const SLOG2_EVENT: (ClassId, crate::semantic_id::EventId) = (ClassId::SYSTEM, system::SLOG);
/// 表示イベント名
pub const SLOG2_NAME: &str = "SLOG2";
/// レコードの先頭(リトルエンディアンの u16)。違えば SLOG の書式
pub const SLOG2_MAGIC: u16 = 0xcfbd;
/// レコード長(リトルエンディアンの u16、バイト単位)の位置
pub const SLOG2_LEN_OFFSET: usize = 2;
/// 種別(1 バイト)の位置
pub const SLOG2_TYPE_OFFSET: usize = 7;
/// 本文の位置。payloads がこれより短ければ SLOG の書式、レコード長がこれ以下なら空
pub const SLOG2_BODY_OFFSET: usize = 20;
/// レコード長の上限。超えるか payloads より長ければ空
pub const SLOG2_MAX_RECORD: usize = 536;
/// 種別: 文字列(本文のレコード長 - 1 バイトまでを NUL の手前まで出す)。それ以外の種別は空
pub const SLOG2_TYPE_STRING: u8 = 0;
/// 種別: 書式と引数
pub const SLOG2_TYPE_FORMAT: u8 = 1;
/// 種別が書式のときの、文字列の前置き
pub const SLOG2_FORMAT_PREFIX: &str = "FA ";
/// 書式の長さの上限(NUL を除く)。超えるか NUL がレコードに無ければ前置きだけ
pub const SLOG2_MAX_FORMAT: usize = 511;
/// 書式を当てはめた文字列の長さの上限(超えた分は切る)
pub const SLOG2_MAX_OUTPUT: usize = 1023;
/// 引数のヘッダの揃え(レコードの先頭から)
pub const SLOG2_ARG_ALIGN: usize = 2;
/// 引数のヘッダの、データが文字列であることを表すビット
pub const SLOG2_ARG_STRING: u16 = 0x8000;
/// 引数のヘッダの、データのバイト数のマスク
pub const SLOG2_ARG_LEN_MASK: u16 = 0x7fff;
/// 数として読めるデータのバイト数(リトルエンディアン)。ほかの長さは書式を当てはめられない
pub const SLOG2_INT_SIZES: &[usize] = &[1, 2, 4, 8];
/// 整数の変換の長さ修飾子とビット数(修飾子が無いときと L は SLOG2_INT_BITS)。traceprinter は Windows の
/// 実行ファイルなので long は 32 ビット
pub const SLOG2_LENGTHS: &[(&str, u32)] = &[
    ("hh", 8),
    ("h", 16),
    ("ll", 64),
    ("l", 32),
    ("j", 64),
    ("z", 64),
    ("t", 64),
    ("L", 32),
];
pub const SLOG2_INT_BITS: u32 = 32;
/// %s・%c をワイド文字(UTF-16)にする長さ修飾子。1 バイトにならない文字(0xff より大きい)は、
/// %s ではそこで終わり、%c では空になる(traceprinter の C ロケールの変換)
pub const SLOG2_WIDE_LENGTHS: &[&str] = &["l", "ll", "j", "z", "t"];
/// %p(フラグ・幅・精度・長さ修飾子の無いもの)の桁数(0 で埋めた 16 進)
pub const SLOG2_POINTER_DIGITS: usize = 16;

// ---------------------------------------------------------------- 後ろに空行を入れるイベント
pub const BLANK_AFTER: &[(ClassId, crate::semantic_id::EventId)] = &[
    (ClassId::CONTROL, control::BUFFER),
    (ClassId::CONTROL, control::BUFFER_END),
    (ClassId::CONTROL, control::DROPPED),
];

// ---------------------------------------------------------------- イベントごとの書式
// セマンティック ID ごとに 1 行。args: fast と wide に同じ書式、fast / wide: それぞれの書式、*_silent: 出力しない、
// 書式の無い行: traceprinter は出力しない(fast が無ければ wide を使うかは FAST_FALLBACK_WIDE)。
// intcall: 引数の無い INT_CALL の形。any: そのクラスで定義の無い番号(max_code があればその番号まで)。

pub const EVENTS: &[Ev] = &[
    // ---- CONTROL
    ev(ClassId::CONTROL, control::TIME).args(&[h("msb", 0), h("lsb(offset)", 1)]),
    ev(ClassId::CONTROL, control::BUFFER).args(&[t(" sequence = {u0}, num_events = {u1}")]),
    ev(ClassId::CONTROL, control::BUFFER_END).args(&[t(" cookie1 = {x0}, cookie2 = {x1}")]),
    ev(ClassId::CONTROL, control::DROPPED).args(&[t(" dropped_events = {u0}")]),
    // ---- KER_CALL(コールごとに enter・exit・int、それぞれ 64 ビット版)
    ev(ClassId::KER_CALL_ENTER, kercall::NOP)
        .fast(&[h("dummy", 0), h("empty", 1)])
        .wide(&[h("dummy", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::NOP_64).fast_silent().wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::NOP)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::NOP_64).fast_silent().wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::NOP).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::NOP_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TRACE_EVENT)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::TRACE_EVENT_64)
        .fast(&[q("cmd", 0)])
        .wide(&[
            q("cmd", 0),
            q("param1", 2),
            q("param2", 4),
            q("param3", 6),
            q("param4", 8),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::TRACE_EVENT)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::TRACE_EVENT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TRACE_EVENT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::TRACE_EVENT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::RING0)
        .fast(&[h("func_p", 0), h("arg_p", 1)])
        .wide(&[h("func_p", 0), h("arg_p", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::RING0_64)
        .fast(&[q("func_p", 0)])
        .wide(&[q("func_p", 0)]),
    ev(ClassId::KER_CALL_EXIT, kercall::RING0)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::RING0_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::RING0).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::RING0_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CACHE_CONTROL)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::CACHE_CONTROL_64)
        .fast(&[q("addr", 0)])
        .wide(&[q("addr", 0), q("len", 2), h("flags", 4)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CACHE_CONTROL)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CACHE_CONTROL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CACHE_CONTROL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CACHE_CONTROL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_SRANDOM)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_SRANDOM_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_SRANDOM)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_SRANDOM_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYS_SRANDOM).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYS_SRANDOM_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_REGISTER_EVENT)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_REGISTER_EVENT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_REGISTER_EVENT)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_REGISTER_EVENT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_REGISTER_EVENT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_REGISTER_EVENT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SEND_PULSEPTR)
        .fast(&[h("coid", 0), h("code", 1)])
        .wide(&[h("coid", 0), d("priority", 1), h("code", 2), h("value", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SEND_PULSEPTR_64)
        .fast(&[h("coid", 0), h("code", 1)])
        .wide(&[h("coid", 0), d("priority", 1), h("code", 2), q("value", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SEND_PULSEPTR)
        .fast(&[d("status", 0), h("empty", 1)])
        .wide(&[d("status", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SEND_PULSEPTR_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SEND_PULSEPTR).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SEND_PULSEPTR_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_CPUPAGE_GET)
        .fast(&[d("index", 0), h("empty", 1)])
        .wide(&[d("index", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_CPUPAGE_GET_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_CPUPAGE_GET)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_CPUPAGE_GET_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYS_CPUPAGE_GET).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYS_CPUPAGE_GET_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_CPUPAGE_SET)
        .fast(&[d("index", 0), h("value", 1)])
        .wide(&[d("index", 0), h("value", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_CPUPAGE_SET_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_CPUPAGE_SET)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_CPUPAGE_SET_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYS_CPUPAGE_SET).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYS_CPUPAGE_SET_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_PAUSE)
        .fast(&[h("rcvid", 0), h("cookie", 1)])
        .wide(&[h("rcvid", 0), h("cookie", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_PAUSE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_PAUSE).intcall().fast(&[]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_PAUSE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_PAUSE).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_PAUSE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_CURRENT)
        .fast(&[h("rcvid", 0), h("empty", 1)])
        .wide(&[h("rcvid", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_CURRENT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_CURRENT)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_CURRENT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_CURRENT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_CURRENT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SENDV)
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), d("sparts", 1), d("rparts", 2), s("msg[0]", 3, 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SENDV_64)
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), e("sparts", 1), e("rparts", 3), s("msg[0]", 5, 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SENDV)
        .fast(&[d("status", 0), s("rmsg[0]", 1, 1)])
        .wide(&[d("status", 0), s("rmsg[0]", 1, 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SENDV_64)
        .fast(&[e("status", 0)])
        .wide(&[e("status", 0), s("rmsg[0]", 2, 3)]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SENDV).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SENDV_64).wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SENDVNC)
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), d("sparts", 1), d("rparts", 2), s("msg[0]", 3, 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SENDVNC_64)
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), e("sparts", 1), e("rparts", 3), s("msg[0]", 5, 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SENDVNC)
        .fast(&[h("ret_val", 0), s("rmsg[0]", 1, 1)])
        .wide(&[h("ret_val", 0), s("rmsg[0]", 1, 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SENDVNC_64)
        .fast(&[q("ret_val", 0)])
        .wide(&[q("ret_val", 0), s("rmsg[0]", 2, 3)]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SENDVNC).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SENDVNC_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_ERROR)
        .fast(&[h("rcvid", 0), d("err", 1)])
        .wide(&[h("rcvid", 0), d("err", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_ERROR_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_ERROR)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_ERROR_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_ERROR).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_ERROR_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_RECEIVEV)
        .fast(&[h("chid", 0), d("rparts", 1)])
        .wide(&[h("chid", 0), d("rparts", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_RECEIVEV_64)
        .fast(&[])
        .wide(&[h("chid", 0), e("rparts", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_RECEIVEV)
        .fast(&[h("rcvid", 0), s("rmsg[0]", 1, 1)])
        .wide(&[
            h("rcvid", 0),
            s("rmsg[0]", 1, 3),
            d("info->nd", 4),
            d("info->srcnd", 5),
            d("info->pid", 6),
            d("info->tid", 7),
            d("info->chid", 8),
            d("info->scoid", 9),
            d("info->coid", 10),
            d("info->msglen", 11),
            d("info->srcmsglen", 12),
            d("info->dstmsglen", 13),
            d("info->priority", 14),
            d("info->flags", 15),
            d("info->reserved", 16),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_RECEIVEV_64)
        .fast(&[h("rcvid", 0), s("rmsg[0]", 1, 1)])
        .wide(&[
            h("rcvid", 0),
            s("rmsg[0]", 1, 3),
            d("info->nd", 4),
            d("info->srcnd", 5),
            d("info->pid", 6),
            d("info->tid", 7),
            d("info->chid", 8),
            d("info->scoid", 9),
            d("info->coid", 10),
            e("info->msglen", 11),
            e("info->srcmsglen", 13),
            e("info->dstmsglen", 15),
            d("info->priority", 17),
            d("info->flags", 18),
            d("info->reserved", 19),
        ]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_RECEIVEV).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_RECEIVEV_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_REPLYV)
        .fast(&[h("rcvid", 0), h("status", 1)])
        .wide(&[h("rcvid", 0), d("sparts", 1), h("status", 2), s("smsg[0]", 3, 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_REPLYV_64).fast(&[]).wide(&[
        h("rcvid", 0),
        e("sparts", 1),
        q("status", 3),
        s("smsg[0]", 5, 3),
    ]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_REPLYV)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_REPLYV_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_REPLYV).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_REPLYV_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_READV)
        .fast(&[h("rcvid", 0), h("offset", 1)])
        .wide(&[h("rcvid", 0), h("rmsg_p", 1), d("rparts", 2), h("offset", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_READV_64)
        .fast(&[h("rcvid", 0)])
        .wide(&[h("rcvid", 0), q("rmsg_p", 1), e("rparts", 3), q("offset", 5)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_READV)
        .fast(&[d("rbytes", 0), s("rmsg[0]", 1, 1)])
        .wide(&[d("rbytes", 0), s("rmsg[0]", 1, 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_READV_64)
        .fast(&[e("rbytes", 0)])
        .wide(&[e("rbytes", 0), s("rmsg[0]", 2, 3)]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_READV).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_READV_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_WRITEV)
        .fast(&[h("rcvid", 0), h("offset", 1)])
        .wide(&[h("rcvid", 0), d("sparts", 1), h("offset", 2), s("msg[0]", 3, 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_WRITEV_64)
        .fast(&[h("rcvid", 0)])
        .wide(&[h("rcvid", 0), e("sparts", 1), q("offset", 3), s("msg[0]", 5, 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_WRITEV)
        .fast(&[d("wbytes", 0), h("empty", 1)])
        .wide(&[d("wbytes", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_WRITEV_64)
        .fast(&[e("wbytes", 0)])
        .wide(&[e("wbytes", 0)]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_WRITEV).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_WRITEV_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_INFO)
        .fast(&[h("rcvid", 0), h("info_p", 1)])
        .wide(&[h("rcvid", 0), h("info_p", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_INFO_64)
        .fast(&[h("rcvid", 0), h("empty", 1)])
        .wide(&[h("rcvid", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_INFO)
        .fast(&[h("ret_val", 0), d("info->nd", 1)])
        .wide(&[
            h("ret_val", 0),
            d("info->nd", 1),
            d("info->srcnd", 2),
            d("info->pid", 3),
            d("info->tid", 4),
            d("info->chid", 5),
            d("info->scoid", 6),
            d("info->coid", 7),
            d("info->msglen", 8),
            d("info->srcmsglen", 9),
            d("info->dstmsglen", 10),
            d("info->priority", 11),
            d("info->flags", 12),
            h("empty", 13),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_INFO_64)
        .fast(&[h("ret_val", 0), d("info->nd", 1)])
        .wide(&[
            h("ret_val", 0),
            d("info->nd", 1),
            d("info->srcnd", 2),
            d("info->pid", 3),
            d("info->tid", 4),
            d("info->chid", 5),
            d("info->scoid", 6),
            d("info->coid", 7),
            e("info->msglen", 8),
            e("info->srcmsglen", 10),
            e("info->dstmsglen", 12),
            d("info->priority", 14),
            d("info->flags", 15),
            h("empty", 16),
        ]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_INFO).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_INFO_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SEND_PULSE)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_SEND_PULSE_64)
        .fast(&[h("coid", 0), h("code", 1)])
        .wide(&[h("coid", 0), d("priority", 1), h("code", 2), h("value", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SEND_PULSE)
        .fast(&[d("status", 0), h("empty", 1)])
        .wide(&[d("status", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_SEND_PULSE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SEND_PULSE).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_SEND_PULSE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_DELIVER_EVENT)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_DELIVER_EVENT_64)
        .fast(&[h("rcvid", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("rcvid", 0),
            d("event->sigev_notify", 1),
            h("pad", 2),
            q("event->sigev_notify_function_p", 3),
            q("event->sigev_value", 5),
            q("event->sigev_notify_attributes_p", 7),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_DELIVER_EVENT)
        .fast(&[h("ret_val", 0), h("event", 1)])
        .wide(&[h("ret_val", 0), h("event", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_DELIVER_EVENT_64)
        .fast(&[])
        .wide(&[h("ret_val", 0), q("event", 1)]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_DELIVER_EVENT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_DELIVER_EVENT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_RECEIVEPULSEV)
        .fast(&[h("chid", 0), d("rparts", 1)])
        .wide(&[h("chid", 0), d("rparts", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_RECEIVEPULSEV_64)
        .fast(&[])
        .wide(&[h("chid", 0), e("rparts", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_RECEIVEPULSEV)
        .fast(&[h("rcvid", 0), s("rmsg[0]", 1, 1)])
        .wide(&[
            h("rcvid", 0),
            s("rmsg[0]", 1, 3),
            d("info->nd", 4),
            d("info->srcnd", 5),
            d("info->pid", 6),
            d("info->tid", 7),
            d("info->chid", 8),
            d("info->scoid", 9),
            d("info->coid", 10),
            d("info->msglen", 11),
            d("info->srcmsglen", 12),
            d("info->dstmsglen", 13),
            d("info->priority", 14),
            d("info->flags", 15),
            d("info->reserved", 16),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_RECEIVEPULSEV_64)
        .fast(&[h("rcvid", 0), s("rmsg[0]", 1, 1)])
        .wide(&[
            h("rcvid", 0),
            s("rmsg[0]", 1, 3),
            d("info->nd", 4),
            d("info->srcnd", 5),
            d("info->pid", 6),
            d("info->tid", 7),
            d("info->chid", 8),
            d("info->scoid", 9),
            d("info->coid", 10),
            e("info->msglen", 11),
            e("info->srcmsglen", 13),
            e("info->dstmsglen", 15),
            d("info->priority", 17),
            d("info->flags", 18),
            d("info->reserved", 19),
        ]),
    ev(ClassId::KER_CALL_INT, kercall::MSG_RECEIVEPULSEV).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_RECEIVEPULSEV_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_VERIFY_EVENT)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_VERIFY_EVENT_64)
        .fast(&[h("rcvid", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("rcvid", 0),
            d("event->sigev_notify", 1),
            h("pad", 2),
            q("event->sigev_notify_function_p", 3),
            q("event->sigev_value", 5),
            q("event->sigev_notify_attributes_p", 7),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_VERIFY_EVENT)
        .fast(&[d("status", 0), h("empty", 1)])
        .wide(&[d("status", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_VERIFY_EVENT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_VERIFY_EVENT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_VERIFY_EVENT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_KILL)
        .fast(&[d("pid", 0), d("signo", 1)])
        .wide(&[
            h("nd", 0),
            d("pid", 1),
            d("tid", 2),
            d("signo", 3),
            h("code", 4),
            h("value", 5),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_KILL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_KILL)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_KILL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_KILL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_KILL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_RETURN)
        .fast(&[h("s_p", 0), h("empty", 1)])
        .wide(&[h("s_p", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_RETURN_64)
        .fast(&[q("s_p", 0)])
        .wide(&[q("s_p", 0)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_RETURN)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_RETURN_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_RETURN).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_RETURN_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_ACTION)
        .fast(&[d("signo", 0), h("act->sa_handler_p", 1)])
        .wide(&[
            d("pid", 0),
            h("sigstub_p", 1),
            d("signo", 2),
            h("act->sa_handler_p", 3),
            h("act->sa_flags", 4),
            h("act->sa_mask.bits[0]", 5),
            h("act->sa_mask.bits[1]", 6),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_ACTION_64)
        .fast(&[d("signo", 0)])
        .wide(&[
            d("pid", 0),
            q("sigstub_p", 1),
            d("signo", 3),
            q("act->sa_handler_p", 4),
            h("act->sa_flags", 6),
            h("act->sa_mask.bits[0]", 7),
            h("act->sa_mask.bits[1]", 8),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_ACTION)
        .fast(&[h("ret_val", 0), h("oact->sa_handler_p", 1)])
        .wide(&[
            h("ret_val", 0),
            h("oact->sa_handler_p", 1),
            h("oact->sa_flags", 2),
            h("oact->sa_mask.bits[0]", 3),
            h("oact->sa_mask.bits[1]", 4),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_ACTION_64)
        .fast(&[h("ret_val", 0)])
        .wide(&[
            h("ret_val", 0),
            q("oact->sa_handler_p", 1),
            h("oact->sa_flags", 3),
            h("oact->sa_mask.bits[0]", 4),
            h("oact->sa_mask.bits[1]", 5),
        ]),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_ACTION).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_ACTION_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_PROCMASK)
        .fast(&[d("pid", 0), d("tid", 1)])
        .wide(&[
            d("pid", 0),
            d("tid", 1),
            h("how", 2),
            h("sig_blocked->bits[0]", 3),
            h("sig_blocked->bits[1]", 4),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_PROCMASK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_PROCMASK)
        .fast(&[d("ret_val", 0), h("old_sig_blocked->bits[0]", 1)])
        .wide(&[
            d("ret_val", 0),
            h("old_sig_blocked->bits[0]", 1),
            h("old_sig_blocked->bits[1]", 2),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_PROCMASK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_PROCMASK).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_PROCMASK_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_SUSPEND)
        .fast(&[h("sig_blocked->bits[0]", 0), h("sig_blocked->bits[1]", 1)])
        .wide(&[h("sig_blocked->bits[0]", 0), h("sig_blocked->bits[1]", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_SUSPEND_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_SUSPEND)
        .fast(&[h("ret_val", 0), h("sig_blocked_p", 1)])
        .wide(&[h("ret_val", 0), h("sig_blocked_p", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_SUSPEND_64)
        .fast(&[])
        .wide(&[h("ret_val", 0), q("sig_blocked_p", 1)]),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_SUSPEND).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_SUSPEND_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_WAITINFO)
        .fast(&[h("sig_wait->bits[0]", 0), h("sig_wait->bits[1]", 1)])
        .wide(&[h("sig_wait->bits[0]", 0), h("sig_wait->bits[1]", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_WAITINFO_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_WAITINFO)
        .fast(&[d("sig_num", 0), d("si_code", 1)])
        .wide(&[
            d("sig_num", 0),
            d("si_signo", 1),
            d("si_code", 2),
            d("si_errno", 3),
            d("p[0]", 4),
            d("p[1]", 5),
            d("p[2]", 6),
            d("p[3]", 7),
            d("p[4]", 8),
            d("p[5]", 9),
            d("p[6]", 10),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_WAITINFO_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_WAITINFO).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_WAITINFO_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_KILL_SIGVAL)
        .fast(&[d("pid", 0), d("signo", 1)])
        .wide(&[
            h("nd", 0),
            d("pid", 1),
            d("tid", 2),
            d("signo", 3),
            h("code", 4),
            h("value", 5),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SIGNAL_KILL_SIGVAL_64)
        .fast(&[d("pid", 0), d("signo", 1)])
        .wide(&[
            h("nd", 0),
            d("pid", 1),
            d("tid", 2),
            d("signo", 3),
            h("code", 4),
            q("value", 5),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_KILL_SIGVAL)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SIGNAL_KILL_SIGVAL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_KILL_SIGVAL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SIGNAL_KILL_SIGVAL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CHANNEL_CREATE)
        .fast(&[h("flags", 0), h("empty", 1)])
        .wide(&[h("flags", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CHANNEL_CREATE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CHANNEL_CREATE)
        .fast(&[h("chid", 0), h("empty", 1)])
        .wide(&[h("chid", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CHANNEL_CREATE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CHANNEL_CREATE).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CHANNEL_CREATE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CHANNEL_DESTROY)
        .fast(&[h("chid", 0), h("empty", 1)])
        .wide(&[h("chid", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CHANNEL_DESTROY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CHANNEL_DESTROY)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CHANNEL_DESTROY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CHANNEL_DESTROY).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CHANNEL_DESTROY_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CHANNEL_CTL)
        .fast(&[d("chid", 0), d("cmd", 1)])
        .wide(&[d("chid", 0), d("cmd", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CHANNEL_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CHANNEL_CTL)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CHANNEL_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CHANNEL_CTL),
    ev(ClassId::KER_CALL_INT, kercall::CHANNEL_CTL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_ATTACH)
        .fast(&[h("nd", 0), d("pid", 1)])
        .wide(&[h("nd", 0), d("pid", 1), h("chid", 2), d("index", 3), h("flags", 4)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_ATTACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_ATTACH)
        .fast(&[h("coid", 0), h("empty", 1)])
        .wide(&[h("coid", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_ATTACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_ATTACH).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_ATTACH_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_DETACH)
        .fast(&[h("coid", 0), h("empty", 1)])
        .wide(&[h("coid", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_DETACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_DETACH)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_DETACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_DETACH).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_DETACH_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_SERVER_INFO)
        .fast(&[d("pid", 0), h("coid", 1)])
        .wide(&[d("pid", 0), h("coid", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_SERVER_INFO_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_SERVER_INFO)
        .fast(&[h("coid", 0), d("info->nd", 1)])
        .wide(&[
            h("coid", 0),
            d("info->nd", 1),
            d("info->srcnd", 2),
            d("info->pid", 3),
            d("info->tid", 4),
            d("info->chid", 5),
            d("info->scoid", 6),
            d("info->coid", 7),
            d("info->msglen", 8),
            d("info->srcmsglen", 9),
            d("info->dstmsglen", 10),
            d("info->priority", 11),
            d("info->flags", 12),
            d("info->reserved", 13),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_SERVER_INFO_64)
        .fast(&[h("coid", 0), d("info->nd", 1)])
        .wide(&[
            h("coid", 0),
            d("info->nd", 1),
            d("info->srcnd", 2),
            d("info->pid", 3),
            d("info->tid", 4),
            d("info->chid", 5),
            d("info->scoid", 6),
            d("info->coid", 7),
            e("info->msglen", 8),
            e("info->srcmsglen", 10),
            e("info->dstmsglen", 12),
            d("info->priority", 14),
            d("info->flags", 15),
            d("info->reserved", 16),
        ]),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_SERVER_INFO).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_SERVER_INFO_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_CLIENT_INFO)
        .fast(&[h("scoid", 0), d("ngroups", 1)])
        .wide(&[h("scoid", 0), d("ngroups", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_CLIENT_INFO_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_CLIENT_INFO)
        .fast(&[h("ret_val", 0), d("info->nd", 1)])
        .wide(&[
            h("ret_val", 0),
            d("info->nd", 1),
            d("info->pid", 2),
            d("info->sid", 3),
            h("flags", 4),
            d("info->ruid", 5),
            d("info->euid", 6),
            d("info->suid", 7),
            d("info->rgid", 8),
            d("info->egid", 9),
            d("info->sgid", 10),
            d("info->ngroups", 11),
            d("info->grouplist[0]", 12),
            d("info->grouplist[1]", 13),
            d("info->grouplist[2]", 14),
            d("info->grouplist[3]", 15),
            d("info->grouplist[4]", 16),
            d("info->grouplist[5]", 17),
            d("info->grouplist[6]", 18),
            d("info->grouplist[7]", 19),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_CLIENT_INFO_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_CLIENT_INFO).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_CLIENT_INFO_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_FLAGS)
        .fast(&[h("coid", 0), h("bits", 1)])
        .wide(&[d("pid", 0), h("coid", 1), h("masks", 2), h("bits", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_FLAGS_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_FLAGS)
        .fast(&[h("old_flags", 0), h("empty", 1)])
        .wide(&[h("old_flags", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_FLAGS_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_FLAGS).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_FLAGS_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_DUPFD)
        .fast(&[u("op", 0)])
        .wide(&[u("op", 0)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CONNECT_DUPFD_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_DUPFD)
        .fast(&[d("result", 0)])
        .wide(&[d("result", 0)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CONNECT_DUPFD_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_DUPFD).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CONNECT_DUPFD_64),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CREATE)
        .fast(&[h("func_p", 0), h("arg_p", 1)])
        .wide(&[
            d("pid", 0),
            h("func_p", 1),
            h("arg_p", 2),
            h("flags", 3),
            d("stacksize", 4),
            h("stackaddr_p", 5),
            h("exitfunc_p", 6),
            d("policy", 7),
            d("sched_priority", 8),
            d("sched_curpriority", 9),
            d("param.__ss_low_priority", 10),
            d("param.__ss_max_repl", 11),
            d("param.__ss_repl_period.tv_sec", 12),
            d("param.__ss_repl_period.tv_nsec", 13),
            d("param.__ss_init_budget.tv_sec", 14),
            d("param.__ss_init_budget.tv_nsec", 15),
            d("param.empty", 16),
            d("param.empty", 17),
            d("guardsize", 18),
            d("empty", 19),
            d("empty", 20),
            d("empty", 21),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CREATE_64)
        .fast(&[q("func_p", 0)])
        .wide(&[
            d("pid", 0),
            q("func_p", 1),
            q("arg_p", 3),
            h("flags", 5),
            h("pad", 6),
            d("stacksize", 7),
            q("stackaddr_p", 8),
            q("exitfunc_p", 10),
            d("policy", 12),
            d("sched_priority", 13),
            d("sched_curpriority", 14),
            d("param.__ss_low_priority", 15),
            d("param.__ss_max_repl", 16),
            q("param.__ss_repl_period.tv_sec", 17),
            q("param.__ss_repl_period.tv_nsec", 19),
            q("param.__ss_init_budget.tv_sec", 21),
            q("param.__ss_init_budget.tv_nsec", 23),
            d("guardsize", 25),
            d("empty", 26),
            d("empty", 27),
            d("empty", 28),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CREATE)
        .fast(&[h("thread_id", 0), h("owner", 1)])
        .wide(&[h("thread_id", 0), h("owner", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CREATE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CREATE).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CREATE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_DESTROY)
        .fast(&[d("tid", 0), h("status_p", 1)])
        .wide(&[d("tid", 0), d("priority", 1), h("status_p", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_DESTROY_64)
        .fast(&[])
        .wide(&[d("tid", 0), d("priority", 1), q("status_p", 2)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_DESTROY)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_DESTROY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_DESTROY).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_DESTROY_64),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_DETACH)
        .fast(&[d("tid", 0), h("empty", 1)])
        .wide(&[d("tid", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_DETACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_DETACH)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_DETACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_DETACH).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_DETACH_64),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_JOIN)
        .fast(&[d("tid", 0), h("status_p", 1)])
        .wide(&[d("tid", 0), h("status_p", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_JOIN_64)
        .fast(&[])
        .wide(&[d("tid", 0), q("status_p", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_JOIN)
        .fast(&[h("ret_val", 0), h("status_p", 1)])
        .wide(&[h("ret_val", 0), h("status_p", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_JOIN_64)
        .fast(&[])
        .wide(&[h("ret_val", 0), q("status_p", 1)]),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_JOIN).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_JOIN_64),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CANCEL)
        .fast(&[d("tid", 0), h("canstub_p", 1)])
        .wide(&[d("tid", 0), h("canstub_p", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CANCEL_64)
        .fast(&[])
        .wide(&[d("tid", 0), q("canstub_p", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CANCEL)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CANCEL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CANCEL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CANCEL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CTL)
        .fast(&[h("cmd", 0), h("data_p", 1)])
        .wide(&[h("cmd", 0), h("data_p", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CTL_64)
        .fast(&[])
        .wide(&[h("cmd", 0), q("data_p", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CTL)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CTL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CTL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CTLEXT)
        .fast(&[h("pid", 0), h("tid", 1)])
        .wide(&[h("pid", 0), h("tid", 1), h("cmd", 2), h("data_p", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::THREAD_CTLEXT_64)
        .fast(&[h("pid", 0), h("tid", 1)])
        .wide(&[h("pid", 0), h("tid", 1), h("cmd", 2), q("data_p", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CTLEXT)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::THREAD_CTLEXT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CTLEXT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::THREAD_CTLEXT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_ATTACH)
        .fast(&[d("intr", 0), h("flags", 1)])
        .wide(&[
            d("intr", 0),
            h("handler_p", 1),
            h("area_p", 2),
            d("areasize", 3),
            h("flags", 4),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_ATTACH_64)
        .fast(&[d("intr", 0), h("flags", 1)])
        .wide(&[
            d("intr", 0),
            q("handler_p", 1),
            q("area_p", 3),
            d("areasize", 5),
            h("flags", 6),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_ATTACH)
        .fast(&[h("int_fun_id", 0), h("empty", 1)])
        .wide(&[h("int_fun_id", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_ATTACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_ATTACH).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_ATTACH_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_QUERY)
        .fast(&[d("type", 0), d("id", 1)])
        .wide(&[d("type", 0), d("id", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_QUERY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_QUERY)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_QUERY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_QUERY),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_QUERY_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_DETACH)
        .fast(&[d("id", 0), h("empty", 1)])
        .wide(&[d("id", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_DETACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_DETACH)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_DETACH_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_DETACH).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_DETACH_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_WAIT)
        .fast(&[h("flags", 0), d("timeout_tv_sec", 1)])
        .wide(&[h("flags", 0), d("timeout_tv_sec", 1), d("timeout_tv_nsec", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_WAIT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_WAIT)
        .fast(&[d("ret_val", 0), h("timeout", 1)])
        .wide(&[d("ret_val", 0), h("timeout", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_WAIT_64)
        .fast(&[])
        .wide(&[d("ret_val", 0), q("timeout", 1)]),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_WAIT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_WAIT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_MASK)
        .fast(&[d("intr", 0), d("id", 1)])
        .wide(&[d("intr", 0), d("id", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_MASK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_MASK)
        .fast(&[h("mask_level", 0), h("empty", 1)])
        .wide(&[h("mask_level", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_MASK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_MASK).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_MASK_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_UNMASK)
        .fast(&[d("intr", 0), d("id", 1)])
        .wide(&[d("intr", 0), d("id", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_UNMASK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_UNMASK)
        .fast(&[h("mask_level", 0), h("empty", 1)])
        .wide(&[h("mask_level", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_UNMASK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_UNMASK).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_UNMASK_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_UNBLOCK)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_UNBLOCK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_UNBLOCK)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_UNBLOCK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_UNBLOCK),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_UNBLOCK_64),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_CTL)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::INTERRUPT_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_CTL)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::INTERRUPT_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_CTL),
    ev(ClassId::KER_CALL_INT, kercall::INTERRUPT_CTL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_TIME)
        .fast(&[d("id", 0), d("new(sec)", 1)])
        .wide(&[d("id", 0), d("new(sec)", 1), d("new(nsec)", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_TIME_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_TIME)
        .fast(&[d("ret_val", 0), d("old(sec)", 1)])
        .wide(&[d("ret_val", 0), d("old(sec)", 1), d("old(nsec)", 2)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_TIME_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_TIME).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_TIME_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_ADJUST)
        .fast(&[d("id", 0), d("new->tick_count", 1)])
        .wide(&[d("id", 0), d("new->tick_count", 1), d("new->tick_nsec_inc", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_ADJUST_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_ADJUST)
        .fast(&[d("ret_val", 0), d("old->tick_count", 1)])
        .wide(&[d("ret_val", 0), d("old->tick_count", 1), d("old->tick_nsec_inc", 2)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_ADJUST_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_ADJUST).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_ADJUST_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_PERIOD)
        .fast(&[d("id", 0), d("new->nsec", 1)])
        .wide(&[d("id", 0), d("new->nsec", 1), d("new->fract", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_PERIOD_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_PERIOD)
        .fast(&[d("ret_val", 0), d("old->nsec", 1)])
        .wide(&[d("ret_val", 0), d("old->nsec", 1), d("old->fract", 2)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_PERIOD_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_PERIOD).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_PERIOD_64),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_ID)
        .fast(&[d("pid", 0), d("tid", 1)])
        .wide(&[d("pid", 0), d("tid", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::CLOCK_ID_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_ID)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::CLOCK_ID_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_ID).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::CLOCK_ID_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_CREATE)
        .fast(&[h("id", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("id", 0),
            d("event->sigev_notify", 1),
            h("event->sigev_notify_function_p", 2),
            h("event->sigev_value", 3),
            h("event->sigev_notify_attributes_p", 4),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_CREATE_64)
        .fast(&[h("id", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("id", 0),
            d("event->sigev_notify", 1),
            h("pad", 2),
            q("event->sigev_notify_function_p", 3),
            q("event->sigev_value", 5),
            q("event->sigev_notify_attributes_p", 7),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_CREATE)
        .fast(&[h("timer_id", 0), h("empty", 1)])
        .wide(&[h("timer_id", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_CREATE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_CREATE).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_CREATE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_DESTROY)
        .fast(&[h("id", 0), h("empty", 1)])
        .wide(&[h("id", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_DESTROY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_DESTROY)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_DESTROY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_DESTROY).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_DESTROY_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_SETTIME)
        .fast(&[h("id", 0), d("itime->nsec(sec)", 1)])
        .wide(&[
            h("id", 0),
            h("flags", 1),
            d("itime->nsec(sec)", 2),
            d("itime->nsec(nsec)", 3),
            d("itime->interval_nsec(sec)", 4),
            d("itime->interval_nsec(nsec)", 5),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_SETTIME_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_SETTIME)
        .fast(&[h("ret_val", 0), d("oitime->nsec(sec)", 1)])
        .wide(&[
            h("ret_val", 0),
            d("oitime->nsec(sec)", 1),
            d("oitime->nsec(nsec)", 2),
            d("oitime->interval_nsec(sec)", 3),
            d("oitime->interval_nsec(nsec)", 4),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_SETTIME_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_SETTIME).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_SETTIME_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_INFO)
        .fast(&[d("pid", 0), h("id", 1)])
        .wide(&[d("pid", 0), h("id", 1), h("flags", 2), h("info_p", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_INFO_64)
        .fast(&[d("pid", 0), h("id", 1)])
        .wide(&[d("pid", 0), h("id", 1), h("flags", 2), q("info_p", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_INFO)
        .fast(&[d("prev_id", 0), d("info->itime.nsec(sec)", 1)])
        .wide(&[
            d("prev_id", 0),
            d("info->itime.nsec(sec)", 1),
            d("info->itime.nsec(nsec)", 2),
            d("info->itime.interval_nsec(sec)", 3),
            d("info->itime.interval_nsec(nsec)", 4),
            d("info->otime.nsec(sec)", 5),
            d("info->otime.nsec(nsec)", 6),
            d("info->otime.interval_nsec(sec)", 7),
            d("info->otime.interval_nsec(nsec)", 8),
            d("info->flags", 9),
            d("info->tid", 10),
            d("info->notify", 11),
            d("info->clockid", 12),
            d("info->overruns", 13),
            d("info->event.sigev_notify", 14),
            h("info->event.sigev_notify_function_p", 15),
            h("info->event.sigev_value", 16),
            h("info->event.sigev_notify_attributes_p", 17),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_INFO_64)
        .fast(&[d("prev_id", 0), d("info->itime.nsec(sec)", 1)])
        .wide(&[
            d("prev_id", 0),
            d("info->itime.nsec(sec)", 1),
            d("info->itime.nsec(nsec)", 2),
            d("info->itime.interval_nsec(sec)", 3),
            d("info->itime.interval_nsec(nsec)", 4),
            d("info->otime.nsec(sec)", 5),
            d("info->otime.nsec(nsec)", 6),
            d("info->otime.interval_nsec(sec)", 7),
            d("info->otime.interval_nsec(nsec)", 8),
            d("info->flags", 9),
            d("info->tid", 10),
            d("info->notify", 11),
            d("info->clockid", 12),
            d("info->overruns", 13),
            d("info->event.sigev_notify", 14),
            h("pad", 15),
            q("info->event.sigev_notify_function_p", 16),
            q("info->event.sigev_value", 18),
            q("info->event.sigev_notify_attributes_p", 20),
        ]),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_INFO).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_INFO_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_ALARM)
        .fast(&[h("id", 0), d("itime->nsec(sec)", 1)])
        .wide(&[
            h("id", 0),
            d("itime->nsec(sec)", 1),
            d("itime->nsec(nsec)", 2),
            d("itime->interval_nsec(sec)", 3),
            d("itime->interval_nsec(nsec)", 4),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_ALARM_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_ALARM)
        .fast(&[h("ret_val", 0), d("oitime->nsec(sec)", 1)])
        .wide(&[
            h("ret_val", 0),
            d("oitime->nsec(sec)", 1),
            d("oitime->nsec(nsec)", 2),
            d("oitime->interval_nsec(sec)", 3),
            d("oitime->interval_nsec(nsec)", 4),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_ALARM_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_ALARM).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_ALARM_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_TIMEOUT)
        .fast(&[h("timeout_flags", 0), d("ntime(sec)", 1)])
        .wide(&[
            d("id", 0),
            h("timeout_flags", 1),
            d("ntime(sec)", 2),
            d("ntime(nsec)", 3),
            d("event->sigev_notify", 4),
            h("event->sigev_notify_function_p", 5),
            h("event->sigev_value", 6),
            h("event->sigev_notify_attributes_p", 7),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_TIMEOUT_64)
        .fast(&[h("timeout_flags", 0), d("ntime(sec)", 1)])
        .wide(&[
            d("id", 0),
            h("timeout_flags", 1),
            d("ntime(sec)", 2),
            d("ntime(nsec)", 3),
            d("event->sigev_notify", 4),
            h("pad", 5),
            q("event->sigev_notify_function_p", 6),
            q("event->sigev_value", 8),
            q("event->sigev_notify_attributes_p", 10),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_TIMEOUT)
        .fast(&[h("prev_timeout_flags", 0), d("otime(sec)", 1)])
        .wide(&[h("prev_timeout_flags", 0), d("otime(sec)", 1), d("otime(nsec)", 2)]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_TIMEOUT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_TIMEOUT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_TIMEOUT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_DELEGATE)
        .fast(&[d("action", 0), d("delegate_cpu", 1)])
        .wide(&[d("action", 0), d("delegate_cpu", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::TIMER_DELEGATE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_DELEGATE)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::TIMER_DELEGATE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_DELEGATE),
    ev(ClassId::KER_CALL_INT, kercall::TIMER_DELEGATE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_RWLOCK)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_RWLOCK_64)
        .fast(&[q("syncp", 0)])
        .wide(&[q("syncp", 0), d("op", 2), d("count", 3), d("owner", 4)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_RWLOCK)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_RWLOCK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_RWLOCK),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_RWLOCK_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CREATE)
        .fast(&[d("type", 0), h("sync_p", 1)])
        .wide(&[
            d("type", 0),
            h("sync_p", 1),
            d("count", 2),
            d("owner", 3),
            d("protocol", 4),
            h("flags", 5),
            d("prioceiling", 6),
            d("clockid", 7),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CREATE_64)
        .fast(&[d("type", 0)])
        .wide(&[
            d("type", 0),
            q("sync_p", 1),
            d("count", 3),
            d("owner", 4),
            d("protocol", 5),
            h("flags", 6),
            d("prioceiling", 7),
            d("clockid", 8),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CREATE)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CREATE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CREATE).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CREATE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_DESTROY)
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_DESTROY_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_DESTROY)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_DESTROY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_DESTROY).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_DESTROY_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_MUTEX_LOCK)
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_MUTEX_LOCK_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_MUTEX_LOCK)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_MUTEX_LOCK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_MUTEX_LOCK).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_MUTEX_LOCK_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_MUTEX_UNLOCK)
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_MUTEX_UNLOCK_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_MUTEX_UNLOCK)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_MUTEX_UNLOCK_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_MUTEX_UNLOCK).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_MUTEX_UNLOCK_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CONDVAR_WAIT)
        .fast(&[h("sync_p", 0), h("mutex_p", 1)])
        .wide(&[
            h("sync_p", 0),
            h("mutex_p", 1),
            d("sync->count", 2),
            d("sync->owner", 3),
            d("mutex->count", 4),
            d("mutex->owner", 5),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CONDVAR_WAIT_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[
            q("sync_p", 0),
            q("mutex_p", 2),
            d("sync->count", 4),
            d("sync->owner", 5),
            d("mutex->count", 6),
            d("mutex->owner", 7),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CONDVAR_WAIT)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CONDVAR_WAIT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CONDVAR_WAIT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CONDVAR_WAIT_64).wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CONDVAR_SIGNAL)
        .fast(&[h("sync_p", 0), d("all", 1)])
        .wide(&[h("sync_p", 0), d("all", 1), d("sync->count", 2), d("sync->owner", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CONDVAR_SIGNAL_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("all", 2), d("sync->count", 3), d("sync->owner", 4)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CONDVAR_SIGNAL)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CONDVAR_SIGNAL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CONDVAR_SIGNAL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CONDVAR_SIGNAL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_SEM_POST)
        .fast(&[h("sync_p", 0), d("count", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_SEM_POST_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_SEM_POST)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_SEM_POST_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_SEM_POST).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_SEM_POST_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_SEM_WAIT)
        .fast(&[h("sync_p", 0), d("count", 1)])
        .wide(&[h("sync_p", 0), d("try", 1), d("count", 2), d("owner", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_SEM_WAIT_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("try", 2), d("count", 3), d("owner", 4)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_SEM_WAIT)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_SEM_WAIT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_SEM_WAIT).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_SEM_WAIT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CTL)
        .fast(&[d("cmd", 0), h("sync_p", 1)])
        .wide(&[
            d("cmd", 0),
            h("sync_p", 1),
            h("data_p", 2),
            d("count", 3),
            d("owner", 4),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_CTL_64)
        .fast(&[d("cmd", 0)])
        .wide(&[
            d("cmd", 0),
            q("sync_p", 1),
            q("data_p", 3),
            d("count", 5),
            d("owner", 6),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CTL)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CTL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_CTL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_BARRIER_WAIT)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_BARRIER_WAIT_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_BARRIER_WAIT)
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_BARRIER_WAIT_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_BARRIER_WAIT),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_BARRIER_WAIT_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_GET)
        .fast(&[d("pid", 0), d("tid", 1)])
        .wide(&[d("pid", 0), d("tid", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_GET_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_GET)
        .fast(&[d("ret_val", 0), d("sched_priority", 1)])
        .wide(&[
            d("ret_val", 0),
            d("sched_priority", 1),
            d("sched_curpriority", 2),
            d("param.__ss_low_priority", 3),
            d("param.__ss_max_repl", 4),
            d("param.__ss_repl_period.tv_sec", 5),
            d("param.__ss_repl_period.tv_nsec", 6),
            d("param.__ss_init_budget.tv_sec", 7),
            d("param.__ss_init_budget.tv_nsec", 8),
            d("param.empty", 9),
            d("param.empty", 10),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_GET_64)
        .fast(&[d("ret_val", 0), d("sched_priority", 1)])
        .wide(&[
            d("ret_val", 0),
            d("sched_priority", 1),
            d("sched_curpriority", 2),
            d("param.__ss_low_priority", 3),
            d("param.__ss_max_repl", 4),
            e("param.__ss_repl_period.tv_sec", 5),
            e("param.__ss_repl_period.tv_nsec", 7),
            e("param.__ss_init_budget.tv_sec", 9),
            e("param.__ss_init_budget.tv_nsec", 11),
        ]),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_GET).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_GET_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_SET)
        .fast(&[d("pid", 0), d("sched_priority", 1)])
        .wide(&[
            d("pid", 0),
            d("tid", 1),
            d("policy", 2),
            d("sched_priority", 3),
            d("sched_curpriority", 4),
            d("param.__ss_low_priority", 5),
            d("param.__ss_max_repl", 6),
            d("param.__ss_repl_period.tv_sec", 7),
            d("param.__ss_repl_period.tv_nsec", 8),
            d("param.__ss_init_budget.tv_sec", 9),
            d("param.__ss_init_budget.tv_nsec", 10),
            d("param.empty", 11),
            d("param.empty", 12),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_SET_64)
        .fast(&[d("pid", 0), d("sched_priority", 1)])
        .wide(&[
            d("pid", 0),
            d("tid", 1),
            d("policy", 2),
            d("sched_priority", 3),
            d("sched_curpriority", 4),
            d("param.__ss_low_priority", 5),
            d("param.__ss_max_repl", 6),
            e("param.__ss_repl_period.tv_sec", 7),
            e("param.__ss_repl_period.tv_nsec", 9),
            e("param.__ss_init_budget.tv_sec", 11),
            e("param.__ss_init_budget.tv_nsec", 13),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_SET)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_SET_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_SET).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_SET_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_YIELD)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_YIELD_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_YIELD)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_YIELD_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_YIELD).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_YIELD_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_INFO)
        .fast(&[d("pid", 0), d("policy", 1)])
        .wide(&[d("pid", 0), d("policy", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_INFO_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_INFO)
        .fast(&[d("ret_val", 0), d("priority_max", 1)])
        .wide(&[
            d("ret_val", 0),
            d("priority_min", 1),
            d("priority_max", 2),
            d("interval_sec", 3),
            d("interval_nsec", 4),
            d("priority_priv", 5),
        ]),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_INFO_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_INFO).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_INFO_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_CTL)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::SCHED_CTL_64)
        .fast(&[h("cmd", 0), q("data", 1)])
        .wide(&[h("cmd", 0), q("data", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_CTL)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SCHED_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_CTL).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SCHED_CTL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_OPEN)
        .fast(&[q("rcvid", 0)])
        .wide(&[
            q("rcvid", 0),
            d("queueid", 2),
            d("msgsize", 3),
            d("maxmsg", 4),
            d("oflag", 5),
        ]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_OPEN_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_OPEN)
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_OPEN_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_OPEN),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_OPEN_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_SEND)
        .fast(&[d("fd", 0), s("msg[0]", 1, 1)])
        .wide(&[d("fd", 0), s("msg[0]", 1, 1), e("msglen", 2), d("priority", 4)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_SEND_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_SEND)
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_SEND_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_SEND),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_SEND_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_RECEIVE)
        .fast(&[d("fd", 0)])
        .wide(&[d("fd", 0), e("msglen", 1), q("msg", 3), q("priority", 5)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_RECEIVE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_RECEIVE)
        .fast(&[d("msglen", 0), s("msg[0]", 1, 1)])
        .wide(&[d("msglen", 0), s("msg[0]", 1, 1), d("priority", 2)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_RECEIVE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_RECEIVE),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_RECEIVE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_CLOSE)
        .fast(&[q("rcvid", 0)])
        .wide(&[q("rcvid", 0), d("queueid", 2)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_CLOSE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_CLOSE)
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_CLOSE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_CLOSE),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_CLOSE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_CTL)
        .fast(&[d("fd", 0), d("cmd", 1)])
        .wide(&[d("fd", 0), d("cmd", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::MSG_QUEUE_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_CTL)
        .fast(&[d("ret_val", 0), d("cmd", 1)])
        .wide(&[d("ret_val", 0), d("cmd", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::MSG_QUEUE_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_CTL),
    ev(ClassId::KER_CALL_INT, kercall::MSG_QUEUE_CTL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::PIPE_OPEN)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::PIPE_OPEN_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::PIPE_OPEN)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::PIPE_OPEN_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::PIPE_OPEN),
    ev(ClassId::KER_CALL_INT, kercall::PIPE_OPEN_64),
    ev(ClassId::KER_CALL_ENTER, kercall::PIPE_CTL)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::PIPE_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::PIPE_CTL)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::PIPE_CTL_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::PIPE_CTL),
    ev(ClassId::KER_CALL_INT, kercall::PIPE_CTL_64),
    ev(ClassId::KER_CALL_ENTER, kercall::PIPE_CLOSE)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::PIPE_CLOSE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::PIPE_CLOSE)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::PIPE_CLOSE_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::PIPE_CLOSE),
    ev(ClassId::KER_CALL_INT, kercall::PIPE_CLOSE_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_TYPE_DESTROY)
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2), d("type", 3)]),
    ev(ClassId::KER_CALL_ENTER, kercall::SYNC_TYPE_DESTROY_64)
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3), d("type", 4)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_TYPE_DESTROY)
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::SYNC_TYPE_DESTROY_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_TYPE_DESTROY).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::SYNC_TYPE_DESTROY_64),
    ev(ClassId::KER_CALL_ENTER, kercall::TEST).fast_silent().wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::TEST_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::TEST).fast_silent().wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::TEST_64).fast_silent().wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::TEST),
    ev(ClassId::KER_CALL_INT, kercall::TEST_64),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_CUSTOM)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_ENTER, kercall::SYS_CUSTOM_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_CUSTOM)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::SYS_CUSTOM_64)
        .fast_silent()
        .wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::SYS_CUSTOM),
    ev(ClassId::KER_CALL_INT, kercall::SYS_CUSTOM_64),
    ev(ClassId::KER_CALL_ENTER, kercall::BAD)
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_ENTER, kercall::BAD_64).fast_silent().wide_silent(),
    ev(ClassId::KER_CALL_EXIT, kercall::BAD)
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev(ClassId::KER_CALL_EXIT, kercall::BAD_64).fast_silent().wide_silent(),
    ev(ClassId::KER_CALL_INT, kercall::BAD).intcall(),
    ev(ClassId::KER_CALL_INT, kercall::BAD_64),
    // ---- INT
    ev(ClassId::INT, int::DELIVER)
        .name("INT_DELIVER")
        .args(&[t("intr_id:0x{x0}({u0})")]),
    // ---- THREAD(スレッドの状態は PR_TH の PR_TH_PROCESS_BASE より小さい番号)
    ev(ClassId::THREAD, thread::DEAD).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::RUNNING).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::READY).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::STOPPED).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::SEND).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::RECEIVE).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::REPLY).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::MQ_SEND).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::MQ_RECEIVE).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::WAITPAGE).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::SIGSUSPEND).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::SIGWAITINFO).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::NANOSLEEP).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::MUTEX).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::CONDVAR).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::JOIN).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::INTR).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::SEM).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::WAITCTX).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::RWLOCK_READ).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::RWLOCK_WRITE).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::BARRIER).name("unknown").args(THREAD_BODY),
    ev(ClassId::THREAD, thread::PIPE).name("unknown").args(THREAD_BODY),
    ev(ClassId::THREAD, thread::CREATE).args(&[d("pid", 0), d("tid", 1), d("policy", 2)]),
    ev(ClassId::THREAD, thread::DESTROY).args(&[d("pid", 0), d("tid", 1)]),
    ev(ClassId::THREAD, thread::MUON_MUTEX).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::TRACEBUFFER).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::INTR_ATTACH_EV).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::TIMER_DELEGATE).args(THREAD_BODY),
    ev(ClassId::THREAD, thread::CREATE_VT).name("THCREATE_VT"),
    ev(ClassId::THREAD, thread::DESTROY_VT).name("THDESTROY_VT"),
    any(ClassId::THREAD)
        .max_code(THREAD_MAX_PRINTED)
        .name("unknown")
        .args(THREAD_BODY),
    // ---- PROCESS
    ev(ClassId::PROCESS, process::CREATE_P)
        .name("PROCCREATE")
        .args(&[d("ppid", 0), d("pid", 1)]),
    ev(ClassId::PROCESS, process::DESTROY_P)
        .name("PROCDESTROY")
        .args(&[d("ppid", 0), d("pid", 1)]),
    ev(ClassId::PROCESS, process::CREATE_P_NAME)
        .name("PROCCREATE_NAME")
        .args(&[d("ppid", 0), d("pid", 1), sz("name", 2)]),
    ev(ClassId::PROCESS, process::DESTROY_P_NAME)
        .name("PROCDESTROY_NAME")
        .args(&[d("ppid", 0), d("pid", 1), sz("name", 2)]),
    ev(ClassId::PROCESS, process::NAME_T)
        .name("PROCTHREAD_NAME")
        .args(&[d("pid", 0), d("tid", 1), sz("name", 2)]),
    // ---- SYSTEM
    ev(ClassId::SYSTEM, system::RESERVED).name("").wide_silent(),
    ev(ClassId::SYSTEM, system::PATHMGR)
        .name("PATHMGR_OPEN")
        .wide(&[d("pid", 0), d("tid", 1), sz("path", 2)]),
    ev(ClassId::SYSTEM, system::APS_NAME).wide(&[d("partition_id", 0), sz("name", 1)]),
    ev(ClassId::SYSTEM, system::APS_BUDGETS).name("APS_NEW_BUDGET").wide(&[
        d("partition_id", 0),
        d("min_percentage_budget", 1),
        d("critical_budget", 2),
        d("max_percentage_budget", 3),
        d("critical_priority", 4),
        d("budget_percent_scale", 5),
    ]),
    ev(ClassId::SYSTEM, system::APS_BNKR).name("APS_BANKRUPTCY").wide(&[
        d("pid", 0),
        d("tid", 1),
        d("partition_id", 2),
    ]),
    ev(ClassId::SYSTEM, system::MMAP).wide(&[
        d("pid", 0),
        q("vaddr", 1),
        q("len", 3),
        h("flags", 5),
        h("prot", 6),
        d("fd", 7),
        q("align", 8),
        q("offset", 10),
        sz("path", 12),
    ]),
    ev(ClassId::SYSTEM, system::MUNMAP).wide(&[d("pid", 0), q("vaddr", 1), q("len", 3)]),
    ev(ClassId::SYSTEM, system::MAPNAME).wide(&[d("pid", 0), h("vaddr", 1), h("len", 2), sz("name", 3)]),
    ev(ClassId::SYSTEM, system::ADDRESS).wide_silent(),
    ev(ClassId::SYSTEM, system::ADDRESS_64).wide(&[q("addr", 0)]),
    ev(ClassId::SYSTEM, system::FUNC_ENTER).wide(&[h("thisfn", 0), h("call_site", 1)]),
    ev(ClassId::SYSTEM, system::FUNC_ENTER_64).wide(&[q("thisfn", 0), q("call_site", 2)]),
    ev(ClassId::SYSTEM, system::FUNC_EXIT).wide(&[h("thisfn", 0), h("call_site", 1)]),
    ev(ClassId::SYSTEM, system::FUNC_EXIT_64).wide(&[q("thisfn", 0), q("call_site", 2)]),
    ev(ClassId::SYSTEM, system::SLOG).wide(&[d("opcode", 0), d("severity", 1), sz("message", 2)]),
    ev(ClassId::SYSTEM, system::DEFRAG_START).wide(&[d("block_size", 0)]),
    ev(ClassId::SYSTEM, system::RUNSTATE).wide(&[h("bitset", 0)]),
    ev(ClassId::SYSTEM, system::POWER).args(&[t(" bitset:0x{x0:08}\t actual frequency:{u1} cpu:{u0&0xffff}")]),
    ev(ClassId::SYSTEM, system::IPI).args_silent(),
    ev(ClassId::SYSTEM, system::IPI_64)
        .name("IPI_64")
        .args(&[t(" ipicmd = {x0:#08}, "), flags(0, "ipi_cmd", " {name}")]),
    ev(ClassId::SYSTEM, system::PAGEWAIT).args(&[
        t(" pid:{d0} tid:{d1} ip:0x{x2:08} vaddr:0x{x3:08} type:{x4:04} --- flags:0x{x5:08} offset:0x{q6:016}"),
        t(" name:{z8:6}").when("words >= 9"),
    ]),
    ev(ClassId::SYSTEM, system::TIMER).args(&[
        t(" pid:{d0} tid:{d1} timer_id:{d2} flags:0x{x3:08}"),
        flags(3, "timer_flags", " {name}"),
    ]),
    ev(ClassId::SYSTEM, system::DEFRAG_END).wide(&[d("result", 0), d("freemem", 1), d("maxblock", 2)]),
    ev(ClassId::SYSTEM, system::PROFILE).args(&[t(" ip = 0x{x0:08}, tid = 0x{x1:08}, pid = 0x{x2:08}")]),
    ev(ClassId::SYSTEM, system::PROFILE_64)
        .name("PROFILE_64")
        .args(&[t(" ip = 0x{q0:016}, tid = 0x{x2:08}, pid = 0x{x3:08}")]),
    ev(ClassId::SYSTEM, system::MAPNAME_64).wide(&[d("pid", 0), q("vaddr", 1), e("len", 3), sz("name", 5)]),
    ev(ClassId::SYSTEM, system::APS_PSTATS).wide(&[
        d("id", 0),
        d("stats_flags", 1),
        e("run_time_cycles", 2),
        e("critical_time_cycles", 4),
        e("dynamic_windowsize_cycles", 6),
    ]),
    ev(ClassId::SYSTEM, system::APS_OSTATS).wide(&[
        d("pid_at_last_bankruptcy", 0),
        d("tid_at_last_bankruptcy", 1),
        d("id_at_last_bankruptcy", 2),
    ]),
    ev(ClassId::SYSTEM, system::APS_INFO).wide(&[
        d("windowsize_ms", 0),
        d("scheduling_policy_flags", 1),
        d("sec_flags", 2),
        d("bankruptcy_policy", 3),
        d("num_partitions", 4),
        d("max_partitions", 5),
        e("cycles_per_ms", 6),
    ]),
    ev(ClassId::SYSTEM, system::APS_JOIN).wide(&[d("id", 0), d("pid", 1), d("tid", 2), d("aid", 3)]),
    ev(ClassId::SYSTEM, system::APS_THREAD).wide(&[
        d("pid", 0),
        d("tid", 1),
        d("id", 2),
        d("inherited_id", 3),
        d("crit_state_flags", 4),
    ]),
    ev(ClassId::SYSTEM, system::APS_PROCESS).wide(&[d("pid", 0), d("id", 1)]),
    ev(ClassId::SYSTEM, system::SCHED_CONF).wide_silent(),
    ev(ClassId::SYSTEM, system::IST_ATTACH).min_words(6).args(&[
        t("pid:{d0} tid:{d1} intr_id:0x{x2}({u2})"),
        t(" intr_cpu:{d3:02}").when("u3 != 0xffffffff"),
        t(" flags:0x{x4:08}"),
        t(" EVENT").when("u4 & 0x10000000"),
        t(" ist_prio:{d5}"),
    ]),
    ev(ClassId::SYSTEM, system::IST_DETACH).min_words(6).args(&[
        t("pid:{d0} tid:{d1} intr_id:0x{x2}({u2})"),
        t(" intr_cpu:{d3:02}").when("u3 != 0xffffffff"),
        t(" flags:0x{x4:08}"),
        t(" EVENT").when("u4 & 0x10000000"),
        t(" ist_prio:{d5}"),
    ]),
    ev(ClassId::SYSTEM, system::PCTRACE).wide_silent(),
    // ---- USER(番号は任意)
    any(ClassId::USER).name("EVENT:{code}").args(&[dump()]),
    // ---- COMM
    ev(ClassId::COMM, comm::SMSG)
        .name("SND_MESSAGE")
        .args(&[h("rcvid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::SPULSE)
        .name("SND_PULSE")
        .args(&[h("scoid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::RMSG)
        .name("REC_MESSAGE")
        .args(&[h("rcvid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::RPULSE)
        .name("REC_PULSE")
        .args(&[h("scoid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::SPULSE_EXE)
        .name("SND_PULSE_EXE")
        .args(&[h("scoid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::SPULSE_DIS)
        .name("SND_PULSE_DIS")
        .args(&[h("scoid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::SPULSE_DEA)
        .name("SND_PULSE_DEA")
        .args(&[h("scoid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::SPULSE_UN)
        .name("SND_PULSE_UN")
        .args(&[h("scoid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::SPULSE_QUN)
        .name("SND_PULSE_QUN")
        .args(&[h("scoid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::SIGNAL).args(&[
        d("si_signo", 0),
        d("si_code", 1),
        d("si_errno", 2),
        h("__pad[0]", 3),
        h("__pad[1]", 4),
        h("__pad[2]", 5),
        h("__pad[3]", 6),
        h("__pad[4]", 7),
        h("__pad[5]", 8),
    ]),
    ev(ClassId::COMM, comm::REPLY)
        .name("REPLY_MESSAGE")
        .args(&[d("tid", 0), d("pid", 1)]),
    ev(ClassId::COMM, comm::ERROR)
        .name("MSG_ERROR")
        .args(&[d("tid", 0), d("pid", 1)]),
    // ---- QUIP(番号は任意)
    any(ClassId::QUIP).name("EVENT:{code}").args(&[dump()]),
    // ---- SEC
    ev(ClassId::SEC, sec::ABLE).args(&[
        t(" pid:{d0} able:{u1} ({enum:sec_able:u1:UNKNOWN})"),
        t(" start:0x{q4:016}").when("words >= 6"),
        t(" end:0x{q6:016}").when("words >= 8"),
        t(" ALLOW").when("words >= 4 && u2 & 0x1"),
        t(" DENY").when("words >= 4 && u2 & 0x1 == 0"),
        t(" as root").when("words >= 4 && u2 & 0x2"),
        t(" as non-root").when("words >= 4 && u2 & 0x2 == 0"),
        t(" TRUNC").when("words < 8"),
    ]),
    ev(ClassId::SEC, sec::ABLE_LOOKUP).args(&[t(" name:{r1} able:{u0}")]),
    ev(ClassId::SEC, sec::PATH_ATTACH).args(&[
        t(" pid:{d0} ptype:{u1}"),
        t(" TRUNC").when("words < 4"),
        t(" ctype:{d2} chid:{d3} status:{enum:sec_path_status:u4:UNKNOWN}").when("words >= 4"),
        t(" TRUNC").when("words >= 4 && words < 6"),
        t(" path:{r5}").when("words >= 6"),
    ]),
    ev(ClassId::SEC, sec::QNET_CONNECT).name("unknown(3)").args(&[]),
    ev(ClassId::SEC, sec::PERM_LOOKUP).args(&[t(" pid:{d0} class:{r4} ({d2}) perm:{rnext} ({d3}) errno:{d1}")]),
    ev(ClassId::SEC, sec::PERM_TEST).args(&[
        t(" pid:{d0} ptype:{u1} otype:{u2} class:{d3} perm:{d4} errno:{u5}").when("words >= 6"),
        t(" pid:{d0} ptype:{u1} TRUNC").when("words < 6"),
    ]),
    ev(ClassId::SEC, sec::UNREG_EVENT).args(&[
        t(" src pid:{d0} dst:{r4} (pid:{d1} tid:{u2}) denied:{u3}").when("words >= 6"),
        t(" TRUNC").when("words < 6"),
    ]),
    ev(ClassId::SEC, sec::OPEN_EVENT).name("unknown(7)").args(&[]),
    any(ClassId::SEC).name("unknown({code})").args(&[]),
    // ---- QVM
    ev(ClassId::QVM, qvm::GUEST_ENTER).wide(&[q("guest_ip", 0)]),
    ev(ClassId::QVM, qvm::GUEST_EXIT).wide(&[
        h("status", 0),
        h("hw_reason", 1),
        q("clockcycles_offset", 2),
        q("guest_ip", 4),
        q("payload", 6),
    ]),
    ev(ClassId::QVM, qvm::CREATE_VCPU_THREAD).wide(&[d("vcpu_id", 0)]),
    ev(ClassId::QVM, qvm::RAISE_INTR)
        .name("INTR_RAISE")
        .wide(&[d("vcpu_id", 0), h("intr_vector", 1)]),
    ev(ClassId::QVM, qvm::LOWER_INTR)
        .name("INTR_LOWER")
        .wide(&[d("vcpu_id", 0), h("intr_vector", 1)]),
    ev(ClassId::QVM, qvm::TIMER_CREATE).wide(&[h("timer_id", 0), d("vcpu_id", 1)]),
    ev(ClassId::QVM, qvm::TIMER_FIRE).wide(&[h("timer_id", 0)]),
    ev(ClassId::QVM, qvm::CYCLES).wide(&[q("at_entry", 0), q("at_exit", 2)]),
];
