//! QNX 8.0 の traceprinter(1.02)の表示の定義。
//!
//! traceprinter の出力とバイト単位で同じにするための、表示名・引数の書式・出力しない条件。
//! 手で保守する。イベントは def.rs の名前(内部クラス名・内部イベント名)で指す。
//! 書式の要素とテンプレートの書き方は crate::tp_format を参照。

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
/// 内部クラス名 → 表示クラス名(KER_CALL と PR_TH は下)
pub const CLASS_DISPLAY: &[(&str, &str)] = &[
    ("CONTROL", "CONTROL"),
    ("INT", "INTERRUPT"),
    ("SYSTEM", "SYSTEM"),
    ("USER", "USREVENT"),
    ("COMM", "COMM"),
    ("QUIP", "QUIP"),
    ("SEC", "SEC"),
    ("QVM", "QVM"),
];
pub const DISPLAY_KER_ENTER: &str = "KER_CALL";
pub const DISPLAY_KER_EXIT: &str = "KER_EXIT";
pub const DISPLAY_KER_INT: &str = "KER_INT";
pub const DISPLAY_INT_CALL: &str = "INT_CALL";
pub const DISPLAY_THREAD: &str = "THREAD";
pub const DISPLAY_PROCESS: &str = "PROCESS";

// ---------------------------------------------------------------- fast の書式が無いとき
/// true: wide の書式で表示する、false: 表示しない
pub const FAST_FALLBACK_WIDE: &[(&str, bool)] = &[("KER_CALL", false)];
pub const FAST_FALLBACK_WIDE_DEFAULT: bool = true;

// ---------------------------------------------------------------- 表示イベント名
/// def.rs の名前と違う表示名(None = traceprinter が名前を持たない)
pub const EVENT_NAMES: &[(&str, &str, Option<&str>)] = &[
    ("PR_TH", "CREATE_P", Some("PROCCREATE")),
    ("PR_TH", "DESTROY_P", Some("PROCDESTROY")),
    ("PR_TH", "CREATE_P_NAME", Some("PROCCREATE_NAME")),
    ("PR_TH", "DESTROY_P_NAME", Some("PROCDESTROY_NAME")),
    ("PR_TH", "NAME_T", Some("PROCTHREAD_NAME")),
    ("PR_TH", "CREATE_VT", Some("THCREATE_VT")),
    ("PR_TH", "DESTROY_VT", Some("THDESTROY_VT")),
    ("INT", "DELIVER", Some("INT_DELIVER")),
    ("SYSTEM", "RESERVED", None),
    ("SYSTEM", "PATHMGR", Some("PATHMGR_OPEN")),
    ("SYSTEM", "APS_BUDGETS", Some("APS_NEW_BUDGET")),
    ("SYSTEM", "APS_BNKR", Some("APS_BANKRUPTCY")),
    ("COMM", "SMSG", Some("SND_MESSAGE")),
    ("COMM", "SPULSE", Some("SND_PULSE")),
    ("COMM", "RMSG", Some("REC_MESSAGE")),
    ("COMM", "RPULSE", Some("REC_PULSE")),
    ("COMM", "SPULSE_EXE", Some("SND_PULSE_EXE")),
    ("COMM", "SPULSE_DIS", Some("SND_PULSE_DIS")),
    ("COMM", "SPULSE_DEA", Some("SND_PULSE_DEA")),
    ("COMM", "SPULSE_UN", Some("SND_PULSE_UN")),
    ("COMM", "SPULSE_QUN", Some("SND_PULSE_QUN")),
    ("COMM", "REPLY", Some("REPLY_MESSAGE")),
    ("COMM", "ERROR", Some("MSG_ERROR")),
    ("SEC", "QNET_CONNECT", None),
    ("SEC", "OPEN_EVENT", None),
    ("QVM", "RAISE_INTR", Some("INTR_RAISE")),
    ("QVM", "LOWER_INTR", Some("INTR_LOWER")),
];
/// 64 ビット版の表示名(ここに無いものは 64 ビットでない版の名前)
pub const EVENT_NAMES_64: &[(&str, &str, &str)] = &[("SYSTEM", "IPI", "IPI_64"), ("SYSTEM", "PROFILE", "PROFILE_64")];

// ---------------------------------------------------------------- カーネルコール
pub const KERCALL_NAME: &str = "{call}/{num:02}";
pub const KERCALL_UNKNOWN_CALL: &str = "KER_{num:02}";
/// INT_CALL の形(引数なし)
pub const INTCALL_NAME: &str = "KER_{call}/{num}";
pub const INTCALL_UNKNOWN_CALL: &str = "{num:02}";
/// int の 64 ビットでない版を INT_CALL の形で表示するコール
pub const INT_CALLS: &[&str] = &[
    "NOP",
    "TRACE_EVENT",
    "RING0",
    "CACHE_CONTROL",
    "SYS_SRANDOM",
    "MSG_REGISTER_EVENT",
    "MSG_SEND_PULSEPTR",
    "SYS_CPUPAGE_GET",
    "SYS_CPUPAGE_SET",
    "MSG_PAUSE",
    "MSG_CURRENT",
    "MSG_SENDV",
    "MSG_SENDVNC",
    "MSG_ERROR",
    "MSG_RECEIVEV",
    "MSG_REPLYV",
    "MSG_READV",
    "MSG_WRITEV",
    "MSG_INFO",
    "MSG_SEND_PULSE",
    "MSG_DELIVER_EVENT",
    "MSG_RECEIVEPULSEV",
    "MSG_VERIFY_EVENT",
    "SIGNAL_KILL",
    "SIGNAL_RETURN",
    "SIGNAL_ACTION",
    "SIGNAL_PROCMASK",
    "SIGNAL_SUSPEND",
    "SIGNAL_WAITINFO",
    "SIGNAL_KILL_SIGVAL",
    "CHANNEL_CREATE",
    "CHANNEL_DESTROY",
    "CONNECT_ATTACH",
    "CONNECT_DETACH",
    "CONNECT_SERVER_INFO",
    "CONNECT_CLIENT_INFO",
    "CONNECT_FLAGS",
    "CONNECT_DUPFD",
    "THREAD_CREATE",
    "THREAD_DESTROY",
    "THREAD_DETACH",
    "THREAD_JOIN",
    "THREAD_CANCEL",
    "THREAD_CTL",
    "THREAD_CTLEXT",
    "INTERRUPT_ATTACH",
    "INTERRUPT_DETACH",
    "INTERRUPT_WAIT",
    "INTERRUPT_MASK",
    "INTERRUPT_UNMASK",
    "CLOCK_TIME",
    "CLOCK_ADJUST",
    "CLOCK_PERIOD",
    "CLOCK_ID",
    "TIMER_CREATE",
    "TIMER_DESTROY",
    "TIMER_SETTIME",
    "TIMER_INFO",
    "TIMER_ALARM",
    "TIMER_TIMEOUT",
    "SYNC_CREATE",
    "SYNC_DESTROY",
    "SYNC_MUTEX_LOCK",
    "SYNC_MUTEX_UNLOCK",
    "SYNC_CONDVAR_WAIT",
    "SYNC_CONDVAR_SIGNAL",
    "SYNC_SEM_POST",
    "SYNC_SEM_WAIT",
    "SYNC_CTL",
    "SCHED_GET",
    "SCHED_SET",
    "SCHED_YIELD",
    "SCHED_INFO",
    "SCHED_CTL",
    "SYNC_TYPE_DESTROY",
    "BAD",
];
/// 失敗した exit(表示する exit に限る)
pub const KERCALL_FAILURE_WHEN: &str = "u0 == 0xffffffff && words >= 2";
pub const KERCALL_FAILURE: &[Item] = &[t(" ret_val:-1, errno:{u1}")];
/// コール名の表示の違い(enter / exit 共通)
pub const KERCALL_NAMES: &[(&str, &str)] = &[("SIGNAL_KILL_SIGVAL", "SIGNAL_KILL")];
/// exit だけの表示の違い
pub const KERCALL_NAMES_EXIT: &[(&str, &str)] = &[("SYNC_CONDVAR_SIGNAL", "SYNC_CONDVAR_SIG")];

