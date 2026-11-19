//! QNX 8.0 の内部 ID の定義。
//!
//! 生成ファイル(tests/kev/tools/gen_qnx_def.py)。手で編集しない。
//! 入力: SDP のヘッダ(sys/trace.h、sys/kercalls.h、sys/states.h、sys/nto_version.h)と TableOfEvents。

use crate::semantic_id::{self, EventId};

// ---- header のビット配置(_NTO_TRACE_GETEVENT・_NTO_TRACE_GETEVENT_C・_NTO_TRACE_GETCPU・_TRACE_GET_STRUCT)
pub const EVENT_SHIFT: u32 = 0;
pub const EVENT_MASK: u32 = 0x3ff;
pub const CLASS_SHIFT: u32 = 10;
pub const CLASS_MASK: u32 = 0x1f;
pub const CPU_SHIFT: u32 = 24;
pub const CPU_MASK: u32 = 0x3f;
pub const STRUCT_SHIFT: u32 = 30;
pub const STRUCT_MASK: u32 = 0x3;
/// フラグ(_NTO_TRACE_PROCESS_INJECTED のビットから CPU の手前まで)
pub const FLAGS_SHIFT: u32 = 16;
pub const FLAGS_MASK: u32 = 0xff;
pub const FLAG_PROCESS_INJECTED: u8 = 0x1;

// ---- 構造(_TRACE_STRUCT_*。シフト後の値)
pub const STRUCT_S: u32 = 0;
pub const STRUCT_CB: u32 = 1;
pub const STRUCT_CC: u32 = 2;
pub const STRUCT_CE: u32 = 3;

// ---- 内部クラス ID(_TRACE_*_C。シフト後の値)
pub const CLASS_CONTROL: u8 = 1;
pub const CLASS_KER_CALL: u8 = 2;
pub const CLASS_INT: u8 = 3;
pub const CLASS_PR_TH: u8 = 4;
pub const CLASS_SYSTEM: u8 = 5;
pub const CLASS_USER: u8 = 6;
pub const CLASS_COMM: u8 = 7;
pub const CLASS_QUIP: u8 = 8;
pub const CLASS_SEC: u8 = 9;
pub const CLASS_QVM: u8 = 10;
/// 内部クラス ID → ヘッダの定義名
pub const CLASS_DEFINE_NAMES: &[(u8, &str)] = &[
    (1, "_TRACE_CONTROL_C"),
    (2, "_TRACE_KER_CALL_C"),
    (3, "_TRACE_INT_C"),
    (4, "_TRACE_PR_TH_C"),
    (5, "_TRACE_SYSTEM_C"),
    (6, "_TRACE_USER_C"),
    (7, "_TRACE_COMM_C"),
    (8, "_TRACE_QUIP_C"),
    (9, "_TRACE_SEC_C"),
    (10, "_TRACE_QVM_C"),
];

// ---- CONTROL のイベント番号(_TRACE_CONTROL_*)
pub const CONTROL_TIME: u16 = 1;
pub const CONTROL_BUFFER: u16 = 2;
pub const CONTROL_BUFFER_END: u16 = 4;
pub const CONTROL_DROPPED: u16 = 8;

// ---- CONTROL の payloads の位置(TableOfEvents)
pub const TIME_MSB_WORD: usize = 0;
pub const TIME_LSB_WORD: usize = 1;
pub const BUFFER_SEQ_WORD: usize = 0;
pub const BUFFER_COUNT_WORD: usize = 1;
pub const DROPPED_COUNT_WORD: usize = 0;
/// BUFFER_END の payloads(_TRACECOOKIE)
pub const BUFFER_COOKIE: u32 = 0xe6e6e6e6;

// ---- KER_CALL のイベント番号 = コール番号 | 種別 | 64 ビット版(_TRACE_MAX_KER_CALL_NUM・_NTO_TRACE_KERCALL64)
pub const KERCALL_NUM_MASK: u16 = 0x7f;
pub const KERCALL_EXIT: u16 = 0x80;
pub const KERCALL_INT: u16 = 0x100;
/// 64 ビット版のビット(KER_CALL と SYSTEM で共通)
pub const BIT64: u16 = 0x200;
/// BIT64 のヘッダの定義名
pub const BIT64_DEFINE_NAME: &str = "_NTO_TRACE_KERCALL64";

