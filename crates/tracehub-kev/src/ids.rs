//! セマンティック ID(QNX の版に依存しない番号)。
//!
//! - 手で保守する。一度決めた番号は変えない。追加だけする
//! - クラス ID とイベント ID の組でイベントの種類を表す
//! - KER_CALL の enter / exit / int はクラスで、64 ビット版はイベント ID の [`EventId::BIT64`] で区別する
//! - 番号が任意のイベント(USER・QUIP)は、kev の内部イベント番号をそのままイベント ID にする
//! - 解釈できなかったものは [`ClassId::UNKNOWN`] / [`EventId::UNKNOWN`]

/// クラス ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassId(pub u16);

/// イベント ID(クラスごとの番号)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventId(pub u16);

impl ClassId {
    pub const UNKNOWN: ClassId = ClassId(0xFFFF);

    pub const CONTROL: ClassId = ClassId(1);
    pub const KER_CALL_ENTER: ClassId = ClassId(2);
    pub const KER_CALL_EXIT: ClassId = ClassId(3);
    pub const KER_CALL_INT: ClassId = ClassId(4);
    pub const INT: ClassId = ClassId(5);
    pub const THREAD: ClassId = ClassId(6);
    pub const PROCESS: ClassId = ClassId(7);
    pub const SYSTEM: ClassId = ClassId(8);
    pub const USER: ClassId = ClassId(9);
    pub const COMM: ClassId = ClassId(10);
    pub const QUIP: ClassId = ClassId(11);
    pub const SEC: ClassId = ClassId(12);
    pub const QVM: ClassId = ClassId(13);

    pub fn is_unknown(self) -> bool {
        self == Self::UNKNOWN
    }
}

impl EventId {
    pub const UNKNOWN: EventId = EventId(0xFFFF);
    /// 64 ビット版(KER_CALL・SYSTEM の引数が 64 ビットの変種)を表すビット
    pub const BIT64: u16 = 0x8000;

    pub fn is_unknown(self) -> bool {
        self == Self::UNKNOWN
    }
    /// 64 ビット版か
    pub fn is_64bit(self) -> bool {
        !self.is_unknown() && self.0 & Self::BIT64 != 0
    }
    /// 64 ビット版の ID
    pub const fn bit64(self) -> EventId {
        EventId(self.0 | Self::BIT64)
    }
}

/// CONTROL のイベント
pub mod control {
    use super::EventId;

    pub const TIME: EventId = EventId(1);
    pub const BUFFER: EventId = EventId(2);
    pub const BUFFER_END: EventId = EventId(3);
    pub const DROPPED: EventId = EventId(4);
}

/// カーネルコール(KER_CALL_ENTER / KER_CALL_EXIT / KER_CALL_INT で共通)
pub mod kercall {
    use super::EventId;

