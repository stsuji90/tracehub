//! QNX 8.0 の内部 ID の定義。
//!
//! 生成ファイル(tools/gen_qnx_def.py)。手で編集しない。
//! 入力: SDP のヘッダ(sys/trace.h、sys/kercalls.h、sys/states.h、sys/nto_version.h)と TableOfEvents。

use crate::ids::{self, EventId};

// ---- header のビット配置(_NTO_TRACE_GETEVENT・_NTO_TRACE_GETEVENT_C・_NTO_TRACE_GETCPU・_TRACE_GET_STRUCT)
pub const EVENT_SHIFT: u32 = 0;
pub const EVENT_MASK: u32 = 0x3ff;
pub const CLASS_SHIFT: u32 = 10;
pub const CLASS_MASK: u32 = 0x1f;
pub const CPU_SHIFT: u32 = 24;
pub const CPU_MASK: u32 = 0x3f;
pub const STRUCT_SHIFT: u32 = 30;
pub const STRUCT_MASK: u32 = 0x3;

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

// ---- PR_TH のイベント番号: これより小さければスレッドの状態(_TRACE_PR_TH_CREATE_P)
pub const PR_TH_PROCESS_BASE: u16 = 64;

/// カーネルコール(__KER_*): (コール番号, 名前, セマンティック ID)
pub const KERCALLS: &[(u16, &str, EventId)] = &[
    (0, "NOP", ids::kercall::NOP),
    (1, "TRACE_EVENT", ids::kercall::TRACE_EVENT),
    (2, "RING0", ids::kercall::RING0),
    (3, "CACHE_CONTROL", ids::kercall::CACHE_CONTROL),
    (4, "SYS_SRANDOM", ids::kercall::SYS_SRANDOM),
    (5, "MSG_REGISTER_EVENT", ids::kercall::MSG_REGISTER_EVENT),
    (6, "MSG_SEND_PULSEPTR", ids::kercall::MSG_SEND_PULSEPTR),
    (7, "SYS_CPUPAGE_GET", ids::kercall::SYS_CPUPAGE_GET),
    (8, "SYS_CPUPAGE_SET", ids::kercall::SYS_CPUPAGE_SET),
    (9, "MSG_PAUSE", ids::kercall::MSG_PAUSE),
    (10, "MSG_CURRENT", ids::kercall::MSG_CURRENT),
    (11, "MSG_SENDV", ids::kercall::MSG_SENDV),
    (12, "MSG_SENDVNC", ids::kercall::MSG_SENDVNC),
    (13, "MSG_ERROR", ids::kercall::MSG_ERROR),
    (14, "MSG_RECEIVEV", ids::kercall::MSG_RECEIVEV),
    (15, "MSG_REPLYV", ids::kercall::MSG_REPLYV),
    (16, "MSG_READV", ids::kercall::MSG_READV),
    (17, "MSG_WRITEV", ids::kercall::MSG_WRITEV),
    (19, "MSG_INFO", ids::kercall::MSG_INFO),
    (20, "MSG_SEND_PULSE", ids::kercall::MSG_SEND_PULSE),
    (21, "MSG_DELIVER_EVENT", ids::kercall::MSG_DELIVER_EVENT),
    (24, "MSG_RECEIVEPULSEV", ids::kercall::MSG_RECEIVEPULSEV),
    (25, "MSG_VERIFY_EVENT", ids::kercall::MSG_VERIFY_EVENT),
    (26, "SIGNAL_KILL", ids::kercall::SIGNAL_KILL),
    (27, "SIGNAL_RETURN", ids::kercall::SIGNAL_RETURN),
    (29, "SIGNAL_ACTION", ids::kercall::SIGNAL_ACTION),
    (30, "SIGNAL_PROCMASK", ids::kercall::SIGNAL_PROCMASK),
    (31, "SIGNAL_SUSPEND", ids::kercall::SIGNAL_SUSPEND),
    (32, "SIGNAL_WAITINFO", ids::kercall::SIGNAL_WAITINFO),
    (33, "SIGNAL_KILL_SIGVAL", ids::kercall::SIGNAL_KILL_SIGVAL),
    (35, "CHANNEL_CREATE", ids::kercall::CHANNEL_CREATE),
    (36, "CHANNEL_DESTROY", ids::kercall::CHANNEL_DESTROY),
    (37, "CHANNEL_CTL", ids::kercall::CHANNEL_CTL),
    (39, "CONNECT_ATTACH", ids::kercall::CONNECT_ATTACH),
    (40, "CONNECT_DETACH", ids::kercall::CONNECT_DETACH),
    (41, "CONNECT_SERVER_INFO", ids::kercall::CONNECT_SERVER_INFO),
    (42, "CONNECT_CLIENT_INFO", ids::kercall::CONNECT_CLIENT_INFO),
    (43, "CONNECT_FLAGS", ids::kercall::CONNECT_FLAGS),
    (44, "CONNECT_DUPFD", ids::kercall::CONNECT_DUPFD),
    (46, "THREAD_CREATE", ids::kercall::THREAD_CREATE),
    (47, "THREAD_DESTROY", ids::kercall::THREAD_DESTROY),
    (49, "THREAD_DETACH", ids::kercall::THREAD_DETACH),
    (50, "THREAD_JOIN", ids::kercall::THREAD_JOIN),
    (51, "THREAD_CANCEL", ids::kercall::THREAD_CANCEL),
    (52, "THREAD_CTL", ids::kercall::THREAD_CTL),
    (53, "THREAD_CTLEXT", ids::kercall::THREAD_CTLEXT),
    (55, "INTERRUPT_ATTACH", ids::kercall::INTERRUPT_ATTACH),
    (56, "INTERRUPT_QUERY", ids::kercall::INTERRUPT_QUERY),
    (57, "INTERRUPT_DETACH", ids::kercall::INTERRUPT_DETACH),
    (58, "INTERRUPT_WAIT", ids::kercall::INTERRUPT_WAIT),
    (59, "INTERRUPT_MASK", ids::kercall::INTERRUPT_MASK),
    (60, "INTERRUPT_UNMASK", ids::kercall::INTERRUPT_UNMASK),
    (61, "INTERRUPT_UNBLOCK", ids::kercall::INTERRUPT_UNBLOCK),
    (62, "INTERRUPT_CTL", ids::kercall::INTERRUPT_CTL),
    (65, "CLOCK_TIME", ids::kercall::CLOCK_TIME),
    (66, "CLOCK_ADJUST", ids::kercall::CLOCK_ADJUST),
    (67, "CLOCK_PERIOD", ids::kercall::CLOCK_PERIOD),
    (68, "CLOCK_ID", ids::kercall::CLOCK_ID),
    (70, "TIMER_CREATE", ids::kercall::TIMER_CREATE),
    (71, "TIMER_DESTROY", ids::kercall::TIMER_DESTROY),
    (72, "TIMER_SETTIME", ids::kercall::TIMER_SETTIME),
    (73, "TIMER_INFO", ids::kercall::TIMER_INFO),
    (74, "TIMER_ALARM", ids::kercall::TIMER_ALARM),
    (75, "TIMER_TIMEOUT", ids::kercall::TIMER_TIMEOUT),
    (76, "TIMER_DELEGATE", ids::kercall::TIMER_DELEGATE),
    (77, "SYNC_RWLOCK", ids::kercall::SYNC_RWLOCK),
    (78, "SYNC_CREATE", ids::kercall::SYNC_CREATE),
    (79, "SYNC_DESTROY", ids::kercall::SYNC_DESTROY),
    (80, "SYNC_MUTEX_LOCK", ids::kercall::SYNC_MUTEX_LOCK),
    (81, "SYNC_MUTEX_UNLOCK", ids::kercall::SYNC_MUTEX_UNLOCK),
    (82, "SYNC_CONDVAR_WAIT", ids::kercall::SYNC_CONDVAR_WAIT),
    (83, "SYNC_CONDVAR_SIGNAL", ids::kercall::SYNC_CONDVAR_SIGNAL),
    (84, "SYNC_SEM_POST", ids::kercall::SYNC_SEM_POST),
    (85, "SYNC_SEM_WAIT", ids::kercall::SYNC_SEM_WAIT),
    (86, "SYNC_CTL", ids::kercall::SYNC_CTL),
    (87, "SYNC_BARRIER_WAIT", ids::kercall::SYNC_BARRIER_WAIT),
    (88, "SCHED_GET", ids::kercall::SCHED_GET),
    (89, "SCHED_SET", ids::kercall::SCHED_SET),
    (90, "SCHED_YIELD", ids::kercall::SCHED_YIELD),
    (91, "SCHED_INFO", ids::kercall::SCHED_INFO),
    (92, "SCHED_CTL", ids::kercall::SCHED_CTL),
    (93, "MSG_QUEUE_OPEN", ids::kercall::MSG_QUEUE_OPEN),
    (94, "MSG_QUEUE_SEND", ids::kercall::MSG_QUEUE_SEND),
    (95, "MSG_QUEUE_RECEIVE", ids::kercall::MSG_QUEUE_RECEIVE),
    (96, "MSG_QUEUE_CLOSE", ids::kercall::MSG_QUEUE_CLOSE),
    (97, "MSG_QUEUE_CTL", ids::kercall::MSG_QUEUE_CTL),
    (98, "PIPE_OPEN", ids::kercall::PIPE_OPEN),
    (99, "PIPE_CTL", ids::kercall::PIPE_CTL),
    (100, "PIPE_CLOSE", ids::kercall::PIPE_CLOSE),
    (104, "SYNC_TYPE_DESTROY", ids::kercall::SYNC_TYPE_DESTROY),
    (105, "TEST", ids::kercall::TEST),
    (106, "SYS_CUSTOM", ids::kercall::SYS_CUSTOM),
    (107, "BAD", ids::kercall::BAD),
];