// ---- PR_TH のイベント番号: これより小さければスレッドの状態(_TRACE_PR_TH_CREATE_P)
pub const PR_TH_PROCESS_BASE: u16 = 64;

/// カーネルコール(__KER_*): (コール番号, 名前, セマンティック ID)
pub const KERCALLS: &[(u16, &str, EventId)] = &[
    (0, "NOP", semantic_id::kercall::NOP),
    (1, "TRACE_EVENT", semantic_id::kercall::TRACE_EVENT),
    (2, "RING0", semantic_id::kercall::RING0),
    (3, "CACHE_CONTROL", semantic_id::kercall::CACHE_CONTROL),
    (4, "SYS_SRANDOM", semantic_id::kercall::SYS_SRANDOM),
    (5, "MSG_REGISTER_EVENT", semantic_id::kercall::MSG_REGISTER_EVENT),
    (6, "MSG_SEND_PULSEPTR", semantic_id::kercall::MSG_SEND_PULSEPTR),
    (7, "SYS_CPUPAGE_GET", semantic_id::kercall::SYS_CPUPAGE_GET),
    (8, "SYS_CPUPAGE_SET", semantic_id::kercall::SYS_CPUPAGE_SET),
    (9, "MSG_PAUSE", semantic_id::kercall::MSG_PAUSE),
    (10, "MSG_CURRENT", semantic_id::kercall::MSG_CURRENT),
    (11, "MSG_SENDV", semantic_id::kercall::MSG_SENDV),
    (12, "MSG_SENDVNC", semantic_id::kercall::MSG_SENDVNC),
    (13, "MSG_ERROR", semantic_id::kercall::MSG_ERROR),
    (14, "MSG_RECEIVEV", semantic_id::kercall::MSG_RECEIVEV),
    (15, "MSG_REPLYV", semantic_id::kercall::MSG_REPLYV),
    (16, "MSG_READV", semantic_id::kercall::MSG_READV),
    (17, "MSG_WRITEV", semantic_id::kercall::MSG_WRITEV),
    (19, "MSG_INFO", semantic_id::kercall::MSG_INFO),
    (20, "MSG_SEND_PULSE", semantic_id::kercall::MSG_SEND_PULSE),
    (21, "MSG_DELIVER_EVENT", semantic_id::kercall::MSG_DELIVER_EVENT),
    (24, "MSG_RECEIVEPULSEV", semantic_id::kercall::MSG_RECEIVEPULSEV),
    (25, "MSG_VERIFY_EVENT", semantic_id::kercall::MSG_VERIFY_EVENT),
    (26, "SIGNAL_KILL", semantic_id::kercall::SIGNAL_KILL),
    (27, "SIGNAL_RETURN", semantic_id::kercall::SIGNAL_RETURN),
    (29, "SIGNAL_ACTION", semantic_id::kercall::SIGNAL_ACTION),
    (30, "SIGNAL_PROCMASK", semantic_id::kercall::SIGNAL_PROCMASK),
    (31, "SIGNAL_SUSPEND", semantic_id::kercall::SIGNAL_SUSPEND),
    (32, "SIGNAL_WAITINFO", semantic_id::kercall::SIGNAL_WAITINFO),
    (33, "SIGNAL_KILL_SIGVAL", semantic_id::kercall::SIGNAL_KILL_SIGVAL),
    (35, "CHANNEL_CREATE", semantic_id::kercall::CHANNEL_CREATE),
    (36, "CHANNEL_DESTROY", semantic_id::kercall::CHANNEL_DESTROY),
    (37, "CHANNEL_CTL", semantic_id::kercall::CHANNEL_CTL),
    (39, "CONNECT_ATTACH", semantic_id::kercall::CONNECT_ATTACH),
    (40, "CONNECT_DETACH", semantic_id::kercall::CONNECT_DETACH),
    (41, "CONNECT_SERVER_INFO", semantic_id::kercall::CONNECT_SERVER_INFO),
    (42, "CONNECT_CLIENT_INFO", semantic_id::kercall::CONNECT_CLIENT_INFO),
    (43, "CONNECT_FLAGS", semantic_id::kercall::CONNECT_FLAGS),
    (44, "CONNECT_DUPFD", semantic_id::kercall::CONNECT_DUPFD),
    (46, "THREAD_CREATE", semantic_id::kercall::THREAD_CREATE),
    (47, "THREAD_DESTROY", semantic_id::kercall::THREAD_DESTROY),
    (49, "THREAD_DETACH", semantic_id::kercall::THREAD_DETACH),
    (50, "THREAD_JOIN", semantic_id::kercall::THREAD_JOIN),
    (51, "THREAD_CANCEL", semantic_id::kercall::THREAD_CANCEL),
    (52, "THREAD_CTL", semantic_id::kercall::THREAD_CTL),
    (53, "THREAD_CTLEXT", semantic_id::kercall::THREAD_CTLEXT),
    (55, "INTERRUPT_ATTACH", semantic_id::kercall::INTERRUPT_ATTACH),
    (56, "INTERRUPT_QUERY", semantic_id::kercall::INTERRUPT_QUERY),
    (57, "INTERRUPT_DETACH", semantic_id::kercall::INTERRUPT_DETACH),
    (58, "INTERRUPT_WAIT", semantic_id::kercall::INTERRUPT_WAIT),
    (59, "INTERRUPT_MASK", semantic_id::kercall::INTERRUPT_MASK),
    (60, "INTERRUPT_UNMASK", semantic_id::kercall::INTERRUPT_UNMASK),
    (61, "INTERRUPT_UNBLOCK", semantic_id::kercall::INTERRUPT_UNBLOCK),
    (62, "INTERRUPT_CTL", semantic_id::kercall::INTERRUPT_CTL),
    (65, "CLOCK_TIME", semantic_id::kercall::CLOCK_TIME),
    (66, "CLOCK_ADJUST", semantic_id::kercall::CLOCK_ADJUST),
    (67, "CLOCK_PERIOD", semantic_id::kercall::CLOCK_PERIOD),
    (68, "CLOCK_ID", semantic_id::kercall::CLOCK_ID),
    (70, "TIMER_CREATE", semantic_id::kercall::TIMER_CREATE),
    (71, "TIMER_DESTROY", semantic_id::kercall::TIMER_DESTROY),
    (72, "TIMER_SETTIME", semantic_id::kercall::TIMER_SETTIME),
    (73, "TIMER_INFO", semantic_id::kercall::TIMER_INFO),
    (74, "TIMER_ALARM", semantic_id::kercall::TIMER_ALARM),
    (75, "TIMER_TIMEOUT", semantic_id::kercall::TIMER_TIMEOUT),
    (76, "TIMER_DELEGATE", semantic_id::kercall::TIMER_DELEGATE),
    (77, "SYNC_RWLOCK", semantic_id::kercall::SYNC_RWLOCK),
    (78, "SYNC_CREATE", semantic_id::kercall::SYNC_CREATE),
    (79, "SYNC_DESTROY", semantic_id::kercall::SYNC_DESTROY),
    (80, "SYNC_MUTEX_LOCK", semantic_id::kercall::SYNC_MUTEX_LOCK),
    (81, "SYNC_MUTEX_UNLOCK", semantic_id::kercall::SYNC_MUTEX_UNLOCK),
    (82, "SYNC_CONDVAR_WAIT", semantic_id::kercall::SYNC_CONDVAR_WAIT),
    (83, "SYNC_CONDVAR_SIGNAL", semantic_id::kercall::SYNC_CONDVAR_SIGNAL),
    (84, "SYNC_SEM_POST", semantic_id::kercall::SYNC_SEM_POST),
    (85, "SYNC_SEM_WAIT", semantic_id::kercall::SYNC_SEM_WAIT),
    (86, "SYNC_CTL", semantic_id::kercall::SYNC_CTL),
    (87, "SYNC_BARRIER_WAIT", semantic_id::kercall::SYNC_BARRIER_WAIT),
    (88, "SCHED_GET", semantic_id::kercall::SCHED_GET),
    (89, "SCHED_SET", semantic_id::kercall::SCHED_SET),
    (90, "SCHED_YIELD", semantic_id::kercall::SCHED_YIELD),
    (91, "SCHED_INFO", semantic_id::kercall::SCHED_INFO),
    (92, "SCHED_CTL", semantic_id::kercall::SCHED_CTL),
    (93, "MSG_QUEUE_OPEN", semantic_id::kercall::MSG_QUEUE_OPEN),
    (94, "MSG_QUEUE_SEND", semantic_id::kercall::MSG_QUEUE_SEND),
    (95, "MSG_QUEUE_RECEIVE", semantic_id::kercall::MSG_QUEUE_RECEIVE),
    (96, "MSG_QUEUE_CLOSE", semantic_id::kercall::MSG_QUEUE_CLOSE),
    (97, "MSG_QUEUE_CTL", semantic_id::kercall::MSG_QUEUE_CTL),
    (98, "PIPE_OPEN", semantic_id::kercall::PIPE_OPEN),
    (99, "PIPE_CTL", semantic_id::kercall::PIPE_CTL),
    (100, "PIPE_CLOSE", semantic_id::kercall::PIPE_CLOSE),
    (104, "SYNC_TYPE_DESTROY", semantic_id::kercall::SYNC_TYPE_DESTROY),
    (105, "TEST", semantic_id::kercall::TEST),
    (106, "SYS_CUSTOM", semantic_id::kercall::SYS_CUSTOM),
    (107, "BAD", semantic_id::kercall::BAD),
];
/// KERCALLS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const KERCALLS_PREFIX: &str = "__KER_";