    pub const NOP: EventId = EventId(1);
    pub const TRACE_EVENT: EventId = EventId(2);
    pub const RING0: EventId = EventId(3);
    pub const CACHE_CONTROL: EventId = EventId(4);
    pub const SYS_SRANDOM: EventId = EventId(5);
    pub const MSG_REGISTER_EVENT: EventId = EventId(6);
    pub const MSG_SEND_PULSEPTR: EventId = EventId(7);
    pub const SYS_CPUPAGE_GET: EventId = EventId(8);
    pub const SYS_CPUPAGE_SET: EventId = EventId(9);
    pub const MSG_PAUSE: EventId = EventId(10);
    pub const MSG_CURRENT: EventId = EventId(11);
    pub const MSG_SENDV: EventId = EventId(12);
    pub const MSG_SENDVNC: EventId = EventId(13);
    pub const MSG_ERROR: EventId = EventId(14);
    pub const MSG_RECEIVEV: EventId = EventId(15);
    pub const MSG_REPLYV: EventId = EventId(16);
    pub const MSG_READV: EventId = EventId(17);
    pub const MSG_WRITEV: EventId = EventId(18);
    pub const MSG_INFO: EventId = EventId(19);
    pub const MSG_SEND_PULSE: EventId = EventId(20);
    pub const MSG_DELIVER_EVENT: EventId = EventId(21);
    pub const MSG_RECEIVEPULSEV: EventId = EventId(22);
    pub const MSG_VERIFY_EVENT: EventId = EventId(23);
    pub const SIGNAL_KILL: EventId = EventId(24);
    pub const SIGNAL_RETURN: EventId = EventId(25);
    pub const SIGNAL_ACTION: EventId = EventId(26);
    pub const SIGNAL_PROCMASK: EventId = EventId(27);
    pub const SIGNAL_SUSPEND: EventId = EventId(28);
    pub const SIGNAL_WAITINFO: EventId = EventId(29);
    pub const SIGNAL_KILL_SIGVAL: EventId = EventId(30);
    pub const CHANNEL_CREATE: EventId = EventId(31);
    pub const CHANNEL_DESTROY: EventId = EventId(32);
    pub const CHANNEL_CTL: EventId = EventId(33);
    pub const CONNECT_ATTACH: EventId = EventId(34);
    pub const CONNECT_DETACH: EventId = EventId(35);
    pub const CONNECT_SERVER_INFO: EventId = EventId(36);
    pub const CONNECT_CLIENT_INFO: EventId = EventId(37);
    pub const CONNECT_FLAGS: EventId = EventId(38);
    pub const CONNECT_DUPFD: EventId = EventId(39);
    pub const THREAD_CREATE: EventId = EventId(40);
    pub const THREAD_DESTROY: EventId = EventId(41);
    pub const THREAD_DETACH: EventId = EventId(42);
    pub const THREAD_JOIN: EventId = EventId(43);
    pub const THREAD_CANCEL: EventId = EventId(44);
    pub const THREAD_CTL: EventId = EventId(45);
    pub const THREAD_CTLEXT: EventId = EventId(46);
    pub const INTERRUPT_ATTACH: EventId = EventId(47);
    pub const INTERRUPT_QUERY: EventId = EventId(48);
    pub const INTERRUPT_DETACH: EventId = EventId(49);
    pub const INTERRUPT_WAIT: EventId = EventId(50);
    pub const INTERRUPT_MASK: EventId = EventId(51);
    pub const INTERRUPT_UNMASK: EventId = EventId(52);
    pub const INTERRUPT_UNBLOCK: EventId = EventId(53);
    pub const INTERRUPT_CTL: EventId = EventId(54);
    pub const CLOCK_TIME: EventId = EventId(55);
    pub const CLOCK_ADJUST: EventId = EventId(56);
    pub const CLOCK_PERIOD: EventId = EventId(57);
    pub const CLOCK_ID: EventId = EventId(58);
    pub const TIMER_CREATE: EventId = EventId(59);
    pub const TIMER_DESTROY: EventId = EventId(60);
    pub const TIMER_SETTIME: EventId = EventId(61);
    pub const TIMER_INFO: EventId = EventId(62);
    pub const TIMER_ALARM: EventId = EventId(63);
    pub const TIMER_TIMEOUT: EventId = EventId(64);
    pub const TIMER_DELEGATE: EventId = EventId(65);
    pub const SYNC_RWLOCK: EventId = EventId(66);
    pub const SYNC_CREATE: EventId = EventId(67);
    pub const SYNC_DESTROY: EventId = EventId(68);
    pub const SYNC_MUTEX_LOCK: EventId = EventId(69);
    pub const SYNC_MUTEX_UNLOCK: EventId = EventId(70);
    pub const SYNC_CONDVAR_WAIT: EventId = EventId(71);
    pub const SYNC_CONDVAR_SIGNAL: EventId = EventId(72);
    pub const SYNC_SEM_POST: EventId = EventId(73);
    pub const SYNC_SEM_WAIT: EventId = EventId(74);
    pub const SYNC_CTL: EventId = EventId(75);
    pub const SYNC_BARRIER_WAIT: EventId = EventId(76);
    pub const SCHED_GET: EventId = EventId(77);
    pub const SCHED_SET: EventId = EventId(78);
    pub const SCHED_YIELD: EventId = EventId(79);
    pub const SCHED_INFO: EventId = EventId(80);
    pub const SCHED_CTL: EventId = EventId(81);
    pub const MSG_QUEUE_OPEN: EventId = EventId(82);
    pub const MSG_QUEUE_SEND: EventId = EventId(83);
    pub const MSG_QUEUE_RECEIVE: EventId = EventId(84);
    pub const MSG_QUEUE_CLOSE: EventId = EventId(85);
    pub const MSG_QUEUE_CTL: EventId = EventId(86);
    pub const PIPE_OPEN: EventId = EventId(87);
    pub const PIPE_CTL: EventId = EventId(88);
    pub const PIPE_CLOSE: EventId = EventId(89);
    pub const SYNC_TYPE_DESTROY: EventId = EventId(90);
    pub const TEST: EventId = EventId(91);
    pub const SYS_CUSTOM: EventId = EventId(92);
    pub const BAD: EventId = EventId(93);
}

/// INT のイベント
pub mod int {
    use super::EventId;

    pub const DELIVER: EventId = EventId(1);
}

/// THREAD のイベント(スレッドの状態と仮想スレッド)
pub mod thread {
    use super::EventId;