/// スレッドの状態(STATE_*)。PR_TH のイベント番号
pub const THREAD_STATES: &[(u16, &str, EventId)] = &[
    (0, "DEAD", ids::thread::DEAD),
    (1, "RUNNING", ids::thread::RUNNING),
    (2, "READY", ids::thread::READY),
    (3, "STOPPED", ids::thread::STOPPED),
    (4, "SEND", ids::thread::SEND),
    (5, "RECEIVE", ids::thread::RECEIVE),
    (6, "REPLY", ids::thread::REPLY),
    (7, "MQ_SEND", ids::thread::MQ_SEND),
    (8, "MQ_RECEIVE", ids::thread::MQ_RECEIVE),
    (9, "WAITPAGE", ids::thread::WAITPAGE),
    (10, "SIGSUSPEND", ids::thread::SIGSUSPEND),
    (11, "SIGWAITINFO", ids::thread::SIGWAITINFO),
    (12, "NANOSLEEP", ids::thread::NANOSLEEP),
    (13, "MUTEX", ids::thread::MUTEX),
    (14, "CONDVAR", ids::thread::CONDVAR),
    (15, "JOIN", ids::thread::JOIN),
    (16, "INTR", ids::thread::INTR),
    (17, "SEM", ids::thread::SEM),
    (18, "WAITCTX", ids::thread::WAITCTX),
    (19, "RWLOCK_READ", ids::thread::RWLOCK_READ),
    (20, "RWLOCK_WRITE", ids::thread::RWLOCK_WRITE),
    (21, "BARRIER", ids::thread::BARRIER),
    (22, "PIPE", ids::thread::PIPE),
    (24, "CREATE", ids::thread::CREATE),
    (25, "DESTROY", ids::thread::DESTROY),
    (26, "MUON_MUTEX", ids::thread::MUON_MUTEX),
    (27, "TRACEBUFFER", ids::thread::TRACEBUFFER),
    (28, "INTR_ATTACH_EV", ids::thread::INTR_ATTACH_EV),
    (29, "TIMER_DELEGATE", ids::thread::TIMER_DELEGATE),
];