/// カーネルコールの 64 ビット版(全コール): (コール番号, 名前, セマンティック ID)
pub const KERCALLS_64: &[(u16, &str, EventId)] = &[
    (0, "NOP", semantic_id::kercall::NOP_64),
    (1, "TRACE_EVENT", semantic_id::kercall::TRACE_EVENT_64),
    (2, "RING0", semantic_id::kercall::RING0_64),
    (3, "CACHE_CONTROL", semantic_id::kercall::CACHE_CONTROL_64),
    (4, "SYS_SRANDOM", semantic_id::kercall::SYS_SRANDOM_64),
    (5, "MSG_REGISTER_EVENT", semantic_id::kercall::MSG_REGISTER_EVENT_64),
    (6, "MSG_SEND_PULSEPTR", semantic_id::kercall::MSG_SEND_PULSEPTR_64),
    (7, "SYS_CPUPAGE_GET", semantic_id::kercall::SYS_CPUPAGE_GET_64),
    (8, "SYS_CPUPAGE_SET", semantic_id::kercall::SYS_CPUPAGE_SET_64),
    (9, "MSG_PAUSE", semantic_id::kercall::MSG_PAUSE_64),
    (10, "MSG_CURRENT", semantic_id::kercall::MSG_CURRENT_64),
    (11, "MSG_SENDV", semantic_id::kercall::MSG_SENDV_64),
    (12, "MSG_SENDVNC", semantic_id::kercall::MSG_SENDVNC_64),
    (13, "MSG_ERROR", semantic_id::kercall::MSG_ERROR_64),
    (14, "MSG_RECEIVEV", semantic_id::kercall::MSG_RECEIVEV_64),
    (15, "MSG_REPLYV", semantic_id::kercall::MSG_REPLYV_64),
    (16, "MSG_READV", semantic_id::kercall::MSG_READV_64),
    (17, "MSG_WRITEV", semantic_id::kercall::MSG_WRITEV_64),
    (19, "MSG_INFO", semantic_id::kercall::MSG_INFO_64),
    (20, "MSG_SEND_PULSE", semantic_id::kercall::MSG_SEND_PULSE_64),
    (21, "MSG_DELIVER_EVENT", semantic_id::kercall::MSG_DELIVER_EVENT_64),
    (24, "MSG_RECEIVEPULSEV", semantic_id::kercall::MSG_RECEIVEPULSEV_64),
    (25, "MSG_VERIFY_EVENT", semantic_id::kercall::MSG_VERIFY_EVENT_64),
    (26, "SIGNAL_KILL", semantic_id::kercall::SIGNAL_KILL_64),
    (27, "SIGNAL_RETURN", semantic_id::kercall::SIGNAL_RETURN_64),
    (29, "SIGNAL_ACTION", semantic_id::kercall::SIGNAL_ACTION_64),
    (30, "SIGNAL_PROCMASK", semantic_id::kercall::SIGNAL_PROCMASK_64),
    (31, "SIGNAL_SUSPEND", semantic_id::kercall::SIGNAL_SUSPEND_64),
    (32, "SIGNAL_WAITINFO", semantic_id::kercall::SIGNAL_WAITINFO_64),
    (33, "SIGNAL_KILL_SIGVAL", semantic_id::kercall::SIGNAL_KILL_SIGVAL_64),
    (35, "CHANNEL_CREATE", semantic_id::kercall::CHANNEL_CREATE_64),
    (36, "CHANNEL_DESTROY", semantic_id::kercall::CHANNEL_DESTROY_64),
    (37, "CHANNEL_CTL", semantic_id::kercall::CHANNEL_CTL_64),
    (39, "CONNECT_ATTACH", semantic_id::kercall::CONNECT_ATTACH_64),
    (40, "CONNECT_DETACH", semantic_id::kercall::CONNECT_DETACH_64),
    (41, "CONNECT_SERVER_INFO", semantic_id::kercall::CONNECT_SERVER_INFO_64),
    (42, "CONNECT_CLIENT_INFO", semantic_id::kercall::CONNECT_CLIENT_INFO_64),
    (43, "CONNECT_FLAGS", semantic_id::kercall::CONNECT_FLAGS_64),
    (44, "CONNECT_DUPFD", semantic_id::kercall::CONNECT_DUPFD_64),
    (46, "THREAD_CREATE", semantic_id::kercall::THREAD_CREATE_64),
    (47, "THREAD_DESTROY", semantic_id::kercall::THREAD_DESTROY_64),
    (49, "THREAD_DETACH", semantic_id::kercall::THREAD_DETACH_64),
    (50, "THREAD_JOIN", semantic_id::kercall::THREAD_JOIN_64),
    (51, "THREAD_CANCEL", semantic_id::kercall::THREAD_CANCEL_64),
    (52, "THREAD_CTL", semantic_id::kercall::THREAD_CTL_64),
    (53, "THREAD_CTLEXT", semantic_id::kercall::THREAD_CTLEXT_64),
    (55, "INTERRUPT_ATTACH", semantic_id::kercall::INTERRUPT_ATTACH_64),
    (56, "INTERRUPT_QUERY", semantic_id::kercall::INTERRUPT_QUERY_64),
    (57, "INTERRUPT_DETACH", semantic_id::kercall::INTERRUPT_DETACH_64),
    (58, "INTERRUPT_WAIT", semantic_id::kercall::INTERRUPT_WAIT_64),
    (59, "INTERRUPT_MASK", semantic_id::kercall::INTERRUPT_MASK_64),
    (60, "INTERRUPT_UNMASK", semantic_id::kercall::INTERRUPT_UNMASK_64),
    (61, "INTERRUPT_UNBLOCK", semantic_id::kercall::INTERRUPT_UNBLOCK_64),
    (62, "INTERRUPT_CTL", semantic_id::kercall::INTERRUPT_CTL_64),
    (65, "CLOCK_TIME", semantic_id::kercall::CLOCK_TIME_64),
    (66, "CLOCK_ADJUST", semantic_id::kercall::CLOCK_ADJUST_64),
    (67, "CLOCK_PERIOD", semantic_id::kercall::CLOCK_PERIOD_64),
    (68, "CLOCK_ID", semantic_id::kercall::CLOCK_ID_64),
    (70, "TIMER_CREATE", semantic_id::kercall::TIMER_CREATE_64),
    (71, "TIMER_DESTROY", semantic_id::kercall::TIMER_DESTROY_64),
    (72, "TIMER_SETTIME", semantic_id::kercall::TIMER_SETTIME_64),
    (73, "TIMER_INFO", semantic_id::kercall::TIMER_INFO_64),
    (74, "TIMER_ALARM", semantic_id::kercall::TIMER_ALARM_64),
    (75, "TIMER_TIMEOUT", semantic_id::kercall::TIMER_TIMEOUT_64),
    (76, "TIMER_DELEGATE", semantic_id::kercall::TIMER_DELEGATE_64),
    (77, "SYNC_RWLOCK", semantic_id::kercall::SYNC_RWLOCK_64),
    (78, "SYNC_CREATE", semantic_id::kercall::SYNC_CREATE_64),
    (79, "SYNC_DESTROY", semantic_id::kercall::SYNC_DESTROY_64),
    (80, "SYNC_MUTEX_LOCK", semantic_id::kercall::SYNC_MUTEX_LOCK_64),
    (81, "SYNC_MUTEX_UNLOCK", semantic_id::kercall::SYNC_MUTEX_UNLOCK_64),
    (82, "SYNC_CONDVAR_WAIT", semantic_id::kercall::SYNC_CONDVAR_WAIT_64),
    (83, "SYNC_CONDVAR_SIGNAL", semantic_id::kercall::SYNC_CONDVAR_SIGNAL_64),
    (84, "SYNC_SEM_POST", semantic_id::kercall::SYNC_SEM_POST_64),
    (85, "SYNC_SEM_WAIT", semantic_id::kercall::SYNC_SEM_WAIT_64),
    (86, "SYNC_CTL", semantic_id::kercall::SYNC_CTL_64),
    (87, "SYNC_BARRIER_WAIT", semantic_id::kercall::SYNC_BARRIER_WAIT_64),
    (88, "SCHED_GET", semantic_id::kercall::SCHED_GET_64),
    (89, "SCHED_SET", semantic_id::kercall::SCHED_SET_64),
    (90, "SCHED_YIELD", semantic_id::kercall::SCHED_YIELD_64),
    (91, "SCHED_INFO", semantic_id::kercall::SCHED_INFO_64),
    (92, "SCHED_CTL", semantic_id::kercall::SCHED_CTL_64),
    (93, "MSG_QUEUE_OPEN", semantic_id::kercall::MSG_QUEUE_OPEN_64),
    (94, "MSG_QUEUE_SEND", semantic_id::kercall::MSG_QUEUE_SEND_64),
    (95, "MSG_QUEUE_RECEIVE", semantic_id::kercall::MSG_QUEUE_RECEIVE_64),
    (96, "MSG_QUEUE_CLOSE", semantic_id::kercall::MSG_QUEUE_CLOSE_64),
    (97, "MSG_QUEUE_CTL", semantic_id::kercall::MSG_QUEUE_CTL_64),
    (98, "PIPE_OPEN", semantic_id::kercall::PIPE_OPEN_64),
    (99, "PIPE_CTL", semantic_id::kercall::PIPE_CTL_64),
    (100, "PIPE_CLOSE", semantic_id::kercall::PIPE_CLOSE_64),
    (104, "SYNC_TYPE_DESTROY", semantic_id::kercall::SYNC_TYPE_DESTROY_64),
    (105, "TEST", semantic_id::kercall::TEST_64),
    (106, "SYS_CUSTOM", semantic_id::kercall::SYS_CUSTOM_64),
    (107, "BAD", semantic_id::kercall::BAD_64),
];
/// KERCALLS_64 の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const KERCALLS_64_PREFIX: &str = "__KER_";