// ---------------------------------------------------------------- スレッドの状態(PR_TH の PR_TH_PROCESS_BASE より小さい番号)
pub const THREAD_NAME: &str = "TH{name}";
pub const THREAD_UNKNOWN_NAME: &str = "unknown";
/// これより大きい状態の番号は出力しない
pub const THREAD_MAX_PRINTED: u16 = 29;
/// traceprinter が名前を持たない状態
pub const THREAD_HIDDEN: &[&str] = &["BARRIER", "PIPE"];
pub const THREAD_BODY: &[Item] = &[
    t(" pid:{d0} tid:{d1} "),
    t("priority:{d2} policy:{d3}").when("words >= 4"),
    t(" partition:{d4}").when("words >= 5"),
    t(" sched_flags:0x{x5:08}").when("words >= 6"),
];
/// 状態ごとの書式(fast・wide 共通)
pub const THREAD_OVERRIDES: &[(&str, &[Item])] = &[
    ("CREATE", &[d("pid", 0), d("tid", 1), d("policy", 2)]),
    ("DESTROY", &[d("pid", 0), d("tid", 1)]),
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

// ---------------------------------------------------------------- 後ろに空行を入れるイベント
pub const BLANK_AFTER: &[(&str, &str)] = &[("CONTROL", "BUFFER"), ("CONTROL", "BUFFER_END"), ("CONTROL", "DROPPED")];

// ---------------------------------------------------------------- イベントごとの書式
// kind: KER_CALL の種別、is64: 64 ビット版、args: fast と wide に同じ書式、*_silent: 出力しない。
// コメントの Fast / Wide / Call は TableOfEvents の記述(参考)

pub const EVENTS: &[Ev] = &[
    ev("CONTROL", "TIME").args(&[h("msb", 0), h("lsb(offset)", 1)]),
    ev("CONTROL", "BUFFER").args(&[t(" sequence = {u0}, num_events = {u1}")]),
    ev("CONTROL", "BUFFER_END").args(&[t(" cookie1 = {x0}, cookie2 = {x1}")]),
    ev("CONTROL", "DROPPED").args(&[t(" dropped_events = {u0}")]),
    ev("INT", "DELIVER").args(&[t("intr_id:0x{x0}({u0})")]),
    ev("PR_TH", "CREATE_P").args(&[d("ppid", 0), d("pid", 1)]),
    ev("PR_TH", "DESTROY_P").args(&[d("ppid", 0), d("pid", 1)]),
    ev("PR_TH", "CREATE_P_NAME").args(&[d("ppid", 0), d("pid", 1), sz("name", 2)]),
    ev("PR_TH", "DESTROY_P_NAME").args(&[d("ppid", 0), d("pid", 1), sz("name", 2)]),
    ev("PR_TH", "NAME_T").args(&[d("pid", 0), d("tid", 1), sz("name", 2)]),
    ev("COMM", "SMSG").args(&[h("rcvid", 0), d("pid", 1)]),
    ev("COMM", "RMSG").args(&[h("rcvid", 0), d("pid", 1)]),
    ev("COMM", "SPULSE").args(&[h("scoid", 0), d("pid", 1)]),
    ev("COMM", "RPULSE").args(&[h("scoid", 0), d("pid", 1)]),
    ev("COMM", "SPULSE_EXE").args(&[h("scoid", 0), d("pid", 1)]),
    ev("COMM", "SPULSE_DIS").args(&[h("scoid", 0), d("pid", 1)]),
    ev("COMM", "SPULSE_DEA").args(&[h("scoid", 0), d("pid", 1)]),
    ev("COMM", "SPULSE_UN").args(&[h("scoid", 0), d("pid", 1)]),
    ev("COMM", "SPULSE_QUN").args(&[h("scoid", 0), d("pid", 1)]),
    ev("COMM", "SIGNAL").args(&[
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
    ev("COMM", "REPLY").args(&[d("tid", 0), d("pid", 1)]),
    ev("COMM", "ERROR").args(&[d("tid", 0), d("pid", 1)]),
    ev("USER", "*").name("EVENT:{code}").args(&[dump()]),
    ev("QUIP", "*").name("EVENT:{code}").args(&[dump()]),
    ev("SEC", "ABLE").args(&[
        t(" pid:{d0} able:{u1} ({enum:sec_able:u1:UNKNOWN})"),
        t(" start:0x{q4:016}").when("words >= 6"),
        t(" end:0x{q6:016}").when("words >= 8"),
        t(" ALLOW").when("words >= 4 && u2 & 0x1"),
        t(" DENY").when("words >= 4 && u2 & 0x1 == 0"),
        t(" as root").when("words >= 4 && u2 & 0x2"),
        t(" as non-root").when("words >= 4 && u2 & 0x2 == 0"),
        t(" TRUNC").when("words < 8"),
    ]),
    ev("SEC", "ABLE_LOOKUP").args(&[t(" name:{r1} able:{u0}")]),
    ev("SEC", "PATH_ATTACH").args(&[
        t(" pid:{d0} ptype:{u1}"),
        t(" TRUNC").when("words < 4"),
        t(" ctype:{d2} chid:{d3} status:{enum:sec_path_status:u4:UNKNOWN}").when("words >= 4"),
        t(" TRUNC").when("words >= 4 && words < 6"),
        t(" path:{r5}").when("words >= 6"),
    ]),
    ev("SEC", "PERM_LOOKUP").args(&[t(" pid:{d0} class:{r4} ({d2}) perm:{rnext} ({d3}) errno:{d1}")]),
    ev("SEC", "PERM_TEST").args(&[
        t(" pid:{d0} ptype:{u1} otype:{u2} class:{d3} perm:{d4} errno:{u5}").when("words >= 6"),
        t(" pid:{d0} ptype:{u1} TRUNC").when("words < 6"),
    ]),
    ev("SEC", "UNREG_EVENT").args(&[
        t(" src pid:{d0} dst:{r4} (pid:{d1} tid:{u2}) denied:{u3}").when("words >= 6"),
        t(" TRUNC").when("words < 6"),
    ]),
    ev("SEC", "*").name("unknown({code})").args(&[]),
    ev("SYSTEM", "IPI").args_silent(),
    ev("SYSTEM", "IPI")
        .is64()
        .args(&[t(" ipicmd = {x0:#08}, "), flags(0, "ipi_cmd", " {name}")]),
    ev("SYSTEM", "TIMER").args(&[
        t(" pid:{d0} tid:{d1} timer_id:{d2} flags:0x{x3:08}"),
        flags(3, "timer_flags", " {name}"),
    ]),
    ev("SYSTEM", "IST_ATTACH").min_words(6).args(&[
        t("pid:{d0} tid:{d1} intr_id:0x{x2}({u2})"),
        t(" intr_cpu:{d3:02}").when("u3 != 0xffffffff"),
        t(" flags:0x{x4:08}"),
        t(" EVENT").when("u4 & 0x10000000"),
        t(" ist_prio:{d5}"),
    ]),
    ev("SYSTEM", "IST_DETACH").min_words(6).args(&[
        t("pid:{d0} tid:{d1} intr_id:0x{x2}({u2})"),
        t(" intr_cpu:{d3:02}").when("u3 != 0xffffffff"),
        t(" flags:0x{x4:08}"),
        t(" EVENT").when("u4 & 0x10000000"),
        t(" ist_prio:{d5}"),
    ]),
    ev("SYSTEM", "PATHMGR").wide(&[d("pid", 0), d("tid", 1), sz("path", 2)]),
    ev("SYSTEM", "MAPNAME").wide(&[d("pid", 0), h("vaddr", 1), h("len", 2), sz("name", 3)]),
    ev("SYSTEM", "MAPNAME_64").wide(&[d("pid", 0), q("vaddr", 1), e("len", 3), sz("name", 5)]),
    ev("SYSTEM", "SLOG").wide(&[d("opcode", 0), d("severity", 1), sz("message", 2)]),
    ev("SYSTEM", "PAGEWAIT").args(&[
        t(" pid:{d0} tid:{d1} ip:0x{x2:08} vaddr:0x{x3:08} type:{x4:04} --- flags:0x{x5:08} offset:0x{q6:016}"),
        t(" name:{z8:6}").when("words >= 9"),
    ]),
    ev("SYSTEM", "POWER").args(&[t(" bitset:0x{x0:08}\t actual frequency:{u1} cpu:{u0&0xffff}")]),
    ev("SYSTEM", "PROFILE").args(&[t(" ip = 0x{x0:08}, tid = 0x{x1:08}, pid = 0x{x2:08}")]),
    ev("SYSTEM", "PROFILE")
        .is64()
        .args(&[t(" ip = 0x{q0:016}, tid = 0x{x2:08}, pid = 0x{x3:08}")]),
    // Fast: dummy, empty
    // Wide: dummy, empty
    ev("KER_CALL", "NOP")
        .kind("ENTER")
        .fast(&[h("dummy", 0), h("empty", 1)])
        .wide(&[h("dummy", 0), h("empty", 1)]),
    ev("KER_CALL", "NOP").kind("ENTER").is64().fast_silent().wide_silent(),
    // Fast: empty, empty
    // Wide: empty, empty
    ev("KER_CALL", "NOP")
        .kind("EXIT")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "NOP").kind("EXIT").is64().fast_silent().wide_silent(),
    // Fast: mode, class
    // Wide: mode, class, event, data_1, data_2
    // Call: TraceEvent
    ev("KER_CALL", "TRACE_EVENT").kind("ENTER").fast_silent().wide_silent(),
    // Fast: mode, class
    // Wide: mode (64), class (64), event (64), data_1 (64), data_2 (64)
    // Call: TraceEvent
    ev("KER_CALL", "TRACE_EVENT")
        .kind("ENTER")
        .is64()
        .fast(&[q("cmd", 0)])
        .wide(&[
            q("cmd", 0),
            q("param1", 2),
            q("param2", 4),
            q("param3", 6),
            q("param4", 8),
        ]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: TraceEvent
    ev("KER_CALL", "TRACE_EVENT")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "TRACE_EVENT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: func_p, arg_p
    // Wide: func_p, arg_p
    // Call: __Ring0
    ev("KER_CALL", "RING0")
        .kind("ENTER")
        .fast(&[h("func_p", 0), h("arg_p", 1)])
        .wide(&[h("func_p", 0), h("arg_p", 1)]),
    // Fast: func_p (64)
    // Wide: func_p (64)
    // Call: __Ring0
    ev("KER_CALL", "RING0")
        .kind("ENTER")
        .is64()
        .fast(&[q("func_p", 0)])
        .wide(&[q("func_p", 0)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: __Ring0
    ev("KER_CALL", "RING0")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "RING0").kind("EXIT").is64().fast_silent().wide_silent(),
    ev("KER_CALL", "CACHE_CONTROL")
        .kind("ENTER")
        .fast_silent()
        .wide_silent(),
    // Fast: addr(64), len(64), flags
    // Wide: addr(64), len(64), flags
    ev("KER_CALL", "CACHE_CONTROL")
        .kind("ENTER")
        .is64()
        .fast(&[q("addr", 0)])
        .wide(&[q("addr", 0), q("len", 2), h("flags", 4)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: CacheControl
    ev("KER_CALL", "CACHE_CONTROL")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "CACHE_CONTROL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: N/A
    // Wide: N/A
    // Call: SysSrandom
    ev("KER_CALL", "SYS_SRANDOM")
        .kind("ENTER")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "SYS_SRANDOM")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "SYS_SRANDOM")
        .kind("EXIT")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "SYS_SRANDOM")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: empty, empty
    // Wide: empty, empty
    // Call: MsgRegisterEvent
    ev("KER_CALL", "MSG_REGISTER_EVENT")
        .kind("ENTER")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_REGISTER_EVENT")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: empty, empty
    // Wide: empty, empty
    // Call: MsgRegisterEvent
    ev("KER_CALL", "MSG_REGISTER_EVENT")
        .kind("EXIT")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_REGISTER_EVENT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: coid, code
    // Wide: coid, priority, code, value
    // Call: MsgSendPulse
    ev("KER_CALL", "MSG_SEND_PULSEPTR")
        .kind("ENTER")
        .fast(&[h("coid", 0), h("code", 1)])
        .wide(&[h("coid", 0), d("priority", 1), h("code", 2), h("value", 3)]),
    // Fast: coid, code
    // Wide: coid, priority, code, value (64)
    // Call: MsgSendPulse
    ev("KER_CALL", "MSG_SEND_PULSEPTR")
        .kind("ENTER")
        .is64()
        .fast(&[h("coid", 0), h("code", 1)])
        .wide(&[h("coid", 0), d("priority", 1), h("code", 2), q("value", 3)]),
    // Fast: status, empty
    // Wide: status, empty
    // Call: MsgSendPulse
    ev("KER_CALL", "MSG_SEND_PULSEPTR")
        .kind("EXIT")
        .fast(&[d("status", 0), h("empty", 1)])
        .wide(&[d("status", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_SEND_PULSEPTR")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: index, empty
    // Wide: index, empty
    ev("KER_CALL", "SYS_CPUPAGE_GET")
        .kind("ENTER")
        .fast(&[d("index", 0), h("empty", 1)])
        .wide(&[d("index", 0), h("empty", 1)]),
    ev("KER_CALL", "SYS_CPUPAGE_GET")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    ev("KER_CALL", "SYS_CPUPAGE_GET")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYS_CPUPAGE_GET")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: index, value
    // Wide: index, value
    ev("KER_CALL", "SYS_CPUPAGE_SET")
        .kind("ENTER")
        .fast(&[d("index", 0), h("value", 1)])
        .wide(&[d("index", 0), h("value", 1)]),
    ev("KER_CALL", "SYS_CPUPAGE_SET")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    ev("KER_CALL", "SYS_CPUPAGE_SET")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYS_CPUPAGE_SET")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid, cookie
    // Wide: rcvid, cookie
    // Call: MsgPause
    ev("KER_CALL", "MSG_PAUSE")
        .kind("ENTER")
        .fast(&[h("rcvid", 0), h("cookie", 1)])
        .wide(&[h("rcvid", 0), h("cookie", 1)]),
    ev("KER_CALL", "MSG_PAUSE")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: empty, empty
    // Wide: empty, empty
    // Call: MsgPause
    ev("KER_CALL", "MSG_PAUSE").kind("EXIT").intcall().fast(&[]),
    ev("KER_CALL", "MSG_PAUSE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid, empty
    // Wide: rcvid, empty
    // Call: MsgCurrent
    ev("KER_CALL", "MSG_CURRENT")
        .kind("ENTER")
        .fast(&[h("rcvid", 0), h("empty", 1)])
        .wide(&[h("rcvid", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_CURRENT")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: empty, empty
    // Wide: empty, empty
    // Call: MsgCurrent
    ev("KER_CALL", "MSG_CURRENT")
        .kind("EXIT")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_CURRENT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: coid, msg[0]
    // Wide: coid, sparts, rparts, msg[0], msg[1], msg[2]
    // Call: MsgSend,MsgSendv,MsgSendvs
    ev("KER_CALL", "MSG_SENDV")
        .kind("ENTER")
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), d("sparts", 1), d("rparts", 2), s("msg[0]", 3, 3)]),
    // Fast: coid, msg[0]
    // Wide: coid, sparts (64), rparts (64), msg[0], msg[1], msg[2]
    // Call: MsgSend,MsgSendv,MsgSendvs
    ev("KER_CALL", "MSG_SENDV")
        .kind("ENTER")
        .is64()
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), e("sparts", 1), e("rparts", 3), s("msg[0]", 5, 3)]),
    // Fast: status, rmsg[0]
    // Wide: status, rmsg[0], rmsg[1], rmsg[2]
    // Call: MsgSend,MsgSendv,MsgSendvs
    ev("KER_CALL", "MSG_SENDV")
        .kind("EXIT")
        .fast(&[d("status", 0), s("rmsg[0]", 1, 1)])
        .wide(&[d("status", 0), s("rmsg[0]", 1, 3)]),
    // Fast: status (64)
    // Wide: status (64), rmsg[0], rmsg[1], rmsg[2]
    // Call: MsgSend,MsgSendv,MsgSendvs
    ev("KER_CALL", "MSG_SENDV")
        .kind("EXIT")
        .is64()
        .fast(&[e("status", 0)])
        .wide(&[e("status", 0), s("rmsg[0]", 2, 3)]),
    ev("KER_CALL", "MSG_SENDV").kind("INT").intcall(),
    ev("KER_CALL", "MSG_SENDV").kind("INT").is64().wide_silent(),
    // Fast: coid, msg[0]
    // Wide: coid, sparts, rparts, msg[0], msg[1], msg[2]
    // Call: MsgSendnc,MsgSendvnc,MsgSendvsnc
    ev("KER_CALL", "MSG_SENDVNC")
        .kind("ENTER")
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), d("sparts", 1), d("rparts", 2), s("msg[0]", 3, 3)]),
    // Fast: coid, msg[0]
    // Wide: coid, sparts (64), rparts (64), msg[0], msg[1], msg[2]
    // Call: MsgSendnc,MsgSendvnc,MsgSendvsnc
    ev("KER_CALL", "MSG_SENDVNC")
        .kind("ENTER")
        .is64()
        .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
        .wide(&[h("coid", 0), e("sparts", 1), e("rparts", 3), s("msg[0]", 5, 3)]),
    // Fast: ret_val, rmsg[0]
    // Wide: ret_val, rmsg[0], rmsg[1], rmsg[2]
    // Call: MsgSendnc,MsgSendvnc,MsgSendvsnc
    ev("KER_CALL", "MSG_SENDVNC")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), s("rmsg[0]", 1, 1)])
        .wide(&[h("ret_val", 0), s("rmsg[0]", 1, 3)]),
    // Fast: ret_val (64)
    // Wide: ret_val (64), rmsg[0], rmsg[1], rmsg[2]
    // Call: MsgSendnc,MsgSendvnc,MsgSendvsnc
    ev("KER_CALL", "MSG_SENDVNC")
        .kind("EXIT")
        .is64()
        .fast(&[q("ret_val", 0)])
        .wide(&[q("ret_val", 0), s("rmsg[0]", 2, 3)]),
    // Fast: rcvid, err
    // Wide: rcvid, err
    // Call: MsgError
    ev("KER_CALL", "MSG_ERROR")
        .kind("ENTER")
        .fast(&[h("rcvid", 0), d("err", 1)])
        .wide(&[h("rcvid", 0), d("err", 1)]),
    ev("KER_CALL", "MSG_ERROR")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: MsgError
    ev("KER_CALL", "MSG_ERROR")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_ERROR")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: chid, rparts
    // Wide: chid, rparts
    // Call: MsgReceive,MsgReceivev
    ev("KER_CALL", "MSG_RECEIVEV")
        .kind("ENTER")
        .fast(&[h("chid", 0), d("rparts", 1)])
        .wide(&[h("chid", 0), d("rparts", 1)]),
    // Fast: N/A
    // Wide: chid, rparts (64)
    // Call: MsgReceive,MsgReceivev
    ev("KER_CALL", "MSG_RECEIVEV")
        .kind("ENTER")
        .is64()
        .fast(&[])
        .wide(&[h("chid", 0), e("rparts", 1)]),
    // Fast: rcvid, rmsg[0]
    // Wide: rcvid, rmsg[0], rmsg[1], rmsg[2], empty, empty, info->pid, info->tid, info->chid, info->scoid, info->coid, info->msglen, info->srcmsglen, info->dstmsglen, info->priority, info->flags, info->reserved
    // Call: MsgReceive,MsgReceivev
    ev("KER_CALL", "MSG_RECEIVEV")
        .kind("EXIT")
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
    // Fast: rcvid, rmsg[0]
    // Wide: rcvid, rmsg[0], rmsg[1], rmsg[2], empty, empty, info->pid, info->tid, info->chid, info->scoid, info->coid, info->msglen (64), info->srcmsglen (64), info->dstmsglen (64), info->priority, info->flags, info->reserved[0], info->reserved[1]
    // Call: MsgReceive,MsgReceivev
    ev("KER_CALL", "MSG_RECEIVEV")
        .kind("EXIT")
        .is64()
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
    // Fast: rcvid, status
    // Wide: rcvid, sparts, status, smsg[0], smsg[1], smsg[2]
    // Call: MsgReply,MsgReplyv
    ev("KER_CALL", "MSG_REPLYV")
        .kind("ENTER")
        .fast(&[h("rcvid", 0), h("status", 1)])
        .wide(&[h("rcvid", 0), d("sparts", 1), h("status", 2), s("smsg[0]", 3, 3)]),
    // Fast: N/A
    // Wide: rcvid, sparts (64), status (64), smsg[0], smsg[1], smsg[2]
    // Call: MsgReply,MsgReplyv
    ev("KER_CALL", "MSG_REPLYV").kind("ENTER").is64().fast(&[]).wide(&[
        h("rcvid", 0),
        e("sparts", 1),
        q("status", 3),
        s("smsg[0]", 5, 3),
    ]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: MsgReply,MsgReplyv
    ev("KER_CALL", "MSG_REPLYV")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_REPLYV")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid, offset
    // Wide: rcvid, rmsg_p, rparts, offset
    // Call: MsgRead,MsgReadv
    ev("KER_CALL", "MSG_READV")
        .kind("ENTER")
        .fast(&[h("rcvid", 0), h("offset", 1)])
        .wide(&[h("rcvid", 0), h("rmsg_p", 1), d("rparts", 2), h("offset", 3)]),
    // Fast: rcvid, empty
    // Wide: rcvid, rmsg_p (64), rparts (64), offset (64)
    // Call: MsgRead,MsgReadv
    ev("KER_CALL", "MSG_READV")
        .kind("ENTER")
        .is64()
        .fast(&[h("rcvid", 0)])
        .wide(&[h("rcvid", 0), q("rmsg_p", 1), e("rparts", 3), q("offset", 5)]),
    // Fast: rbytes, rmsg[0]
    // Wide: rbytes, rmsg[0], rmsg[1], rmsg[2]
    // Call: MsgRead,MsgReadv
    ev("KER_CALL", "MSG_READV")
        .kind("EXIT")
        .fast(&[d("rbytes", 0), s("rmsg[0]", 1, 1)])
        .wide(&[d("rbytes", 0), s("rmsg[0]", 1, 3)]),
    // Fast: rbytes (64)
    // Wide: rbytes (64), rmsg[0], rmsg[1], rmsg[2]
    // Call: MsgRead,MsgReadv
    ev("KER_CALL", "MSG_READV")
        .kind("EXIT")
        .is64()
        .fast(&[e("rbytes", 0)])
        .wide(&[e("rbytes", 0), s("rmsg[0]", 2, 3)]),
    // Fast: rcvid, offset
    // Wide: rcvid, sparts, offset, msg[0], msg[1], msg[2]
    // Call: MsgWrite,MsgWritev
    ev("KER_CALL", "MSG_WRITEV")
        .kind("ENTER")
        .fast(&[h("rcvid", 0), h("offset", 1)])
        .wide(&[h("rcvid", 0), d("sparts", 1), h("offset", 2), s("msg[0]", 3, 3)]),
    // Fast: rcvid, empty
    // Wide: rcvid, sparts (64), offset (64), msg[0], msg[1], msg[2]
    // Call: MsgWrite,MsgWritev
    ev("KER_CALL", "MSG_WRITEV")
        .kind("ENTER")
        .is64()
        .fast(&[h("rcvid", 0)])
        .wide(&[h("rcvid", 0), e("sparts", 1), q("offset", 3), s("msg[0]", 5, 3)]),
    // Fast: wbytes, empty
    // Wide: wbytes, empty
    // Call: MsgWrite,MsgWritev
    ev("KER_CALL", "MSG_WRITEV")
        .kind("EXIT")
        .fast(&[d("wbytes", 0), h("empty", 1)])
        .wide(&[d("wbytes", 0), h("empty", 1)]),
    // Fast: wbytes (64)
    // Wide: wbytes (64)
    // Call: MsgWrite,MsgWritev
    ev("KER_CALL", "MSG_WRITEV")
        .kind("EXIT")
        .is64()
        .fast(&[e("wbytes", 0)])
        .wide(&[e("wbytes", 0)]),
    // Fast: rcvid, info_p
    // Wide: rcvid, info_p
    // Call: MsgInfo
    ev("KER_CALL", "MSG_INFO")
        .kind("ENTER")
        .fast(&[h("rcvid", 0), h("info_p", 1)])
        .wide(&[h("rcvid", 0), h("info_p", 1)]),
    // Fast: rcvid, empty
    // Wide: rcvid, empty
    // Call: MsgInfo
    ev("KER_CALL", "MSG_INFO")
        .kind("ENTER")
        .is64()
        .fast(&[h("rcvid", 0), h("empty", 1)])
        .wide(&[h("rcvid", 0), h("empty", 1)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty, empty, info->pid, info->tid, info->chid, info->scoid, info->coid, info->msglen, info->srcmsglen, info->dstmsglen, info->priority, info->flags, info->reserved
    // Call: MsgInfo
    ev("KER_CALL", "MSG_INFO")
        .kind("EXIT")
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
    // Fast: ret_val, empty
    // Wide: ret_val, empty, empty, info->pid, info->tid, info->chid, info->scoid, info->coid, info->msglen (64), info->srcmsglen (64), info->dstmsglen (64), info->priority, info->flags, info->reserved[0], info->reserved[1]
    // Call: MsgInfo
    ev("KER_CALL", "MSG_INFO")
        .kind("EXIT")
        .is64()
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
    // Fast: coid, code
    // Wide: coid, priority, code, value
    // Call: MsgSendPulse
    ev("KER_CALL", "MSG_SEND_PULSE")
        .kind("ENTER")
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "MSG_SEND_PULSE")
        .kind("ENTER")
        .is64()
        .fast(&[h("coid", 0), h("code", 1)])
        .wide(&[h("coid", 0), d("priority", 1), h("code", 2), h("value", 3)]),
    // Fast: status, empty
    // Wide: status, empty
    // Call: MsgSendPulse
    ev("KER_CALL", "MSG_SEND_PULSE")
        .kind("EXIT")
        .fast(&[d("status", 0), h("empty", 1)])
        .wide(&[d("status", 0), h("empty", 1)]),
    ev("KER_CALL", "MSG_SEND_PULSE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid, event->sigev_notify
    // Wide: rcvid, event->sigev_notify, event->sigev_notify_function_p, event->sigev_value, event->sigev_notify_attributes_p
    // Call: MsgDeliverEvent
    ev("KER_CALL", "MSG_DELIVER_EVENT")
        .kind("ENTER")
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid, event->sigev_notify
    // Wide: rcvid, event->sigev_notify, pad, event->sigev_notify_function_p (64), event->sigev_value (64), event->sigev_notify_attributes_p (64)
    // Call: MsgDeliverEvent
    ev("KER_CALL", "MSG_DELIVER_EVENT")
        .kind("ENTER")
        .is64()
        .fast(&[h("rcvid", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("rcvid", 0),
            d("event->sigev_notify", 1),
            h("pad", 2),
            q("event->sigev_notify_function_p", 3),
            q("event->sigev_value", 5),
            q("event->sigev_notify_attributes_p", 7),
        ]),
    // Fast: ret_val, eventp
    // Wide: ret_val, eventp
    // Call: MsgDeliverEvent
    ev("KER_CALL", "MSG_DELIVER_EVENT")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("event", 1)])
        .wide(&[h("ret_val", 0), h("event", 1)]),
    // Fast: N/A
    // Wide: ret_val, eventp (64)
    // Call: MsgDeliverEvent
    ev("KER_CALL", "MSG_DELIVER_EVENT")
        .kind("EXIT")
        .is64()
        .fast(&[])
        .wide(&[h("ret_val", 0), q("event", 1)]),
    ev("KER_CALL", "MSG_RECEIVEPULSEV")
        .kind("ENTER")
        .fast(&[h("chid", 0), d("rparts", 1)])
        .wide(&[h("chid", 0), d("rparts", 1)]),
    ev("KER_CALL", "MSG_RECEIVEPULSEV")
        .kind("ENTER")
        .is64()
        .fast(&[])
        .wide(&[h("chid", 0), e("rparts", 1)]),
    ev("KER_CALL", "MSG_RECEIVEPULSEV")
        .kind("EXIT")
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
    ev("KER_CALL", "MSG_RECEIVEPULSEV")
        .kind("EXIT")
        .is64()
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
    ev("KER_CALL", "MSG_VERIFY_EVENT")
        .kind("ENTER")
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid, event->sigev_notify
    // Wide: rcvid, event->sigev_notify, pad, event->sigev_notify_function_p (64), event->sigev_value (64), event->sigev_notify_attribute_p (64)
    // Call: MsgVerifyEvent
    ev("KER_CALL", "MSG_VERIFY_EVENT")
        .kind("ENTER")
        .is64()
        .fast(&[h("rcvid", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("rcvid", 0),
            d("event->sigev_notify", 1),
            h("pad", 2),
            q("event->sigev_notify_function_p", 3),
            q("event->sigev_value", 5),
            q("event->sigev_notify_attributes_p", 7),
        ]),
    ev("KER_CALL", "MSG_VERIFY_EVENT")
        .kind("EXIT")
        .fast(&[d("status", 0), h("empty", 1)])
        .wide(&[d("status", 0), h("empty", 1)]),
    // Fast: status, empty
    // Wide: status, empty
    // Call: MsgVerifyEvent
    ev("KER_CALL", "MSG_VERIFY_EVENT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, signo
    // Wide: empty, pid, tid, signo, code, value
    // Call: SignalKill
    ev("KER_CALL", "SIGNAL_KILL")
        .kind("ENTER")
        .fast(&[d("pid", 0), d("signo", 1)])
        .wide(&[
            h("nd", 0),
            d("pid", 1),
            d("tid", 2),
            d("signo", 3),
            h("code", 4),
            h("value", 5),
        ]),
    ev("KER_CALL", "SIGNAL_KILL")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SignalKill
    ev("KER_CALL", "SIGNAL_KILL")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SIGNAL_KILL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: s_p, empty
    // Wide: s_p, empty
    // Call: SignalReturn
    ev("KER_CALL", "SIGNAL_RETURN")
        .kind("ENTER")
        .fast(&[h("s_p", 0), h("empty", 1)])
        .wide(&[h("s_p", 0), h("empty", 1)]),
    // Fast: s_p (64)
    // Wide: s_p (64)
    // Call: SignalReturn
    ev("KER_CALL", "SIGNAL_RETURN")
        .kind("ENTER")
        .is64()
        .fast(&[q("s_p", 0)])
        .wide(&[q("s_p", 0)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SignalReturn
    ev("KER_CALL", "SIGNAL_RETURN")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SIGNAL_RETURN")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: signo, act->sa_handler_p
    // Wide: pid, sigstub_p, signo, act->sa_handler_p, act->sa_flags, act->sa_mask.bits[0], act->sa_mask.bits[1]
    // Call: SignalAction
    ev("KER_CALL", "SIGNAL_ACTION")
        .kind("ENTER")
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
    // Fast: signo
    // Wide: pid, sigstub_p (64), signo, act->sa_handler_p (64), act->sa_flags, act->sa_mask.bits[0], act->sa_mask.bits[1]
    // Call: SignalAction
    ev("KER_CALL", "SIGNAL_ACTION")
        .kind("ENTER")
        .is64()
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
    // Fast: ret_val, act->sa_handler_p
    // Wide: ret_val, act->sa_handler_p, act->sa_flags, act->sa_mask.bits[0], act->sa_mask.bits[1]
    // Call: SignalAction
    ev("KER_CALL", "SIGNAL_ACTION")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("oact->sa_handler_p", 1)])
        .wide(&[
            h("ret_val", 0),
            h("oact->sa_handler_p", 1),
            h("oact->sa_flags", 2),
            h("oact->sa_mask.bits[0]", 3),
            h("oact->sa_mask.bits[1]", 4),
        ]),
    // Fast: ret_val
    // Wide: ret_val, act->sa_handler_p (64), act->sa_flags, act->sa_mask.bits[0], act->sa_mask.bits[1]
    // Call: SignalAction
    ev("KER_CALL", "SIGNAL_ACTION")
        .kind("EXIT")
        .is64()
        .fast(&[h("ret_val", 0)])
        .wide(&[
            h("ret_val", 0),
            q("oact->sa_handler_p", 1),
            h("oact->sa_flags", 3),
            h("oact->sa_mask.bits[0]", 4),
            h("oact->sa_mask.bits[1]", 5),
        ]),
    // Fast: pid, tid
    // Wide: pid, tid, how, sig_blocked->bits[0], sig_blocked->bits[1]
    // Call: SignalProcmask
    ev("KER_CALL", "SIGNAL_PROCMASK")
        .kind("ENTER")
        .fast(&[d("pid", 0), d("tid", 1)])
        .wide(&[
            d("pid", 0),
            d("tid", 1),
            h("how", 2),
            h("sig_blocked->bits[0]", 3),
            h("sig_blocked->bits[1]", 4),
        ]),
    ev("KER_CALL", "SIGNAL_PROCMASK")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, sig_blocked->bits[0]
    // Wide: ret_val, sig_blocked->bits[0], sig_blocked->bits[1]
    // Call: SignalProcmask
    ev("KER_CALL", "SIGNAL_PROCMASK")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("old_sig_blocked->bits[0]", 1)])
        .wide(&[
            d("ret_val", 0),
            h("old_sig_blocked->bits[0]", 1),
            h("old_sig_blocked->bits[1]", 2),
        ]),
    ev("KER_CALL", "SIGNAL_PROCMASK")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sig_blocked->bits[0], sig_blocked->bits[1]
    // Wide: sig_blocked->bits[0], sig_blocked->bits[1]
    // Call: SignalSuspend
    ev("KER_CALL", "SIGNAL_SUSPEND")
        .kind("ENTER")
        .fast(&[h("sig_blocked->bits[0]", 0), h("sig_blocked->bits[1]", 1)])
        .wide(&[h("sig_blocked->bits[0]", 0), h("sig_blocked->bits[1]", 1)]),
    ev("KER_CALL", "SIGNAL_SUSPEND")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, sig_blocked_p
    // Wide: ret_val, sig_blocked_p
    // Call: SignalSuspend
    ev("KER_CALL", "SIGNAL_SUSPEND")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("sig_blocked_p", 1)])
        .wide(&[h("ret_val", 0), h("sig_blocked_p", 1)]),
    // Fast: N/A
    // Wide: ret_val, sig_blocked_p (64)
    // Call: SignalSuspend
    ev("KER_CALL", "SIGNAL_SUSPEND")
        .kind("EXIT")
        .is64()
        .fast(&[])
        .wide(&[h("ret_val", 0), q("sig_blocked_p", 1)]),
    // Fast: sig_wait->bits[0], sig_wait->bits[1]
    // Wide: sig_wait->bits[0], sig_wait->bits[1]
    // Call: SignalWaitinfo
    ev("KER_CALL", "SIGNAL_WAITINFO")
        .kind("ENTER")
        .fast(&[h("sig_wait->bits[0]", 0), h("sig_wait->bits[1]", 1)])
        .wide(&[h("sig_wait->bits[0]", 0), h("sig_wait->bits[1]", 1)]),
    ev("KER_CALL", "SIGNAL_WAITINFO")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sig_num, si_code
    // Wide: sig_num, si_signo, si_code, si_errno, p[0], p[1], p[2], p[3], p[4], p[5], p[6]
    // Call: SignalWaitinfo
    ev("KER_CALL", "SIGNAL_WAITINFO")
        .kind("EXIT")
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
    ev("KER_CALL", "SIGNAL_WAITINFO")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, signo
    // Wide: empty, pid, tid, signo, code, value
    // Call: SignalKill
    ev("KER_CALL", "SIGNAL_KILL_SIGVAL")
        .kind("ENTER")
        .fast(&[d("pid", 0), d("signo", 1)])
        .wide(&[
            h("nd", 0),
            d("pid", 1),
            d("tid", 2),
            d("signo", 3),
            h("code", 4),
            h("value", 5),
        ]),
    // Fast: pid, signo
    // Wide: empty, pid, tid, signo, code, value (64)
    // Call: SignalKillSigval
    ev("KER_CALL", "SIGNAL_KILL_SIGVAL")
        .kind("ENTER")
        .is64()
        .fast(&[d("pid", 0), d("signo", 1)])
        .wide(&[
            h("nd", 0),
            d("pid", 1),
            d("tid", 2),
            d("signo", 3),
            h("code", 4),
            q("value", 5),
        ]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SignalKillSigval
    ev("KER_CALL", "SIGNAL_KILL_SIGVAL")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SIGNAL_KILL_SIGVAL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: flags, empty
    // Wide: flags, empty
    // Call: ChannelCreate
    ev("KER_CALL", "CHANNEL_CREATE")
        .kind("ENTER")
        .fast(&[h("flags", 0), h("empty", 1)])
        .wide(&[h("flags", 0), h("empty", 1)]),
    ev("KER_CALL", "CHANNEL_CREATE")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: chid, empty
    // Wide: chid, empty
    // Call: ChannelCreate
    ev("KER_CALL", "CHANNEL_CREATE")
        .kind("EXIT")
        .fast(&[h("chid", 0), h("empty", 1)])
        .wide(&[h("chid", 0), h("empty", 1)]),
    ev("KER_CALL", "CHANNEL_CREATE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: chid, empty
    // Wide: chid, empty
    // Call: ChannelDestroy
    ev("KER_CALL", "CHANNEL_DESTROY")
        .kind("ENTER")
        .fast(&[h("chid", 0), h("empty", 1)])
        .wide(&[h("chid", 0), h("empty", 1)]),
    ev("KER_CALL", "CHANNEL_DESTROY")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ChannelDestroy
    ev("KER_CALL", "CHANNEL_DESTROY")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "CHANNEL_DESTROY")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: chid, cmd
    // Wide: chid, cmd
    // Call: ChannelCtl
    ev("KER_CALL", "CHANNEL_CTL")
        .kind("ENTER")
        .fast(&[d("chid", 0), d("cmd", 1)])
        .wide(&[d("chid", 0), d("cmd", 1)]),
    ev("KER_CALL", "CHANNEL_CTL")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ChannelCtl
    ev("KER_CALL", "CHANNEL_CTL")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "CHANNEL_CTL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: empty, pid
    // Wide: empty, pid, chid, index, flags
    // Call: ConnectAttach
    ev("KER_CALL", "CONNECT_ATTACH")
        .kind("ENTER")
        .fast(&[h("nd", 0), d("pid", 1)])
        .wide(&[h("nd", 0), d("pid", 1), h("chid", 2), d("index", 3), h("flags", 4)]),
    ev("KER_CALL", "CONNECT_ATTACH")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: coid, empty
    // Wide: coid, empty
    // Call: ConnectAttach
    ev("KER_CALL", "CONNECT_ATTACH")
        .kind("EXIT")
        .fast(&[h("coid", 0), h("empty", 1)])
        .wide(&[h("coid", 0), h("empty", 1)]),
    ev("KER_CALL", "CONNECT_ATTACH")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: coid, empty
    // Wide: coid, empty
    // Call: ConnectDetach
    ev("KER_CALL", "CONNECT_DETACH")
        .kind("ENTER")
        .fast(&[h("coid", 0), h("empty", 1)])
        .wide(&[h("coid", 0), h("empty", 1)]),
    ev("KER_CALL", "CONNECT_DETACH")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ConnectDetach
    ev("KER_CALL", "CONNECT_DETACH")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "CONNECT_DETACH")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, coid
    // Wide: pid, coid
    // Call: ConnectServerInfo
    ev("KER_CALL", "CONNECT_SERVER_INFO")
        .kind("ENTER")
        .fast(&[d("pid", 0), h("coid", 1)])
        .wide(&[d("pid", 0), h("coid", 1)]),
    ev("KER_CALL", "CONNECT_SERVER_INFO")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: coid, empty
    // Wide: coid, empty, empty, info->pid, info->tid, info->chid, info->scoid, info->coid, info->msglen, info->srcmsglen, info->dstmsglen, info->priority, info->flags, info->reserved
    // Call: ConnectServerInfo
    ev("KER_CALL", "CONNECT_SERVER_INFO")
        .kind("EXIT")
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
    // Fast: coid, empty
    // Wide: coid, empty, empty, info->pid, info->tid, info->chid, info->scoid, info->coid, info->msglen (64), info->srcmsglen (64), info->dstmsglen (64), info->priority, info->flags, info->reserved[0], info->reserved[1]
    // Call: ConnectServerInfo
    ev("KER_CALL", "CONNECT_SERVER_INFO")
        .kind("EXIT")
        .is64()
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
    // Fast: scoid, ngroups
    // Wide: scoid, ngroups
    // Call: ConnectClientInfo
    ev("KER_CALL", "CONNECT_CLIENT_INFO")
        .kind("ENTER")
        .fast(&[h("scoid", 0), d("ngroups", 1)])
        .wide(&[h("scoid", 0), d("ngroups", 1)]),
    ev("KER_CALL", "CONNECT_CLIENT_INFO")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty, info->pid, info->sid, flags, info->ruid, info->euid, info->suid, info->rgid, info->egid, info->sgid, info->ngroups, info->grouplist[0], info->grouplist[1], info->grouplist[2], info->grouplist[3], info->grouplist[4], info->grouplist[5], info->grouplist[6], info->grouplist[7]
    // Call: ConnectClientInfo
    ev("KER_CALL", "CONNECT_CLIENT_INFO")
        .kind("EXIT")
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
    ev("KER_CALL", "CONNECT_CLIENT_INFO")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: coid, bits
    // Wide: pid, coid, masks, bits
    // Call: ConnectFlags
    ev("KER_CALL", "CONNECT_FLAGS")
        .kind("ENTER")
        .fast(&[h("coid", 0), h("bits", 1)])
        .wide(&[d("pid", 0), h("coid", 1), h("masks", 2), h("bits", 3)]),
    ev("KER_CALL", "CONNECT_FLAGS")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: old_flags, empty
    // Wide: old_flags, empty
    // Call: ConnectFlags
    ev("KER_CALL", "CONNECT_FLAGS")
        .kind("EXIT")
        .fast(&[h("old_flags", 0), h("empty", 1)])
        .wide(&[h("old_flags", 0), h("empty", 1)]),
    ev("KER_CALL", "CONNECT_FLAGS")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "CONNECT_DUPFD")
        .kind("ENTER")
        .fast(&[u("op", 0)])
        .wide(&[u("op", 0)]),
    ev("KER_CALL", "CONNECT_DUPFD")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "CONNECT_DUPFD")
        .kind("EXIT")
        .fast(&[d("result", 0)])
        .wide(&[d("result", 0)]),
    ev("KER_CALL", "CONNECT_DUPFD")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: func_p, arg_p
    // Wide: pid, func_p, arg_p, flags, stacksize, stackaddr_p, exitfunc_p, policy, sched_priority, sched_curpriority, param.ss_low_priority, param.ss_max_repl, param.ss_repl_period.tv_sec, param.ss_repl_period.tv_nsec, param.ss_init_budget.tv_sec, param.ss_init_budget.tv_nsec, guardsize, empty, empty, empty
    // Call: ThreadCreate
    ev("KER_CALL", "THREAD_CREATE")
        .kind("ENTER")
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
    // Fast: func_p (64)
    // Wide: pid, func_p (64), arg_p (64), flags, pad, stacksize (64), stackaddr_p (64), exitfunc_p (64), policy, pad, sched_priority, sched_curpriority, param.ss_low_priority, param.ss_max_repl, param.ss_repl_period.tv_sec (64), param.ss_repl_period.tv_nsec (64), param.ss_init_budget.tv_sec (64), param.ss_init_budget.tv_nsec (64), guardsize, prealloc, empty, empty
    // Call: ThreadCreate
    ev("KER_CALL", "THREAD_CREATE")
        .kind("ENTER")
        .is64()
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
    // Fast: thread_id, owner
    // Wide: thread_id, owner
    // Call: ThreadCreate
    ev("KER_CALL", "THREAD_CREATE")
        .kind("EXIT")
        .fast(&[h("thread_id", 0), h("owner", 1)])
        .wide(&[h("thread_id", 0), h("owner", 1)]),
    ev("KER_CALL", "THREAD_CREATE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: tid, status_p
    // Wide: tid, priority, status_p
    // Call: ThreadDestroy
    ev("KER_CALL", "THREAD_DESTROY")
        .kind("ENTER")
        .fast(&[d("tid", 0), h("status_p", 1)])
        .wide(&[d("tid", 0), d("priority", 1), h("status_p", 2)]),
    // Fast: N/A
    // Wide: tid, priority, status_p (64)
    // Call: ThreadDestroy
    ev("KER_CALL", "THREAD_DESTROY").kind("ENTER").is64().fast(&[]).wide(&[
        d("tid", 0),
        d("priority", 1),
        q("status_p", 2),
    ]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ThreadDestroy
    ev("KER_CALL", "THREAD_DESTROY")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "THREAD_DESTROY")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: tid, empty
    // Wide: tid, empty
    // Call: ThreadDetach
    ev("KER_CALL", "THREAD_DETACH")
        .kind("ENTER")
        .fast(&[d("tid", 0), h("empty", 1)])
        .wide(&[d("tid", 0), h("empty", 1)]),
    ev("KER_CALL", "THREAD_DETACH")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ThreadDetach
    ev("KER_CALL", "THREAD_DETACH")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "THREAD_DETACH")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: tid, status_p
    // Wide: tid, status_p
    // Call: ThreadJoin
    ev("KER_CALL", "THREAD_JOIN")
        .kind("ENTER")
        .fast(&[d("tid", 0), h("status_p", 1)])
        .wide(&[d("tid", 0), h("status_p", 1)]),
    // Fast: N/A
    // Wide: tid, status_p (64)
    // Call: ThreadJoin
    ev("KER_CALL", "THREAD_JOIN")
        .kind("ENTER")
        .is64()
        .fast(&[])
        .wide(&[d("tid", 0), q("status_p", 1)]),
    // Fast: ret_val, status_p
    // Wide: ret_val, status_p
    // Call: ThreadJoin
    ev("KER_CALL", "THREAD_JOIN")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("status_p", 1)])
        .wide(&[h("ret_val", 0), h("status_p", 1)]),
    // Fast: N/A
    // Wide: ret_val, status_p (64)
    // Call: ThreadJoin
    ev("KER_CALL", "THREAD_JOIN")
        .kind("EXIT")
        .is64()
        .fast(&[])
        .wide(&[h("ret_val", 0), q("status_p", 1)]),
    // Fast: tid, canstub_p
    // Wide: tid, canstub_p
    // Call: ThreadCancel
    ev("KER_CALL", "THREAD_CANCEL")
        .kind("ENTER")
        .fast(&[d("tid", 0), h("canstub_p", 1)])
        .wide(&[d("tid", 0), h("canstub_p", 1)]),
    // Fast: N/A
    // Wide: tid, canstub_p (64)
    // Call: ThreadCancel
    ev("KER_CALL", "THREAD_CANCEL")
        .kind("ENTER")
        .is64()
        .fast(&[])
        .wide(&[d("tid", 0), q("canstub_p", 1)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ThreadCancel
    ev("KER_CALL", "THREAD_CANCEL")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "THREAD_CANCEL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: cmd, data_p
    // Wide: cmd, data_p
    // Call: ThreadCtl
    ev("KER_CALL", "THREAD_CTL")
        .kind("ENTER")
        .fast(&[h("cmd", 0), h("data_p", 1)])
        .wide(&[h("cmd", 0), h("data_p", 1)]),
    // Fast: N/A
    // Wide: cmd, data_p (64)
    // Call: ThreadCtl
    ev("KER_CALL", "THREAD_CTL")
        .kind("ENTER")
        .is64()
        .fast(&[])
        .wide(&[h("cmd", 0), q("data_p", 1)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ThreadCtl
    ev("KER_CALL", "THREAD_CTL")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "THREAD_CTL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, tid
    // Wide: pid, tid, cmd, data_p
    // Call: ThreadCtlExt
    ev("KER_CALL", "THREAD_CTLEXT")
        .kind("ENTER")
        .fast(&[h("pid", 0), h("tid", 1)])
        .wide(&[h("pid", 0), h("tid", 1), h("cmd", 2), h("data_p", 3)]),
    // Fast: pid, tid
    // Wide: pid, tid, cmd, data_p (64)
    // Call: ThreadCtlExt
    ev("KER_CALL", "THREAD_CTLEXT")
        .kind("ENTER")
        .is64()
        .fast(&[h("pid", 0), h("tid", 1)])
        .wide(&[h("pid", 0), h("tid", 1), h("cmd", 2), q("data_p", 3)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ThreadCtlExt
    ev("KER_CALL", "THREAD_CTLEXT")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "THREAD_CTLEXT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: intr, flags
    // Wide: intr, handler_p, area_p, areasize, flags
    // Call: InterruptAttachThread, InterruptAttachEvent
    ev("KER_CALL", "INTERRUPT_ATTACH")
        .kind("ENTER")
        .fast(&[d("intr", 0), h("flags", 1)])
        .wide(&[
            d("intr", 0),
            h("handler_p", 1),
            h("area_p", 2),
            d("areasize", 3),
            h("flags", 4),
        ]),
    // Fast: intr, flags
    // Wide: intr, handler_p (64), area_p (64), areasize, flags
    // Call: InterruptAttachThread, InterruptAttachEvent
    ev("KER_CALL", "INTERRUPT_ATTACH")
        .kind("ENTER")
        .is64()
        .fast(&[d("intr", 0), h("flags", 1)])
        .wide(&[
            d("intr", 0),
            q("handler_p", 1),
            q("area_p", 3),
            d("areasize", 5),
            h("flags", 6),
        ]),
    // Fast: int_fun_id, empty
    // Wide: int_fun_id, empty
    // Call: InterruptAttachThread, InterruptAttachEvent
    ev("KER_CALL", "INTERRUPT_ATTACH")
        .kind("EXIT")
        .fast(&[h("int_fun_id", 0), h("empty", 1)])
        .wide(&[h("int_fun_id", 0), h("empty", 1)]),
    ev("KER_CALL", "INTERRUPT_ATTACH")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: type, id
    // Wide: type, id
    // Call: InterruptQuery
    ev("KER_CALL", "INTERRUPT_QUERY")
        .kind("ENTER")
        .fast(&[d("type", 0), d("id", 1)])
        .wide(&[d("type", 0), d("id", 1)]),
    ev("KER_CALL", "INTERRUPT_QUERY")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: InterruptQuery
    ev("KER_CALL", "INTERRUPT_QUERY")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "INTERRUPT_QUERY")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: id, empty
    // Wide: id, empty
    // Call: InterruptDetach
    ev("KER_CALL", "INTERRUPT_DETACH")
        .kind("ENTER")
        .fast(&[d("id", 0), h("empty", 1)])
        .wide(&[d("id", 0), h("empty", 1)]),
    ev("KER_CALL", "INTERRUPT_DETACH")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: InterruptDetach
    ev("KER_CALL", "INTERRUPT_DETACH")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "INTERRUPT_DETACH")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: flags, timeout_tv_sec
    // Wide: flags, timeout_tv_sec, timeout_tv_nsec
    // Call: InterruptWait
    ev("KER_CALL", "INTERRUPT_WAIT")
        .kind("ENTER")
        .fast(&[h("flags", 0), d("timeout_tv_sec", 1)])
        .wide(&[h("flags", 0), d("timeout_tv_sec", 1), d("timeout_tv_nsec", 2)]),
    ev("KER_CALL", "INTERRUPT_WAIT")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, timeout_p
    // Wide: ret_val, timeout_p
    // Call: InterruptWait
    ev("KER_CALL", "INTERRUPT_WAIT")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("timeout", 1)])
        .wide(&[d("ret_val", 0), h("timeout", 1)]),
    // Fast: N/A
    // Wide: ret_val, timeout_p (64)
    // Call: InterruptWait
    ev("KER_CALL", "INTERRUPT_WAIT")
        .kind("EXIT")
        .is64()
        .fast(&[])
        .wide(&[d("ret_val", 0), q("timeout", 1)]),
    // Fast: intr, id
    // Wide: intr, id
    // Call: InterruptMask
    ev("KER_CALL", "INTERRUPT_MASK")
        .kind("ENTER")
        .fast(&[d("intr", 0), d("id", 1)])
        .wide(&[d("intr", 0), d("id", 1)]),
    ev("KER_CALL", "INTERRUPT_MASK")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: mask_level, empty
    // Wide: mask_level, empty
    // Call: InterruptMask
    ev("KER_CALL", "INTERRUPT_MASK")
        .kind("EXIT")
        .fast(&[h("mask_level", 0), h("empty", 1)])
        .wide(&[h("mask_level", 0), h("empty", 1)]),
    ev("KER_CALL", "INTERRUPT_MASK")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: intr, id
    // Wide: intr, id
    // Call: InterruptUnmask
    ev("KER_CALL", "INTERRUPT_UNMASK")
        .kind("ENTER")
        .fast(&[d("intr", 0), d("id", 1)])
        .wide(&[d("intr", 0), d("id", 1)]),
    ev("KER_CALL", "INTERRUPT_UNMASK")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: mask_level, empty
    // Wide: mask_level, empty
    // Call: InterruptUnmask
    ev("KER_CALL", "INTERRUPT_UNMASK")
        .kind("EXIT")
        .fast(&[h("mask_level", 0), h("empty", 1)])
        .wide(&[h("mask_level", 0), h("empty", 1)]),
    ev("KER_CALL", "INTERRUPT_UNMASK")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "INTERRUPT_UNBLOCK")
        .kind("ENTER")
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "INTERRUPT_UNBLOCK")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "INTERRUPT_UNBLOCK")
        .kind("EXIT")
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "INTERRUPT_UNBLOCK")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "INTERRUPT_CTL")
        .kind("ENTER")
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "INTERRUPT_CTL")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "INTERRUPT_CTL").kind("EXIT").fast_silent().wide_silent(),
    ev("KER_CALL", "INTERRUPT_CTL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: id, new(sec)
    // Wide: id, new(sec), new(nsec)
    // Call: ClockTime
    ev("KER_CALL", "CLOCK_TIME")
        .kind("ENTER")
        .fast(&[d("id", 0), d("new(sec)", 1)])
        .wide(&[d("id", 0), d("new(sec)", 1), d("new(nsec)", 2)]),
    ev("KER_CALL", "CLOCK_TIME")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, old(sec)
    // Wide: ret_val, old(sec), old(nsec)
    // Call: ClockTime
    ev("KER_CALL", "CLOCK_TIME")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), d("old(sec)", 1)])
        .wide(&[d("ret_val", 0), d("old(sec)", 1), d("old(nsec)", 2)]),
    ev("KER_CALL", "CLOCK_TIME")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: id, new->tick_count
    // Wide: id, new->tick_count, new->tick_nsec_inc
    // Call: ClockAdjust
    ev("KER_CALL", "CLOCK_ADJUST")
        .kind("ENTER")
        .fast(&[d("id", 0), d("new->tick_count", 1)])
        .wide(&[d("id", 0), d("new->tick_count", 1), d("new->tick_nsec_inc", 2)]),
    ev("KER_CALL", "CLOCK_ADJUST")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, old->tick_count
    // Wide: ret_val, old->tick_count, old->tick_nsec_inc
    // Call: ClockAdjust
    ev("KER_CALL", "CLOCK_ADJUST")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), d("old->tick_count", 1)])
        .wide(&[d("ret_val", 0), d("old->tick_count", 1), d("old->tick_nsec_inc", 2)]),
    ev("KER_CALL", "CLOCK_ADJUST")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: id, new->nsec
    // Wide: id, new->nsec, new->fract
    // Call: ClockPeriod
    ev("KER_CALL", "CLOCK_PERIOD")
        .kind("ENTER")
        .fast(&[d("id", 0), d("new->nsec", 1)])
        .wide(&[d("id", 0), d("new->nsec", 1), d("new->fract", 2)]),
    ev("KER_CALL", "CLOCK_PERIOD")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, old->nsec
    // Wide: ret_val, old->nsec, old->fract
    // Call: ClockPeriod
    ev("KER_CALL", "CLOCK_PERIOD")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), d("old->nsec", 1)])
        .wide(&[d("ret_val", 0), d("old->nsec", 1), d("old->fract", 2)]),
    ev("KER_CALL", "CLOCK_PERIOD")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, tid
    // Wide: pid, tid
    // Call: ClockId
    ev("KER_CALL", "CLOCK_ID")
        .kind("ENTER")
        .fast(&[d("pid", 0), d("tid", 1)])
        .wide(&[d("pid", 0), d("tid", 1)]),
    ev("KER_CALL", "CLOCK_ID")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: ClockId
    ev("KER_CALL", "CLOCK_ID")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "CLOCK_ID")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: timer_id, event->sigev_notify
    // Wide: timer_id, event->sigev_notify, event->sigev_notify_function_p, event->sigev_value, event->sigev_notify_attributes_p
    // Call: TimerCreate
    ev("KER_CALL", "TIMER_CREATE")
        .kind("ENTER")
        .fast(&[h("id", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("id", 0),
            d("event->sigev_notify", 1),
            h("event->sigev_notify_function_p", 2),
            h("event->sigev_value", 3),
            h("event->sigev_notify_attributes_p", 4),
        ]),
    // Fast: timer_id, event->sigev_notify
    // Wide: timer_id, event->sigev_notify, pad, event->sigev_notify_function_p (64), event->sigev_value (64), event->sigev_notify_attributes_p (64)
    // Call: TimerCreate
    ev("KER_CALL", "TIMER_CREATE")
        .kind("ENTER")
        .is64()
        .fast(&[h("id", 0), d("event->sigev_notify", 1)])
        .wide(&[
            h("id", 0),
            d("event->sigev_notify", 1),
            h("pad", 2),
            q("event->sigev_notify_function_p", 3),
            q("event->sigev_value", 5),
            q("event->sigev_notify_attributes_p", 7),
        ]),
    // Fast: timer_id, empty
    // Wide: timer_id, empty
    // Call: TimerCreate
    ev("KER_CALL", "TIMER_CREATE")
        .kind("EXIT")
        .fast(&[h("timer_id", 0), h("empty", 1)])
        .wide(&[h("timer_id", 0), h("empty", 1)]),
    ev("KER_CALL", "TIMER_CREATE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: id, empty
    // Wide: id, empty
    // Call: TimerDestroy
    ev("KER_CALL", "TIMER_DESTROY")
        .kind("ENTER")
        .fast(&[h("id", 0), h("empty", 1)])
        .wide(&[h("id", 0), h("empty", 1)]),
    ev("KER_CALL", "TIMER_DESTROY")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: TimerDestroy
    ev("KER_CALL", "TIMER_DESTROY")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "TIMER_DESTROY")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: clock_id, itime->nsec_sec
    // Wide: clock_id, flags, itime->nsec_sec, itime->nsec_nsec, itime->interval_nsec_sec, itime->interval_nsec_nsec
    // Call: TimerSettime
    ev("KER_CALL", "TIMER_SETTIME")
        .kind("ENTER")
        .fast(&[h("id", 0), d("itime->nsec(sec)", 1)])
        .wide(&[
            h("id", 0),
            h("flags", 1),
            d("itime->nsec(sec)", 2),
            d("itime->nsec(nsec)", 3),
            d("itime->interval_nsec(sec)", 4),
            d("itime->interval_nsec(nsec)", 5),
        ]),
    ev("KER_CALL", "TIMER_SETTIME")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, oitime->nsec
    // Wide: ret_val, oitime->nsec_sec, oitime->nsec_nsec, oitime->interval_nsec_sec, oitime->interval_nsec_nsec
    // Call: TimerSettime
    ev("KER_CALL", "TIMER_SETTIME")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), d("oitime->nsec(sec)", 1)])
        .wide(&[
            h("ret_val", 0),
            d("oitime->nsec(sec)", 1),
            d("oitime->nsec(nsec)", 2),
            d("oitime->interval_nsec(sec)", 3),
            d("oitime->interval_nsec(nsec)", 4),
        ]),
    ev("KER_CALL", "TIMER_SETTIME")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, id
    // Wide: pid, id, flags, info_p
    // Call: TimerInfo
    ev("KER_CALL", "TIMER_INFO")
        .kind("ENTER")
        .fast(&[d("pid", 0), h("id", 1)])
        .wide(&[d("pid", 0), h("id", 1), h("flags", 2), h("info_p", 3)]),
    // Fast: pid, id
    // Wide: pid, id, flags, info_p (64)
    // Call: TimerInfo
    ev("KER_CALL", "TIMER_INFO")
        .kind("ENTER")
        .is64()
        .fast(&[d("pid", 0), h("id", 1)])
        .wide(&[d("pid", 0), h("id", 1), h("flags", 2), q("info_p", 3)]),
    // Fast: prev_id, info->itime.nsec
    // Wide: prev_id, info->itime.nsec, info->itime.interval_nsec, info->otime.nsec, info->otime.interval_nsec, info->flags, info->tid, info->notify, info->clockid, info->overruns, info->event.sigev_notify, info->event.sigev_notify_function_p, info->event.sigev_value, info->event.sigev_notify_attributes_p
    // Call: TimerInfo
    ev("KER_CALL", "TIMER_INFO")
        .kind("EXIT")
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
    // Fast: prev_id, info->itime.nsec
    // Wide: prev_id, info->itime.nsec, info->itime.interval_nsec, info->otime.nsec, info->otime.interval_nsec, info->flags, info->tid, info->notify, info->clockid, info->overruns, info->event.sigev_notify, pad, info->event.sigev_notify_function_p (64), info->event.sigev_value (64), info->event.sigev_notify_attributes_p (64)
    // Call: TimerInfo
    ev("KER_CALL", "TIMER_INFO")
        .kind("EXIT")
        .is64()
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
    // Fast: clock_id, itime->nsec_sec
    // Wide: clock_id, itime->nsec_sec, itime->nsec_nsec, itime->interval_nsec_sec, itime->interval_nsec_nsec
    // Call: TimerAlarm
    ev("KER_CALL", "TIMER_ALARM")
        .kind("ENTER")
        .fast(&[h("id", 0), d("itime->nsec(sec)", 1)])
        .wide(&[
            h("id", 0),
            d("itime->nsec(sec)", 1),
            d("itime->nsec(nsec)", 2),
            d("itime->interval_nsec(sec)", 3),
            d("itime->interval_nsec(nsec)", 4),
        ]),
    ev("KER_CALL", "TIMER_ALARM")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, otime->nsec_sec
    // Wide: ret_val, otime->nsec_sec, otime->nsec_nsec, otime->interval_nsec_sec, otime->interval_nsec_nsec
    // Call: TimerAlarm
    ev("KER_CALL", "TIMER_ALARM")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), d("oitime->nsec(sec)", 1)])
        .wide(&[
            h("ret_val", 0),
            d("oitime->nsec(sec)", 1),
            d("oitime->nsec(nsec)", 2),
            d("oitime->interval_nsec(sec)", 3),
            d("oitime->interval_nsec(nsec)", 4),
        ]),
    ev("KER_CALL", "TIMER_ALARM")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: clock_id, timeout_flags, ntime_sec
    // Wide: clock_id, timeout_flags, ntime_sec, ntime_nsec, event->sigev_notify, event->sigev_notify_function_p, event->sigev_value, event->sigev_notify_attributes_p, empty
    // Call: TimerTimeout
    ev("KER_CALL", "TIMER_TIMEOUT")
        .kind("ENTER")
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
    // Fast: clock_id, timeout_flags, ntime_sec
    // Wide: clock_id, timeout_flags, ntime_sec, ntime_nsec, event->sigev_notify, pad, event->sigev_notify_function_p (64), event->sigev_value (64), event->sigev_notify_attributes_p (64)
    // Call: TimerTimeout
    ev("KER_CALL", "TIMER_TIMEOUT")
        .kind("ENTER")
        .is64()
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
    // Fast: prev_timeout_flags, otime_sec
    // Wide: prev_timeout_flags, otime_sec, otime_nsec, empty
    // Call: TimerTimeout
    ev("KER_CALL", "TIMER_TIMEOUT")
        .kind("EXIT")
        .fast(&[h("prev_timeout_flags", 0), d("otime(sec)", 1)])
        .wide(&[h("prev_timeout_flags", 0), d("otime(sec)", 1), d("otime(nsec)", 2)]),
    ev("KER_CALL", "TIMER_TIMEOUT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: action, delegate_cpu
    // Wide: action, delegate_cpu
    // Call: TimerDelegate
    ev("KER_CALL", "TIMER_DELEGATE")
        .kind("ENTER")
        .fast(&[d("action", 0), d("delegate_cpu", 1)])
        .wide(&[d("action", 0), d("delegate_cpu", 1)]),
    ev("KER_CALL", "TIMER_DELEGATE")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: TimerDelegate
    ev("KER_CALL", "TIMER_DELEGATE")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "TIMER_DELEGATE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "SYNC_RWLOCK").kind("ENTER").fast_silent().wide_silent(),
    // Fast: syncp(64), op, count, owner
    // Wide: syncp(64), op, count, owner
    // Call: SyncRWLock
    ev("KER_CALL", "SYNC_RWLOCK")
        .kind("ENTER")
        .is64()
        .fast(&[q("syncp", 0)])
        .wide(&[q("syncp", 0), d("op", 2), d("count", 3), d("owner", 4)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncRWLock
    ev("KER_CALL", "SYNC_RWLOCK")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_RWLOCK")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: type, sync_p
    // Wide: type, sync_p, count, owner, protocol, flags, prioceiling, clockid
    // Call: SyncCreate
    ev("KER_CALL", "SYNC_CREATE")
        .kind("ENTER")
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
    // Fast: N/A
    // Wide: type, sync_p (64), count, owner, protocol, flags, prioceiling, clockid
    // Call: SyncCreate
    ev("KER_CALL", "SYNC_CREATE")
        .kind("ENTER")
        .is64()
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
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncCreate
    ev("KER_CALL", "SYNC_CREATE")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_CREATE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p, owner
    // Wide: sync_p, count, owner
    // Call: SyncDestroy
    ev("KER_CALL", "SYNC_DESTROY")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    // Fast: sync_p (64)
    // Wide: sync_p (64), count, owner
    // Call: SyncDestroy
    ev("KER_CALL", "SYNC_DESTROY")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncDestroy
    ev("KER_CALL", "SYNC_DESTROY")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_DESTROY")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p, owner
    // Wide: sync_p, count, owner
    // Call: SyncMutexLock
    ev("KER_CALL", "SYNC_MUTEX_LOCK")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    // Fast: sync_p (64)
    // Wide: sync_p (64), count, owner
    // Call: SyncMutexLock
    ev("KER_CALL", "SYNC_MUTEX_LOCK")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncMutexLock
    ev("KER_CALL", "SYNC_MUTEX_LOCK")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_MUTEX_LOCK")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p, owner
    // Wide: sync_p, count, owner
    // Call: SyncMutexUnlock
    ev("KER_CALL", "SYNC_MUTEX_UNLOCK")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    // Fast: sync_p (64), owner
    // Wide: sync_p (64), count, owner
    // Call: SyncMutexUnlock
    ev("KER_CALL", "SYNC_MUTEX_UNLOCK")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncMutexUnlock
    ev("KER_CALL", "SYNC_MUTEX_UNLOCK")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_MUTEX_UNLOCK")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p, mutex_p
    // Wide: sync_p, mutex_p, sync->count, sync->owner, mutex->count, mutex->owner
    // Call: SyncCondvarWait
    ev("KER_CALL", "SYNC_CONDVAR_WAIT")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), h("mutex_p", 1)])
        .wide(&[
            h("sync_p", 0),
            h("mutex_p", 1),
            d("sync->count", 2),
            d("sync->owner", 3),
            d("mutex->count", 4),
            d("mutex->owner", 5),
        ]),
    // Fast: sync_p (64)
    // Wide: sync_p (64), mutex_p (64), sync->count, sync->owner, mutex->count, mutex->owner
    // Call: SyncCondvarWait
    ev("KER_CALL", "SYNC_CONDVAR_WAIT")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[
            q("sync_p", 0),
            q("mutex_p", 2),
            d("sync->count", 4),
            d("sync->owner", 5),
            d("mutex->count", 6),
            d("mutex->owner", 7),
        ]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncCondvarWait
    ev("KER_CALL", "SYNC_CONDVAR_WAIT")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_CONDVAR_WAIT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "SYNC_CONDVAR_WAIT").kind("INT").intcall(),
    ev("KER_CALL", "SYNC_CONDVAR_WAIT").kind("INT").is64().wide_silent(),
    // Fast: sync_p, all
    // Wide: sync_p, all, sync->count, sync->owner
    // Call: SyncCondvarSignal
    ev("KER_CALL", "SYNC_CONDVAR_SIGNAL")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), d("all", 1)])
        .wide(&[h("sync_p", 0), d("all", 1), d("sync->count", 2), d("sync->owner", 3)]),
    // Fast: sync_p (64)
    // Wide: sync_p (64), all, sync->count, sync->owner
    // Call: SyncCondvarSignal
    ev("KER_CALL", "SYNC_CONDVAR_SIGNAL")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("all", 2), d("sync->count", 3), d("sync->owner", 4)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncCondvarSignal
    ev("KER_CALL", "SYNC_CONDVAR_SIGNAL")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_CONDVAR_SIGNAL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p, count
    // Wide: sync_p, count, owner
    // Call: SyncSemPost
    ev("KER_CALL", "SYNC_SEM_POST")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), d("count", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2)]),
    // Fast: sync_p (64)
    // Wide: sync_p (64), count, owner
    // Call: SyncSemPost
    ev("KER_CALL", "SYNC_SEM_POST")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncSemPost
    ev("KER_CALL", "SYNC_SEM_POST")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_SEM_POST")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p, count
    // Wide: sync_p, try, count, owner
    // Call: SyncSemWait
    ev("KER_CALL", "SYNC_SEM_WAIT")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), d("count", 1)])
        .wide(&[h("sync_p", 0), d("try", 1), d("count", 2), d("owner", 3)]),
    // Fast: sync_p (64)
    // Wide: sync_p (64), try, count, owner
    // Call: SyncSemWait
    ev("KER_CALL", "SYNC_SEM_WAIT")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("try", 2), d("count", 3), d("owner", 4)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncSemWait
    ev("KER_CALL", "SYNC_SEM_WAIT")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_SEM_WAIT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: cmd, sync_p
    // Wide: cmd, sync_p, data_p, count, owner
    // Call: SyncCtl
    ev("KER_CALL", "SYNC_CTL")
        .kind("ENTER")
        .fast(&[d("cmd", 0), h("sync_p", 1)])
        .wide(&[
            d("cmd", 0),
            h("sync_p", 1),
            h("data_p", 2),
            d("count", 3),
            d("owner", 4),
        ]),
    // Fast: cmd
    // Wide: cmd, sync_p (64), data_p (64), count, owner
    // Call: SyncCtl
    ev("KER_CALL", "SYNC_CTL")
        .kind("ENTER")
        .is64()
        .fast(&[d("cmd", 0)])
        .wide(&[
            d("cmd", 0),
            q("sync_p", 1),
            q("data_p", 3),
            d("count", 5),
            d("owner", 6),
        ]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncCtl
    ev("KER_CALL", "SYNC_CTL")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_CTL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "SYNC_BARRIER_WAIT")
        .kind("ENTER")
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p(64)
    // Wide: sync_p(64), count, owner
    // Call: SyncBarrierWait
    ev("KER_CALL", "SYNC_BARRIER_WAIT")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncBarrierWait
    ev("KER_CALL", "SYNC_BARRIER_WAIT")
        .kind("EXIT")
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev("KER_CALL", "SYNC_BARRIER_WAIT")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, tid
    // Wide: pid, tid
    // Call: SchedGet
    ev("KER_CALL", "SCHED_GET")
        .kind("ENTER")
        .fast(&[d("pid", 0), d("tid", 1)])
        .wide(&[d("pid", 0), d("tid", 1)]),
    ev("KER_CALL", "SCHED_GET")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, sched_priority
    // Wide: ret_val, sched_priority, sched_curpriority, param.ss_low_priority, param.ss_max_repl, param.ss_repl_period.tv_sec, param.ss_repl_period.tv_nsec, param.ss_init_budget.tv_sec, param.ss_init_budget.tv_nsec
    // Call: SchedGet
    ev("KER_CALL", "SCHED_GET")
        .kind("EXIT")
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
    // Fast: ret_val, sched_priority
    // Wide: ret_val, sched_priority, sched_curpriority, param.ss_low_priority, param.ss_max_repl, param.ss_repl_period.tv_sec (64), param.ss_repl_period.tv_nsec (64), param.ss_init_budget.tv_sec (64), param.ss_init_budget.tv_nsec (64)
    // Call: SchedGet
    ev("KER_CALL", "SCHED_GET")
        .kind("EXIT")
        .is64()
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
    // Fast: pid, sched_priority
    // Wide: pid, tid, policy, sched_priority, sched_curpriority, param.ss_low_priority, param.ss_max_repl, param.ss_repl_period.tv_sec, param.ss_repl_period.tv_nsec, param.ss_init_budget.tv_sec, param.ss_init_budget.tv_nsec
    // Call: SchedSet
    ev("KER_CALL", "SCHED_SET")
        .kind("ENTER")
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
    // Fast: pid, sched_priority
    // Wide: pid, tid, policy, sched_priority, sched_curpriority, param.ss_low_priority, param.ss_max_repl, param.ss_repl_period.tv_sec (64), param.ss_repl_period.tv_nsec (64), param.ss_init_budget.tv_sec (64), param.ss_init_budget.tv_nsec (64)
    // Call: SchedSet
    ev("KER_CALL", "SCHED_SET")
        .kind("ENTER")
        .is64()
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
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SchedSet
    ev("KER_CALL", "SCHED_SET")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SCHED_SET")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: empty, empty
    // Wide: empty, empty
    // Call: SchedYield
    ev("KER_CALL", "SCHED_YIELD")
        .kind("ENTER")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "SCHED_YIELD")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SchedYield
    ev("KER_CALL", "SCHED_YIELD")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SCHED_YIELD")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: pid, policy
    // Wide: pid, policy
    // Call: SchedInfo
    ev("KER_CALL", "SCHED_INFO")
        .kind("ENTER")
        .fast(&[d("pid", 0), d("policy", 1)])
        .wide(&[d("pid", 0), d("policy", 1)]),
    ev("KER_CALL", "SCHED_INFO")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val, priority_max
    // Wide: ret_val, priority_min, priority_max, interval_sec, interval_nsec, priority_priv
    // Call: SchedInfo
    ev("KER_CALL", "SCHED_INFO")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), d("priority_max", 1)])
        .wide(&[
            d("ret_val", 0),
            d("priority_min", 1),
            d("priority_max", 2),
            d("interval_sec", 3),
            d("interval_nsec", 4),
            d("priority_priv", 5),
        ]),
    ev("KER_CALL", "SCHED_INFO")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: cmd, data_p
    // Wide: cmd, data_p
    // Call: SchedCtl
    ev("KER_CALL", "SCHED_CTL").kind("ENTER").fast_silent().wide_silent(),
    // Fast: N/A
    // Wide: cmd, data_p (64)
    // Call: SchedCtl
    ev("KER_CALL", "SCHED_CTL")
        .kind("ENTER")
        .is64()
        .fast(&[h("cmd", 0), q("data", 1)])
        .wide(&[h("cmd", 0), q("data", 1)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SchedCtl
    ev("KER_CALL", "SCHED_CTL")
        .kind("EXIT")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "SCHED_CTL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid(64), queueid, msgsize, maxmsg, oflag
    // Wide: rcvid(64), queueid, msgsize, maxmsg, oflag
    // Call: MsgQueueOpen
    ev("KER_CALL", "MSG_QUEUE_OPEN")
        .kind("ENTER")
        .fast(&[q("rcvid", 0)])
        .wide(&[
            q("rcvid", 0),
            d("queueid", 2),
            d("msgsize", 3),
            d("maxmsg", 4),
            d("oflag", 5),
        ]),
    ev("KER_CALL", "MSG_QUEUE_OPEN")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val
    // Wide: ret_val
    // Call: MsgQueueOpen
    ev("KER_CALL", "MSG_QUEUE_OPEN")
        .kind("EXIT")
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev("KER_CALL", "MSG_QUEUE_OPEN")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: fd, msg[0]
    // Wide: fr, msg[0], msglen(64), priority
    // Call: MsgQueueSend
    ev("KER_CALL", "MSG_QUEUE_SEND")
        .kind("ENTER")
        .fast(&[d("fd", 0), s("msg[0]", 1, 1)])
        .wide(&[d("fd", 0), s("msg[0]", 1, 1), e("msglen", 2), d("priority", 4)]),
    ev("KER_CALL", "MSG_QUEUE_SEND")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val
    // Wide: ret_val
    // Call: MsgQueueSend
    ev("KER_CALL", "MSG_QUEUE_SEND")
        .kind("EXIT")
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev("KER_CALL", "MSG_QUEUE_SEND")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: fd
    // Wide: fd, msglen(64), msg(64), priority(64)
    // Call: MsgQueueReceive
    ev("KER_CALL", "MSG_QUEUE_RECEIVE")
        .kind("ENTER")
        .fast(&[d("fd", 0)])
        .wide(&[d("fd", 0), e("msglen", 1), q("msg", 3), q("priority", 5)]),
    ev("KER_CALL", "MSG_QUEUE_RECEIVE")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: msglen, msg[0], priority
    // Wide: msglen, msg[0], priority
    // Call: MsgQueueReceive
    ev("KER_CALL", "MSG_QUEUE_RECEIVE")
        .kind("EXIT")
        .fast(&[d("msglen", 0), s("msg[0]", 1, 1)])
        .wide(&[d("msglen", 0), s("msg[0]", 1, 1), d("priority", 2)]),
    ev("KER_CALL", "MSG_QUEUE_RECEIVE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: rcvid(64), queueid
    // Wide: rcvid(64), queueid
    // Call: MsgQueueClose
    ev("KER_CALL", "MSG_QUEUE_CLOSE")
        .kind("ENTER")
        .fast(&[q("rcvid", 0)])
        .wide(&[q("rcvid", 0), d("queueid", 2)]),
    ev("KER_CALL", "MSG_QUEUE_CLOSE")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: ret_val
    // Wide: ret_val
    // Call: MsgQueueClose
    ev("KER_CALL", "MSG_QUEUE_CLOSE")
        .kind("EXIT")
        .fast(&[d("ret_val", 0)])
        .wide(&[d("ret_val", 0)]),
    ev("KER_CALL", "MSG_QUEUE_CLOSE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: (_NTO_MQCTL_NOTIFY): fd, cmd / (_NTO_MQCTL_SETATTR): fd, cmd / (Other): fd, cmd
    // Wide: (_NTO_MQCTL_NOTIFY): fd, cmd, sigev_notify, empty, sigev_notify_function(64), sigev_value(64), sigev_notify_attributes(64) / (_NTO_MQCTL_SETATTR): fd, cmd, mq_maxmsg(64), mq_msgsize(64), mq_flags(64), mq_curmsgs(64), mq_sendwait(64), mq_recvwait(64) / (Other): fd, cmd
    // Call: MsgQueueCtl
    ev("KER_CALL", "MSG_QUEUE_CTL")
        .kind("ENTER")
        .fast(&[d("fd", 0), d("cmd", 1)])
        .wide(&[d("fd", 0), d("cmd", 1)]),
    ev("KER_CALL", "MSG_QUEUE_CTL")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: (_NTO_MQCTL_GETATTR): ret_val, cmd / (Other): ret_val, cmd
    // Wide: (_NTO_MQCTL_GETATTR): ret_val, cmd, mq_maxmsg(64), mq_msgsize(64), mq_flags(64), mq_curmsgs(64), mq_sendwait(64), mq_recvwait(64) / (Other): ret_val, cmd
    // Call: mq_unlink, mq_notify, mq_setaddr, mq_getaddr
    ev("KER_CALL", "MSG_QUEUE_CTL")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), d("cmd", 1)])
        .wide(&[d("ret_val", 0), d("cmd", 1)]),
    ev("KER_CALL", "MSG_QUEUE_CTL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "PIPE_OPEN").kind("ENTER").fast_silent().wide_silent(),
    ev("KER_CALL", "PIPE_OPEN")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "PIPE_OPEN").kind("EXIT").fast_silent().wide_silent(),
    ev("KER_CALL", "PIPE_OPEN")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "PIPE_CTL").kind("ENTER").fast_silent().wide_silent(),
    ev("KER_CALL", "PIPE_CTL")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "PIPE_CTL").kind("EXIT").fast_silent().wide_silent(),
    ev("KER_CALL", "PIPE_CTL")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "PIPE_CLOSE").kind("ENTER").fast_silent().wide_silent(),
    ev("KER_CALL", "PIPE_CLOSE")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "PIPE_CLOSE").kind("EXIT").fast_silent().wide_silent(),
    ev("KER_CALL", "PIPE_CLOSE")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: sync_p, owner
    // Wide: sync_p, count, owner
    // Call: SyncTypeDestroy
    ev("KER_CALL", "SYNC_TYPE_DESTROY")
        .kind("ENTER")
        .fast(&[h("sync_p", 0), d("owner", 1)])
        .wide(&[h("sync_p", 0), d("count", 1), d("owner", 2), d("type", 3)]),
    // Fast: sync_p (64)
    // Wide: sync_p (64), count, owner
    // Call: SyncTypeDestroy
    ev("KER_CALL", "SYNC_TYPE_DESTROY")
        .kind("ENTER")
        .is64()
        .fast(&[q("sync_p", 0)])
        .wide(&[q("sync_p", 0), d("count", 2), d("owner", 3), d("type", 4)]),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    // Call: SyncTypeDestroy
    ev("KER_CALL", "SYNC_TYPE_DESTROY")
        .kind("EXIT")
        .fast(&[d("ret_val", 0), h("empty", 1)])
        .wide(&[d("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "SYNC_TYPE_DESTROY")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "TEST").kind("ENTER").fast_silent().wide_silent(),
    ev("KER_CALL", "TEST").kind("ENTER").is64().fast_silent().wide_silent(),
    ev("KER_CALL", "TEST").kind("EXIT").fast_silent().wide_silent(),
    ev("KER_CALL", "TEST").kind("EXIT").is64().fast_silent().wide_silent(),
    ev("KER_CALL", "SYS_CUSTOM").kind("ENTER").fast_silent().wide_silent(),
    ev("KER_CALL", "SYS_CUSTOM")
        .kind("ENTER")
        .is64()
        .fast_silent()
        .wide_silent(),
    ev("KER_CALL", "SYS_CUSTOM").kind("EXIT").fast_silent().wide_silent(),
    ev("KER_CALL", "SYS_CUSTOM")
        .kind("EXIT")
        .is64()
        .fast_silent()
        .wide_silent(),
    // Fast: empty, empty
    // Wide: empty, empty
    ev("KER_CALL", "BAD")
        .kind("ENTER")
        .fast(&[h("empty", 0), h("empty", 1)])
        .wide(&[h("empty", 0), h("empty", 1)]),
    ev("KER_CALL", "BAD").kind("ENTER").is64().fast_silent().wide_silent(),
    // Fast: ret_val, empty
    // Wide: ret_val, empty
    ev("KER_CALL", "BAD")
        .kind("EXIT")
        .fast(&[h("ret_val", 0), h("empty", 1)])
        .wide(&[h("ret_val", 0), h("empty", 1)]),
    ev("KER_CALL", "BAD").kind("EXIT").is64().fast_silent().wide_silent(),
    ev("SYSTEM", "RESERVED").wide_silent(),
    ev("SYSTEM", "APS_NAME").wide(&[d("partition_id", 0), sz("name", 1)]),
    ev("SYSTEM", "APS_BUDGETS").wide(&[
        d("partition_id", 0),
        d("min_percentage_budget", 1),
        d("critical_budget", 2),
        d("max_percentage_budget", 3),
        d("critical_priority", 4),
        d("budget_percent_scale", 5),
    ]),
    ev("SYSTEM", "APS_BNKR").wide(&[d("pid", 0), d("tid", 1), d("partition_id", 2)]),
    // Fast: pid, addr (64), len (64), flags
    // Wide: pid, addr (64), len (64), flags, prot, fd, align (64), offset (64), name
    // Call: mmap/mmap64
    ev("SYSTEM", "MMAP").wide(&[
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
    // Fast: pid, addr (64), len (64)
    // Wide: pid, addr (64), len (64)
    // Call: munmap
    ev("SYSTEM", "MUNMAP").wide(&[d("pid", 0), q("vaddr", 1), q("len", 3)]),
    // Fast: pc_addr(32), <null>
    // Wide: pc_addr(32), <null>
    // Call: whenever a breakpoint is hit
    ev("SYSTEM", "ADDRESS").wide_silent(),
    // Fast: thisfn(32), call_site(32)
    // Wide: thisfn(32), call_site(32)
    // Call: whenever a function is entered (and it is instrumented)
    ev("SYSTEM", "FUNC_ENTER").wide(&[h("thisfn", 0), h("call_site", 1)]),
    // Fast: thisfn(32), call_site(32)
    // Wide: thisfn(32), call_site(32)
    // Call: whenever a function is exited (and it is instrumented)
    ev("SYSTEM", "FUNC_EXIT").wide(&[h("thisfn", 0), h("call_site", 1)]),
    ev("SYSTEM", "DEFRAG_START").wide(&[d("block_size", 0)]),
    // Fast: bitset(32) 0x1 - CPU is on/offline, 0x2 - CPU manually requested on/offline, 0x4 CPU is dynamically offline-able or not, 0x8 system is in runstate burst mode
    // Wide: same as above
    // Call: when the runstate for a CPU changes
    ev("SYSTEM", "RUNSTATE").wide(&[h("bitset", 0)]),
    // Fast: rc(32), freemem(32), maxblock(32)
    // Wide: rc(32), freemem(32), maxblock(32)
    // Call: on completion of the defragmentation process (whether successful or not)
    ev("SYSTEM", "DEFRAG_END").wide(&[d("result", 0), d("freemem", 1), d("maxblock", 2)]),
    ev("SYSTEM", "APS_PSTATS").wide(&[
        d("id", 0),
        d("stats_flags", 1),
        e("run_time_cycles", 2),
        e("critical_time_cycles", 4),
        e("dynamic_windowsize_cycles", 6),
    ]),
    ev("SYSTEM", "APS_OSTATS").wide(&[
        d("pid_at_last_bankruptcy", 0),
        d("tid_at_last_bankruptcy", 1),
        d("id_at_last_bankruptcy", 2),
    ]),
    ev("SYSTEM", "APS_INFO").wide(&[
        d("windowsize_ms", 0),
        d("scheduling_policy_flags", 1),
        d("sec_flags", 2),
        d("bankruptcy_policy", 3),
        d("num_partitions", 4),
        d("max_partitions", 5),
        e("cycles_per_ms", 6),
    ]),
    ev("SYSTEM", "APS_JOIN").wide(&[d("id", 0), d("pid", 1), d("tid", 2), d("aid", 3)]),
    ev("SYSTEM", "APS_THREAD").wide(&[
        d("pid", 0),
        d("tid", 1),
        d("id", 2),
        d("inherited_id", 3),
        d("crit_state_flags", 4),
    ]),
    ev("SYSTEM", "APS_PROCESS").wide(&[d("pid", 0), d("id", 1)]),
    // Fast: old low latency priority, old migrate priority, new low latency priority, new migrate priority
    // Wide: same as above
    // Call: SchedCtl with SCHED_CONFIGURE
    ev("SYSTEM", "SCHED_CONF").wide_silent(),
    ev("SYSTEM", "PCTRACE").wide_silent(),
    // Fast: pc_addr(64)
    // Wide: pc_addr(64)
    // Call: if enabled via _NTO_TRACE_SYS_ADDRESS_64, whenever a breakpoint is hit; if enabled via _NTO_TRACE_SYS_PCTRACE, whenever a context switch occurs
    ev("SYSTEM", "ADDRESS").is64().wide(&[q("addr", 0)]),
    // Fast: thisfn(64), call_site(64)
    // Wide: thisfn(64), call_site(64)
    // Call: whenever a function is entered (and it is instrumented)
    ev("SYSTEM", "FUNC_ENTER")
        .is64()
        .wide(&[q("thisfn", 0), q("call_site", 2)]),
    // Fast: thisfn(64), call_site(64)
    // Wide: thisfn(64), call_site(64)
    // Call: whenever a function is exited (and it is instrumented)
    ev("SYSTEM", "FUNC_EXIT")
        .is64()
        .wide(&[q("thisfn", 0), q("call_site", 2)]),
    // Fast: guest_ip(64)
    // Wide: guest_ip(64)
    // Call: Before a virtual CPU thread attempts executing a guest.
    ev("QVM", "GUEST_ENTER").wide(&[q("guest_ip", 0)]),
    // Fast: status(32), hw_reason(32), clockcycles_offset(64), guest_ip(64), payload(64)
    // Wide: status(32), hw_reason(32), clockcycles_offset(64), guest_ip(64), payload(64)
    // Call: After a virtual CPU thread has attempted executing a guest.
    ev("QVM", "GUEST_EXIT").wide(&[
        h("status", 0),
        h("hw_reason", 1),
        q("clockcycles_offset", 2),
        q("guest_ip", 4),
        q("payload", 6),
    ]),
    // Fast: vcpu_id(32)
    // Wide: vcpu_id(32)
    // Call: When qvm creates a thread to encapsulate a guest's virtual CPU.
    ev("QVM", "CREATE_VCPU_THREAD").wide(&[d("vcpu_id", 0)]),
    // Fast: vcpu_id(32), intr_vector(32)
    // Wide: vcpu_id(32), intr_vector(32)
    // Call: When a virtual device tries to signal an interrupt.
    ev("QVM", "RAISE_INTR").wide(&[d("vcpu_id", 0), h("intr_vector", 1)]),
    // Fast: vcpu_id(32), intr_vector(32)
    // Wide: vcpu_id(32), intr_vector(32)
    // Call: When an interrupt is de-asserted for any reason.
    ev("QVM", "LOWER_INTR").wide(&[d("vcpu_id", 0), h("intr_vector", 1)]),
    // Fast: timer_id(32), vcpu_id(32)
    // Wide: timer_id(32), vcpu_id(32)
    // Call: When a virtual device creates a timer.
    ev("QVM", "TIMER_CREATE").wide(&[h("timer_id", 0), d("vcpu_id", 1)]),
    // Fast: timer_id(32)
    // Wide: timer_id(32)
    // Call: When the hypervisor invokes a virtual device's callback for a timer.
    ev("QVM", "TIMER_FIRE").wide(&[h("timer_id", 0)]),
    // Fast: at_entry(64), at_exit(64)
    // Wide: at_entry(64), at_exit(64)
    // Call: When a guest has successfully executed for a period of time.
    ev("QVM", "CYCLES").wide(&[q("at_entry", 0), q("at_exit", 2)]),
];