/// 仮想スレッドの生成・破棄(_TRACE_PR_TH_*_VT)
pub const PR_TH_VTHREAD: &[(u16, &str, EventId)] = &[
    (87, "CREATE_VT", ids::thread::CREATE_VT),
    (88, "DESTROY_VT", ids::thread::DESTROY_VT),
];

/// プロセスのイベント(_TRACE_PR_TH_*)
pub const PR_TH_PROCESS: &[(u16, &str, EventId)] = &[
    (64, "CREATE_P", ids::process::CREATE_P),
    (128, "DESTROY_P", ids::process::DESTROY_P),
    (192, "CREATE_P_NAME", ids::process::CREATE_P_NAME),
    (256, "DESTROY_P_NAME", ids::process::DESTROY_P_NAME),
    (320, "NAME_T", ids::process::NAME_T),
];

/// CONTROL のイベント
pub const CONTROL_EVENTS: &[(u16, &str, EventId)] = &[
    (1, "TIME", ids::control::TIME),
    (2, "BUFFER", ids::control::BUFFER),
    (4, "BUFFER_END", ids::control::BUFFER_END),
    (8, "DROPPED", ids::control::DROPPED),
];

/// INT のイベント(_TRACE_INT_*)
pub const INT_EVENTS: &[(u16, &str, EventId)] = &[
    (7, "DELIVER", ids::int::DELIVER),
];