/// スレッドの状態(STATE_*)。PR_TH のイベント番号
pub const THREAD_STATES: &[(u16, &str, EventId)] = &[
    (0, "DEAD", semantic_id::thread::DEAD),
    (1, "RUNNING", semantic_id::thread::RUNNING),
    (2, "READY", semantic_id::thread::READY),
    (3, "STOPPED", semantic_id::thread::STOPPED),
    (4, "SEND", semantic_id::thread::SEND),
    (5, "RECEIVE", semantic_id::thread::RECEIVE),
    (6, "REPLY", semantic_id::thread::REPLY),
    (7, "MQ_SEND", semantic_id::thread::MQ_SEND),
    (8, "MQ_RECEIVE", semantic_id::thread::MQ_RECEIVE),
    (9, "WAITPAGE", semantic_id::thread::WAITPAGE),
    (10, "SIGSUSPEND", semantic_id::thread::SIGSUSPEND),
    (11, "SIGWAITINFO", semantic_id::thread::SIGWAITINFO),
    (12, "NANOSLEEP", semantic_id::thread::NANOSLEEP),
    (13, "MUTEX", semantic_id::thread::MUTEX),
    (14, "CONDVAR", semantic_id::thread::CONDVAR),
    (15, "JOIN", semantic_id::thread::JOIN),
    (16, "INTR", semantic_id::thread::INTR),
    (17, "SEM", semantic_id::thread::SEM),
    (18, "WAITCTX", semantic_id::thread::WAITCTX),
    (19, "RWLOCK_READ", semantic_id::thread::RWLOCK_READ),
    (20, "RWLOCK_WRITE", semantic_id::thread::RWLOCK_WRITE),
    (21, "BARRIER", semantic_id::thread::BARRIER),
    (22, "PIPE", semantic_id::thread::PIPE),
    (24, "CREATE", semantic_id::thread::CREATE),
    (25, "DESTROY", semantic_id::thread::DESTROY),
    (26, "MUON_MUTEX", semantic_id::thread::MUON_MUTEX),
    (27, "TRACEBUFFER", semantic_id::thread::TRACEBUFFER),
    (28, "INTR_ATTACH_EV", semantic_id::thread::INTR_ATTACH_EV),
    (29, "TIMER_DELEGATE", semantic_id::thread::TIMER_DELEGATE),
];
/// THREAD_STATES の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const THREAD_STATES_PREFIX: &str = "STATE_";