    pub const DEAD: EventId = EventId(1);
    pub const RUNNING: EventId = EventId(2);
    pub const READY: EventId = EventId(3);
    pub const STOPPED: EventId = EventId(4);
    pub const SEND: EventId = EventId(5);
    pub const RECEIVE: EventId = EventId(6);
    pub const REPLY: EventId = EventId(7);
    pub const MQ_SEND: EventId = EventId(8);
    pub const MQ_RECEIVE: EventId = EventId(9);
    pub const WAITPAGE: EventId = EventId(10);
    pub const SIGSUSPEND: EventId = EventId(11);
    pub const SIGWAITINFO: EventId = EventId(12);
    pub const NANOSLEEP: EventId = EventId(13);
    pub const MUTEX: EventId = EventId(14);
    pub const CONDVAR: EventId = EventId(15);
    pub const JOIN: EventId = EventId(16);
    pub const INTR: EventId = EventId(17);
    pub const SEM: EventId = EventId(18);
    pub const WAITCTX: EventId = EventId(19);
    pub const RWLOCK_READ: EventId = EventId(20);
    pub const RWLOCK_WRITE: EventId = EventId(21);
    pub const BARRIER: EventId = EventId(22);
    pub const PIPE: EventId = EventId(23);
    pub const CREATE: EventId = EventId(24);
    pub const DESTROY: EventId = EventId(25);
    pub const MUON_MUTEX: EventId = EventId(26);
    pub const TRACEBUFFER: EventId = EventId(27);
    pub const INTR_ATTACH_EV: EventId = EventId(28);
    pub const TIMER_DELEGATE: EventId = EventId(29);
    pub const CREATE_VT: EventId = EventId(30);
    pub const DESTROY_VT: EventId = EventId(31);
}

/// PROCESS のイベント
pub mod process {
    use super::EventId;

    pub const CREATE_P: EventId = EventId(1);
    pub const DESTROY_P: EventId = EventId(2);
    pub const CREATE_P_NAME: EventId = EventId(3);
    pub const DESTROY_P_NAME: EventId = EventId(4);
    pub const NAME_T: EventId = EventId(5);
}

/// SYSTEM のイベント
pub mod system {
    use super::EventId;

    pub const RESERVED: EventId = EventId(1);
    pub const PATHMGR: EventId = EventId(2);
    pub const APS_NAME: EventId = EventId(3);
    pub const APS_BUDGETS: EventId = EventId(4);
    pub const APS_BNKR: EventId = EventId(5);
    pub const MMAP: EventId = EventId(6);
    pub const MUNMAP: EventId = EventId(7);
    pub const MAPNAME: EventId = EventId(8);
    pub const ADDRESS: EventId = EventId(9);
    pub const FUNC_ENTER: EventId = EventId(10);
    pub const FUNC_EXIT: EventId = EventId(11);
    pub const SLOG: EventId = EventId(12);
    pub const DEFRAG_START: EventId = EventId(13);
    pub const RUNSTATE: EventId = EventId(14);
    pub const POWER: EventId = EventId(15);
    pub const IPI: EventId = EventId(16);
    pub const PAGEWAIT: EventId = EventId(17);
    pub const TIMER: EventId = EventId(18);
    pub const DEFRAG_END: EventId = EventId(19);
    pub const PROFILE: EventId = EventId(20);
    pub const MAPNAME_64: EventId = EventId(21);
    pub const APS_PSTATS: EventId = EventId(22);
    pub const APS_OSTATS: EventId = EventId(23);
    pub const APS_INFO: EventId = EventId(24);
    pub const APS_JOIN: EventId = EventId(25);
    pub const APS_THREAD: EventId = EventId(26);
    pub const APS_PROCESS: EventId = EventId(27);
    pub const SCHED_CONF: EventId = EventId(28);
    pub const IST_ATTACH: EventId = EventId(29);
    pub const IST_DETACH: EventId = EventId(30);
    pub const PCTRACE: EventId = EventId(31);
}

/// COMM のイベント
pub mod comm {
    use super::EventId;

    pub const SMSG: EventId = EventId(1);
    pub const SPULSE: EventId = EventId(2);
    pub const RMSG: EventId = EventId(3);
    pub const RPULSE: EventId = EventId(4);
    pub const SPULSE_EXE: EventId = EventId(5);
    pub const SPULSE_DIS: EventId = EventId(6);
    pub const SPULSE_DEA: EventId = EventId(7);
    pub const SPULSE_UN: EventId = EventId(8);
    pub const SPULSE_QUN: EventId = EventId(9);
    pub const SIGNAL: EventId = EventId(10);
    pub const REPLY: EventId = EventId(11);
    pub const ERROR: EventId = EventId(12);
}

/// SEC のイベント
pub mod sec {
    use super::EventId;

    pub const ABLE: EventId = EventId(1);
    pub const ABLE_LOOKUP: EventId = EventId(2);
    pub const PATH_ATTACH: EventId = EventId(3);
    pub const QNET_CONNECT: EventId = EventId(4);
    pub const PERM_LOOKUP: EventId = EventId(5);
    pub const PERM_TEST: EventId = EventId(6);
    pub const UNREG_EVENT: EventId = EventId(7);
    pub const OPEN_EVENT: EventId = EventId(8);
}

/// QVM のイベント
pub mod qvm {
    use super::EventId;

    pub const GUEST_ENTER: EventId = EventId(1);
    pub const GUEST_EXIT: EventId = EventId(2);
    pub const CREATE_VCPU_THREAD: EventId = EventId(3);
    pub const RAISE_INTR: EventId = EventId(4);
    pub const LOWER_INTR: EventId = EventId(5);
    pub const TIMER_CREATE: EventId = EventId(6);
    pub const TIMER_FIRE: EventId = EventId(7);
    pub const CYCLES: EventId = EventId(8);
}