/// SYSTEM のイベント(_NTO_TRACE_SYS_*)。64 ビット版は BIT64 を足した番号
pub const SYSTEM_EVENTS: &[(u16, &str, EventId)] = &[
    (1, "RESERVED", ids::system::RESERVED),
    (2, "PATHMGR", ids::system::PATHMGR),
    (3, "APS_NAME", ids::system::APS_NAME),
    (4, "APS_BUDGETS", ids::system::APS_BUDGETS),
    (5, "APS_BNKR", ids::system::APS_BNKR),
    (6, "MMAP", ids::system::MMAP),
    (7, "MUNMAP", ids::system::MUNMAP),
    (8, "MAPNAME", ids::system::MAPNAME),
    (9, "ADDRESS", ids::system::ADDRESS),
    (10, "FUNC_ENTER", ids::system::FUNC_ENTER),
    (11, "FUNC_EXIT", ids::system::FUNC_EXIT),
    (12, "SLOG", ids::system::SLOG),
    (13, "DEFRAG_START", ids::system::DEFRAG_START),
    (14, "RUNSTATE", ids::system::RUNSTATE),
    (15, "POWER", ids::system::POWER),
    (16, "IPI", ids::system::IPI),
    (17, "PAGEWAIT", ids::system::PAGEWAIT),
    (18, "TIMER", ids::system::TIMER),
    (19, "DEFRAG_END", ids::system::DEFRAG_END),
    (20, "PROFILE", ids::system::PROFILE),
    (21, "MAPNAME_64", ids::system::MAPNAME_64),
    (22, "APS_PSTATS", ids::system::APS_PSTATS),
    (23, "APS_OSTATS", ids::system::APS_OSTATS),
    (24, "APS_INFO", ids::system::APS_INFO),
    (25, "APS_JOIN", ids::system::APS_JOIN),
    (26, "APS_THREAD", ids::system::APS_THREAD),
    (27, "APS_PROCESS", ids::system::APS_PROCESS),
    (28, "SCHED_CONF", ids::system::SCHED_CONF),
    (29, "IST_ATTACH", ids::system::IST_ATTACH),
    (30, "IST_DETACH", ids::system::IST_DETACH),
    (31, "PCTRACE", ids::system::PCTRACE),
];

/// COMM のイベント(_NTO_TRACE_COMM_*)
pub const COMM_EVENTS: &[(u16, &str, EventId)] = &[
    (0, "SMSG", ids::comm::SMSG),
    (1, "SPULSE", ids::comm::SPULSE),
    (2, "RMSG", ids::comm::RMSG),
    (3, "RPULSE", ids::comm::RPULSE),
    (4, "SPULSE_EXE", ids::comm::SPULSE_EXE),
    (5, "SPULSE_DIS", ids::comm::SPULSE_DIS),
    (6, "SPULSE_DEA", ids::comm::SPULSE_DEA),
    (7, "SPULSE_UN", ids::comm::SPULSE_UN),
    (8, "SPULSE_QUN", ids::comm::SPULSE_QUN),
    (9, "SIGNAL", ids::comm::SIGNAL),
    (10, "REPLY", ids::comm::REPLY),
    (11, "ERROR", ids::comm::ERROR),
];

/// SEC のイベント(_NTO_TRACE_SEC_*)
pub const SEC_EVENTS: &[(u16, &str, EventId)] = &[
    (0, "ABLE", ids::sec::ABLE),
    (1, "ABLE_LOOKUP", ids::sec::ABLE_LOOKUP),
    (2, "PATH_ATTACH", ids::sec::PATH_ATTACH),
    (3, "QNET_CONNECT", ids::sec::QNET_CONNECT),
    (4, "PERM_LOOKUP", ids::sec::PERM_LOOKUP),
    (5, "PERM_TEST", ids::sec::PERM_TEST),
    (6, "UNREG_EVENT", ids::sec::UNREG_EVENT),
    (7, "OPEN_EVENT", ids::sec::OPEN_EVENT),
];

/// QVM のイベント(_NTO_TRACE_QVM_*)
pub const QVM_EVENTS: &[(u16, &str, EventId)] = &[
    (0, "GUEST_ENTER", ids::qvm::GUEST_ENTER),
    (1, "GUEST_EXIT", ids::qvm::GUEST_EXIT),
    (2, "CREATE_VCPU_THREAD", ids::qvm::CREATE_VCPU_THREAD),
    (3, "RAISE_INTR", ids::qvm::RAISE_INTR),
    (4, "LOWER_INTR", ids::qvm::LOWER_INTR),
    (5, "TIMER_CREATE", ids::qvm::TIMER_CREATE),
    (6, "TIMER_FIRE", ids::qvm::TIMER_FIRE),
    (7, "CYCLES", ids::qvm::CYCLES),
];