/// 仮想スレッドの生成・破棄(_TRACE_PR_TH_*_VT)
pub const PR_TH_VTHREAD: &[(u16, &str, EventId)] = &[
    (87, "CREATE_VT", semantic_id::thread::CREATE_VT),
    (88, "DESTROY_VT", semantic_id::thread::DESTROY_VT),
];
/// PR_TH_VTHREAD の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const PR_TH_VTHREAD_PREFIX: &str = "_TRACE_PR_TH_";

/// プロセスのイベント(_TRACE_PR_TH_*)
pub const PR_TH_PROCESS: &[(u16, &str, EventId)] = &[
    (64, "CREATE_P", semantic_id::process::CREATE_P),
    (128, "DESTROY_P", semantic_id::process::DESTROY_P),
    (192, "CREATE_P_NAME", semantic_id::process::CREATE_P_NAME),
    (256, "DESTROY_P_NAME", semantic_id::process::DESTROY_P_NAME),
    (320, "NAME_T", semantic_id::process::NAME_T),
];
/// PR_TH_PROCESS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const PR_TH_PROCESS_PREFIX: &str = "_TRACE_PR_TH_";

/// CONTROL のイベント
pub const CONTROL_EVENTS: &[(u16, &str, EventId)] = &[
    (1, "TIME", semantic_id::control::TIME),
    (2, "BUFFER", semantic_id::control::BUFFER),
    (4, "BUFFER_END", semantic_id::control::BUFFER_END),
    (8, "DROPPED", semantic_id::control::DROPPED),
];
/// CONTROL_EVENTS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const CONTROL_EVENTS_PREFIX: &str = "_TRACE_CONTROL_";

/// INT のイベント(_TRACE_INT_*)
pub const INT_EVENTS: &[(u16, &str, EventId)] = &[
    (7, "DELIVER", semantic_id::int::DELIVER),
];
/// INT_EVENTS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const INT_EVENTS_PREFIX: &str = "_TRACE_INT_";

/// SYSTEM のイベント(_NTO_TRACE_SYS_*)
pub const SYSTEM_EVENTS: &[(u16, &str, EventId)] = &[
    (1, "RESERVED", semantic_id::system::RESERVED),
    (2, "PATHMGR", semantic_id::system::PATHMGR),
    (3, "APS_NAME", semantic_id::system::APS_NAME),
    (4, "APS_BUDGETS", semantic_id::system::APS_BUDGETS),
    (5, "APS_BNKR", semantic_id::system::APS_BNKR),
    (6, "MMAP", semantic_id::system::MMAP),
    (7, "MUNMAP", semantic_id::system::MUNMAP),
    (8, "MAPNAME", semantic_id::system::MAPNAME),
    (9, "ADDRESS", semantic_id::system::ADDRESS),
    (10, "FUNC_ENTER", semantic_id::system::FUNC_ENTER),
    (11, "FUNC_EXIT", semantic_id::system::FUNC_EXIT),
    (12, "SLOG", semantic_id::system::SLOG),
    (13, "DEFRAG_START", semantic_id::system::DEFRAG_START),
    (14, "RUNSTATE", semantic_id::system::RUNSTATE),
    (15, "POWER", semantic_id::system::POWER),
    (16, "IPI", semantic_id::system::IPI),
    (17, "PAGEWAIT", semantic_id::system::PAGEWAIT),
    (18, "TIMER", semantic_id::system::TIMER),
    (19, "DEFRAG_END", semantic_id::system::DEFRAG_END),
    (20, "PROFILE", semantic_id::system::PROFILE),
    (21, "MAPNAME_64", semantic_id::system::MAPNAME_64),
    (22, "APS_PSTATS", semantic_id::system::APS_PSTATS),
    (23, "APS_OSTATS", semantic_id::system::APS_OSTATS),
    (24, "APS_INFO", semantic_id::system::APS_INFO),
    (25, "APS_JOIN", semantic_id::system::APS_JOIN),
    (26, "APS_THREAD", semantic_id::system::APS_THREAD),
    (27, "APS_PROCESS", semantic_id::system::APS_PROCESS),
    (28, "SCHED_CONF", semantic_id::system::SCHED_CONF),
    (29, "IST_ATTACH", semantic_id::system::IST_ATTACH),
    (30, "IST_DETACH", semantic_id::system::IST_DETACH),
    (31, "PCTRACE", semantic_id::system::PCTRACE),
];
/// SYSTEM_EVENTS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const SYSTEM_EVENTS_PREFIX: &str = "_NTO_TRACE_SYS_";

/// SYSTEM の 64 ビット版(_NTO_TRACE_SYS_*_64、TableOfEvents の |_NTO_TRACE_KERCALL64): (BIT64 を足した番号, 名前, セマンティック ID)
pub const SYSTEM_EVENTS_64: &[(u16, &str, EventId)] = &[
    (521, "ADDRESS", semantic_id::system::ADDRESS_64),
    (522, "FUNC_ENTER", semantic_id::system::FUNC_ENTER_64),
    (523, "FUNC_EXIT", semantic_id::system::FUNC_EXIT_64),
    (528, "IPI", semantic_id::system::IPI_64),
    (532, "PROFILE", semantic_id::system::PROFILE_64),
];
/// SYSTEM_EVENTS_64 の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const SYSTEM_EVENTS_64_PREFIX: &str = "_NTO_TRACE_SYS_";

/// COMM のイベント(_NTO_TRACE_COMM_*)
pub const COMM_EVENTS: &[(u16, &str, EventId)] = &[
    (0, "SMSG", semantic_id::comm::SMSG),
    (1, "SPULSE", semantic_id::comm::SPULSE),
    (2, "RMSG", semantic_id::comm::RMSG),
    (3, "RPULSE", semantic_id::comm::RPULSE),
    (4, "SPULSE_EXE", semantic_id::comm::SPULSE_EXE),
    (5, "SPULSE_DIS", semantic_id::comm::SPULSE_DIS),
    (6, "SPULSE_DEA", semantic_id::comm::SPULSE_DEA),
    (7, "SPULSE_UN", semantic_id::comm::SPULSE_UN),
    (8, "SPULSE_QUN", semantic_id::comm::SPULSE_QUN),
    (9, "SIGNAL", semantic_id::comm::SIGNAL),
    (10, "REPLY", semantic_id::comm::REPLY),
    (11, "ERROR", semantic_id::comm::ERROR),
];
/// COMM_EVENTS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const COMM_EVENTS_PREFIX: &str = "_NTO_TRACE_COMM_";

/// SEC のイベント(_NTO_TRACE_SEC_*)
pub const SEC_EVENTS: &[(u16, &str, EventId)] = &[
    (0, "ABLE", semantic_id::sec::ABLE),
    (1, "ABLE_LOOKUP", semantic_id::sec::ABLE_LOOKUP),
    (2, "PATH_ATTACH", semantic_id::sec::PATH_ATTACH),
    (3, "QNET_CONNECT", semantic_id::sec::QNET_CONNECT),
    (4, "PERM_LOOKUP", semantic_id::sec::PERM_LOOKUP),
    (5, "PERM_TEST", semantic_id::sec::PERM_TEST),
    (6, "UNREG_EVENT", semantic_id::sec::UNREG_EVENT),
    (7, "OPEN_EVENT", semantic_id::sec::OPEN_EVENT),
];
/// SEC_EVENTS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const SEC_EVENTS_PREFIX: &str = "_NTO_TRACE_SEC_";

/// QVM のイベント(_NTO_TRACE_QVM_*)
pub const QVM_EVENTS: &[(u16, &str, EventId)] = &[
    (0, "GUEST_ENTER", semantic_id::qvm::GUEST_ENTER),
    (1, "GUEST_EXIT", semantic_id::qvm::GUEST_EXIT),
    (2, "CREATE_VCPU_THREAD", semantic_id::qvm::CREATE_VCPU_THREAD),
    (3, "RAISE_INTR", semantic_id::qvm::RAISE_INTR),
    (4, "LOWER_INTR", semantic_id::qvm::LOWER_INTR),
    (5, "TIMER_CREATE", semantic_id::qvm::TIMER_CREATE),
    (6, "TIMER_FIRE", semantic_id::qvm::TIMER_FIRE),
    (7, "CYCLES", semantic_id::qvm::CYCLES),
];
/// QVM_EVENTS の名前の接頭辞(ヘッダの定義名 = 接頭辞 + 名前)
pub const QVM_EVENTS_PREFIX: &str = "_NTO_TRACE_QVM_";
