# セマンティック ID

tracehub がイベントの種類を表すのに使う、QNX の版に依存しない番号の説明と一覧。
定義は `crates/tracehub-kev/src/semantic_id.rs`。設計書(design.md)§8 の詳細にあたる。

---

## 1. なぜ要るか

kev のスロットの header には、イベントの種類が**内部 ID**(内部クラス ID と内部イベント番号)で書かれている。
内部 ID は QNX のヘッダ(`sys/trace.h`・`sys/kercalls.h` など)が決める値で、版によって変わりうる。

- カーネルコールの番号(`__KER_*`)は、コールの追加・削除で付け替わることがある。
- クラスの番号やビット配置が変わることもある(例: QNX 7.1 の `_NTO_TRACE_RESERVEDCLASS1` のように、ある版にしか無いクラス)。

tracehub を使う側(分析のプログラムなど)が内部 ID を直接見ると、版ごとに書き分けることになる。
そこで tracehub は、内部 ID を**意味(QNX の名前)ごとに決めた番号**に読み替えて渡す。これがセマンティック ID である。

```
kev の header ──(版の def.rs)──▶ 内部 ID (内部クラス, 内部イベント番号) ──(版の event_parser.rs)──▶ セマンティック ID (class_id, event_id)
                                   例: QNX 8.0 の (2, 0x08b)                                             例: (KER_CALL_EXIT, kercall::MSG_SENDV)
```

---

## 2. 形

```rust
pub struct ClassId(pub u16);   // クラス
pub struct EventId(pub u16);   // クラスの中のイベント
```

- イベントの種類は **(class_id, event_id) の組**で表す。event_id はクラスごとの番号で、それだけでは種類が決まらない。
- 解釈できなかったものは `ClassId::UNKNOWN` / `EventId::UNKNOWN`(どちらも `0xFFFF`)。
- 番号は連番である必要はない。比較とマップのキーにだけ使い、配列の添字や範囲には使わない。

### クラス ID

`ClassId` の定数(`ClassId::CONTROL` など)。定数名と ID の一覧は `ClassId::ALL`、定数名は `ClassId::const_name()` で引ける。

### イベント ID

クラスごとのモジュール(`semantic_id::kercall`・`semantic_id::system` など)に定数で置く。
定数名は QNX のヘッダの名前から接頭辞を除いたもの(`__KER_MSG_SENDV` → `kercall::MSG_SENDV`、`_NTO_TRACE_SYS_SLOG` → `system::SLOG`)。

---

## 3. 内部 ID との対応の規則

| 内容 | 規則 |
|---|---|
| カーネルコール | 内部では 1 つのクラス(KER_CALL)で、内部イベント番号にコール番号と種別のビット(exit・int)が入っている。セマンティック ID では種別を**クラス**で分け(`KER_CALL_ENTER`・`KER_CALL_EXIT`・`KER_CALL_INT`)、イベント ID はコールごとに共通(`kercall::MSG_SENDV`) |
| 64 ビット版 | 内部イベント番号の 64 ビットのビットが立ったものは、**別のイベント ID**(`kercall::MSG_SENDV_64`・`system::IPI_64`)。番号の上で 64 ビットでない版と関係は無い |
| カーネルコールの 64 ビット版 | **全コール**に置く。TableOfEvents に記載の無い 64 ビット版も、カーネルは記録し traceprinter は表示するため |
| SYSTEM の 64 ビット版 | ヘッダに `_NTO_TRACE_SYS_*_64` があるか、TableOfEvents に `\|_NTO_TRACE_KERCALL64` があるものだけ。それ以外の番号は UNKNOWN |
| スレッドとプロセス | 内部では 1 つのクラス(PR_TH)。スレッドの状態と仮想スレッドは `THREAD`、プロセスの生成・破棄・名前は `PROCESS` に分ける |
| USER・QUIP | イベント番号が任意(アプリが決める)なので定数を置かず、**内部イベント番号をそのまま** event_id にする |
| 定義の無いもの | 対応表に無い内部クラス(クラス 0 や未定義の番号)は class_id も event_id も UNKNOWN。クラスはあるがイベントが無いものは、class_id はそのクラスで event_id が UNKNOWN。exit と int の両方のビットが立った KER_CALL は class_id が UNKNOWN |

QNX 8.0 での例(試験 `semantic_round_trip` で確かめている):

| 内部 ID | セマンティック ID |
|---|---|
| (KER_CALL, 11) | (`KER_CALL_ENTER`, `kercall::MSG_SENDV`) |
| (KER_CALL, 11 \| exit \| 64 ビット) | (`KER_CALL_EXIT`, `kercall::MSG_SENDV_64`) |
| (PR_TH, 1) | (`THREAD`, `thread::RUNNING`) |
| (PR_TH, 192) | (`PROCESS`, `process::CREATE_P_NAME`) |
| (SYSTEM, 12) | (`SYSTEM`, `system::SLOG`) |
| (SYSTEM, 16 \| 64 ビット) | (`SYSTEM`, `system::IPI_64`) |
| (SYSTEM, 12 \| 64 ビット) | (`SYSTEM`, UNKNOWN)(SLOG の 64 ビット版は定義が無い) |
| (USER, 77) | (`USER`, `EventId(77)`) |
| (0, 0) | (UNKNOWN, UNKNOWN) |

変換の関数は、版ごとの `qnx<版>/event_parser.rs` の `semantic_ids()`(内部 → セマンティック)と `internal_ids()`(その逆)。

---

## 4. 表示名との違い

セマンティック ID の**定数名**と、traceprinter が表示する**表示名**は別のもの。

| | 定数名(セマンティック ID) | 表示名(traceprinter。QNX 8.0) |
|---|---|---|
| クラス | `KER_CALL_EXIT`、`INT`、`USER` | `KER_EXIT`、`INTERRUPT`、`USREVENT` |
| カーネルコール | `kercall::MSG_SENDV` | `MSG_SENDV/11`(コール名/コール番号。番号は版で変わりうる) |
| 割り込まれたカーネルコール | `kercall::MSG_SENDV`(クラス `KER_CALL_INT`) | クラス `INT_CALL`、名前 `KER_MSG_SENDV/11`(引数なしの形で表示するもの) |
| スレッドの状態 | `thread::RUNNING` | `THRUNNING` |
| 64 ビット版 | `system::IPI_64` | `IPI`(64 ビットでない版と同じ名前) |

表示名は `tracehub_kev::class_name(version, class_id)` / `event_name(version, class_id, event_id)` で引く(版を引数にとる)。

---

## 5. 使い方

```rust
use tracehub_kev::{ClassId, semantic_id::kercall};

for ev in &kev.events {
    if ev.class_id == ClassId::KER_CALL_ENTER && ev.event_id == kercall::MSG_SENDV {
        // MSG_SENDV の呼び出し。payloads の意味は TableOfEvents を参照
    }
    let name = tracehub_kev::event_name(kev.header.version, ev.class_id, ev.event_id); // 表示名
}
```

- 内部 ID も `TraceEvent` の `int_class_id`・`int_event_id` に残っている。版に依存してよい処理(traceprinter 互換出力など)はそちらを使う。
- UNKNOWN のイベントも捨てずに events に入る(`display_status` が Unknown か Silent)。

---

## 6. 足す・変えるときの規則

- **番号は追加だけ**。一度決めた番号は変えない・消さない・使い回さない。版のサポートを切っても消さない。
- 新しい定数は、そのモジュールの末尾に、番号はモジュールの最大 + 1 で足す。
- 新しいクラスは、`ClassId` の定数と `ClassId::ALL` の両方に足す(試験 `class_ids_are_unique` が重複を見つける)。
- ある版で同じ名前のまま意味が変わったら、新しい ID を足すかを判断する(意味が同じなら同じ ID)。
- def.rs(生成ファイル)は semantic_id.rs の定数を名前で参照する。版の def.rs を作ったときに「`semantic_id::xxx::NAME` が無い」とビルドが失敗したら、ここに足す(手順は qnx-version-guide.md §2)。
- 足したら、下の一覧を書き直す: `TRACEHUB_UPDATE_DOCS=1 cargo test`(PowerShell では `$env:TRACEHUB_UPDATE_DOCS=1; cargo test`)。
  一覧が古いままだと、試験 `semantic_id_doc_is_up_to_date` が失敗する。

---

## 7. 一覧

semantic_id.rs と、版ごとの定義(def.rs・args.rs)から生成したもの。

- 内部イベント番号は 16 進。カーネルコールは enter / exit / int の順に並べる。「—」はその版に無いもの(表示名の「—」は、名前の決まらないもの)。
- カーネルコールの KER_INT の列が `KER_名前/番号` のものは、traceprinter が表示クラス名 `INT_CALL`・引数なしの形で表示するもの。
- 表示名は traceprinter が表示する名前。traceprinter がそのイベントの行を出すかどうかは、ここには書かない(確認用のイベント一覧 `event_catalog.py` を使う。testing.md §6)。

<!-- 生成部分ここから(cargo test が確かめる。書き直すときは TRACEHUB_UPDATE_DOCS=1 cargo test) -->
### クラス

| class_id | 定数 | QNX 8.0: 内部クラス | QNX 8.0: 表示クラス名 |
|---|---|---|---|
| 1 | `CONTROL` | 1 | CONTROL |
| 2 | `KER_CALL_ENTER` | 2 | KER_CALL |
| 3 | `KER_CALL_EXIT` | 2 | KER_EXIT |
| 4 | `KER_CALL_INT` | 2 | KER_INT |
| 5 | `INT` | 3 | INTERRUPT |
| 6 | `THREAD` | 4 | THREAD |
| 7 | `PROCESS` | 4 | PROCESS |
| 8 | `SYSTEM` | 5 | SYSTEM |
| 9 | `USER` | 6 | USREVENT |
| 10 | `COMM` | 7 | COMM |
| 11 | `QUIP` | 8 | QUIP |
| 12 | `SEC` | 9 | SEC |
| 13 | `QVM` | 10 | QVM |
| 65535 | `UNKNOWN` | — | — |

### `control`: CONTROL のイベント

クラス: `CONTROL`。4 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `control::TIME` | 0x001 | TIME |
| 2 | `control::BUFFER` | 0x002 | BUFFER |
| 3 | `control::BUFFER_END` | 0x004 | BUFFER_END |
| 4 | `control::DROPPED` | 0x008 | DROPPED |

### `kercall`: カーネルコール(KER_CALL_ENTER / KER_CALL_EXIT / KER_CALL_INT で共通)

クラス: `KER_CALL_ENTER`・`KER_CALL_EXIT`・`KER_CALL_INT`。186 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名(KER_CALL) | QNX 8.0: 表示名(KER_EXIT) | QNX 8.0: 表示名(KER_INT) |
|---|---|---|---|---|---|
| 1 | `kercall::NOP` | 0x000 / 0x080 / 0x100 | NOP/00 | NOP/00 | KER_NOP/0 |
| 2 | `kercall::TRACE_EVENT` | 0x001 / 0x081 / 0x101 | TRACE_EVENT/01 | TRACE_EVENT/01 | KER_TRACE_EVENT/1 |
| 3 | `kercall::RING0` | 0x002 / 0x082 / 0x102 | RING0/02 | RING0/02 | KER_RING0/2 |
| 4 | `kercall::CACHE_CONTROL` | 0x003 / 0x083 / 0x103 | CACHE_CONTROL/03 | CACHE_CONTROL/03 | KER_CACHE_CONTROL/3 |
| 5 | `kercall::SYS_SRANDOM` | 0x004 / 0x084 / 0x104 | SYS_SRANDOM/04 | SYS_SRANDOM/04 | KER_SYS_SRANDOM/4 |
| 6 | `kercall::MSG_REGISTER_EVENT` | 0x005 / 0x085 / 0x105 | MSG_REGISTER_EVENT/05 | MSG_REGISTER_EVENT/05 | KER_MSG_REGISTER_EVENT/5 |
| 7 | `kercall::MSG_SEND_PULSEPTR` | 0x006 / 0x086 / 0x106 | MSG_SEND_PULSEPTR/06 | MSG_SEND_PULSEPTR/06 | KER_MSG_SEND_PULSEPTR/6 |
| 8 | `kercall::SYS_CPUPAGE_GET` | 0x007 / 0x087 / 0x107 | SYS_CPUPAGE_GET/07 | SYS_CPUPAGE_GET/07 | KER_SYS_CPUPAGE_GET/7 |
| 9 | `kercall::SYS_CPUPAGE_SET` | 0x008 / 0x088 / 0x108 | SYS_CPUPAGE_SET/08 | SYS_CPUPAGE_SET/08 | KER_SYS_CPUPAGE_SET/8 |
| 10 | `kercall::MSG_PAUSE` | 0x009 / 0x089 / 0x109 | MSG_PAUSE/09 | KER_MSG_PAUSE/9 | KER_MSG_PAUSE/9 |
| 11 | `kercall::MSG_CURRENT` | 0x00a / 0x08a / 0x10a | MSG_CURRENT/10 | MSG_CURRENT/10 | KER_MSG_CURRENT/10 |
| 12 | `kercall::MSG_SENDV` | 0x00b / 0x08b / 0x10b | MSG_SENDV/11 | MSG_SENDV/11 | KER_MSG_SENDV/11 |
| 13 | `kercall::MSG_SENDVNC` | 0x00c / 0x08c / 0x10c | MSG_SENDVNC/12 | MSG_SENDVNC/12 | KER_MSG_SENDVNC/12 |
| 14 | `kercall::MSG_ERROR` | 0x00d / 0x08d / 0x10d | MSG_ERROR/13 | MSG_ERROR/13 | KER_MSG_ERROR/13 |
| 15 | `kercall::MSG_RECEIVEV` | 0x00e / 0x08e / 0x10e | MSG_RECEIVEV/14 | MSG_RECEIVEV/14 | KER_MSG_RECEIVEV/14 |
| 16 | `kercall::MSG_REPLYV` | 0x00f / 0x08f / 0x10f | MSG_REPLYV/15 | MSG_REPLYV/15 | KER_MSG_REPLYV/15 |
| 17 | `kercall::MSG_READV` | 0x010 / 0x090 / 0x110 | MSG_READV/16 | MSG_READV/16 | KER_MSG_READV/16 |
| 18 | `kercall::MSG_WRITEV` | 0x011 / 0x091 / 0x111 | MSG_WRITEV/17 | MSG_WRITEV/17 | KER_MSG_WRITEV/17 |
| 19 | `kercall::MSG_INFO` | 0x013 / 0x093 / 0x113 | MSG_INFO/19 | MSG_INFO/19 | KER_MSG_INFO/19 |
| 20 | `kercall::MSG_SEND_PULSE` | 0x014 / 0x094 / 0x114 | MSG_SEND_PULSE/20 | MSG_SEND_PULSE/20 | KER_MSG_SEND_PULSE/20 |
| 21 | `kercall::MSG_DELIVER_EVENT` | 0x015 / 0x095 / 0x115 | MSG_DELIVER_EVENT/21 | MSG_DELIVER_EVENT/21 | KER_MSG_DELIVER_EVENT/21 |
| 22 | `kercall::MSG_RECEIVEPULSEV` | 0x018 / 0x098 / 0x118 | MSG_RECEIVEPULSEV/24 | MSG_RECEIVEPULSEV/24 | KER_MSG_RECEIVEPULSEV/24 |
| 23 | `kercall::MSG_VERIFY_EVENT` | 0x019 / 0x099 / 0x119 | MSG_VERIFY_EVENT/25 | MSG_VERIFY_EVENT/25 | KER_MSG_VERIFY_EVENT/25 |
| 24 | `kercall::SIGNAL_KILL` | 0x01a / 0x09a / 0x11a | SIGNAL_KILL/26 | SIGNAL_KILL/26 | KER_SIGNAL_KILL/26 |
| 25 | `kercall::SIGNAL_RETURN` | 0x01b / 0x09b / 0x11b | SIGNAL_RETURN/27 | SIGNAL_RETURN/27 | KER_SIGNAL_RETURN/27 |
| 26 | `kercall::SIGNAL_ACTION` | 0x01d / 0x09d / 0x11d | SIGNAL_ACTION/29 | SIGNAL_ACTION/29 | KER_SIGNAL_ACTION/29 |
| 27 | `kercall::SIGNAL_PROCMASK` | 0x01e / 0x09e / 0x11e | SIGNAL_PROCMASK/30 | SIGNAL_PROCMASK/30 | KER_SIGNAL_PROCMASK/30 |
| 28 | `kercall::SIGNAL_SUSPEND` | 0x01f / 0x09f / 0x11f | SIGNAL_SUSPEND/31 | SIGNAL_SUSPEND/31 | KER_SIGNAL_SUSPEND/31 |
| 29 | `kercall::SIGNAL_WAITINFO` | 0x020 / 0x0a0 / 0x120 | SIGNAL_WAITINFO/32 | SIGNAL_WAITINFO/32 | KER_SIGNAL_WAITINFO/32 |
| 30 | `kercall::SIGNAL_KILL_SIGVAL` | 0x021 / 0x0a1 / 0x121 | SIGNAL_KILL/33 | SIGNAL_KILL/33 | KER_SIGNAL_KILL_SIGVAL/33 |
| 31 | `kercall::CHANNEL_CREATE` | 0x023 / 0x0a3 / 0x123 | CHANNEL_CREATE/35 | CHANNEL_CREATE/35 | KER_CHANNEL_CREATE/35 |
| 32 | `kercall::CHANNEL_DESTROY` | 0x024 / 0x0a4 / 0x124 | CHANNEL_DESTROY/36 | CHANNEL_DESTROY/36 | KER_CHANNEL_DESTROY/36 |
| 33 | `kercall::CHANNEL_CTL` | 0x025 / 0x0a5 / 0x125 | CHANNEL_CTL/37 | CHANNEL_CTL/37 | CHANNEL_CTL/37 |
| 34 | `kercall::CONNECT_ATTACH` | 0x027 / 0x0a7 / 0x127 | CONNECT_ATTACH/39 | CONNECT_ATTACH/39 | KER_CONNECT_ATTACH/39 |
| 35 | `kercall::CONNECT_DETACH` | 0x028 / 0x0a8 / 0x128 | CONNECT_DETACH/40 | CONNECT_DETACH/40 | KER_CONNECT_DETACH/40 |
| 36 | `kercall::CONNECT_SERVER_INFO` | 0x029 / 0x0a9 / 0x129 | CONNECT_SERVER_INFO/41 | CONNECT_SERVER_INFO/41 | KER_CONNECT_SERVER_INFO/41 |
| 37 | `kercall::CONNECT_CLIENT_INFO` | 0x02a / 0x0aa / 0x12a | CONNECT_CLIENT_INFO/42 | CONNECT_CLIENT_INFO/42 | KER_CONNECT_CLIENT_INFO/42 |
| 38 | `kercall::CONNECT_FLAGS` | 0x02b / 0x0ab / 0x12b | CONNECT_FLAGS/43 | CONNECT_FLAGS/43 | KER_CONNECT_FLAGS/43 |
| 39 | `kercall::CONNECT_DUPFD` | 0x02c / 0x0ac / 0x12c | CONNECT_DUPFD/44 | CONNECT_DUPFD/44 | KER_CONNECT_DUPFD/44 |
| 40 | `kercall::THREAD_CREATE` | 0x02e / 0x0ae / 0x12e | THREAD_CREATE/46 | THREAD_CREATE/46 | KER_THREAD_CREATE/46 |
| 41 | `kercall::THREAD_DESTROY` | 0x02f / 0x0af / 0x12f | THREAD_DESTROY/47 | THREAD_DESTROY/47 | KER_THREAD_DESTROY/47 |
| 42 | `kercall::THREAD_DETACH` | 0x031 / 0x0b1 / 0x131 | THREAD_DETACH/49 | THREAD_DETACH/49 | KER_THREAD_DETACH/49 |
| 43 | `kercall::THREAD_JOIN` | 0x032 / 0x0b2 / 0x132 | THREAD_JOIN/50 | THREAD_JOIN/50 | KER_THREAD_JOIN/50 |
| 44 | `kercall::THREAD_CANCEL` | 0x033 / 0x0b3 / 0x133 | THREAD_CANCEL/51 | THREAD_CANCEL/51 | KER_THREAD_CANCEL/51 |
| 45 | `kercall::THREAD_CTL` | 0x034 / 0x0b4 / 0x134 | THREAD_CTL/52 | THREAD_CTL/52 | KER_THREAD_CTL/52 |
| 46 | `kercall::THREAD_CTLEXT` | 0x035 / 0x0b5 / 0x135 | THREAD_CTLEXT/53 | THREAD_CTLEXT/53 | KER_THREAD_CTLEXT/53 |
| 47 | `kercall::INTERRUPT_ATTACH` | 0x037 / 0x0b7 / 0x137 | INTERRUPT_ATTACH/55 | INTERRUPT_ATTACH/55 | KER_INTERRUPT_ATTACH/55 |
| 48 | `kercall::INTERRUPT_QUERY` | 0x038 / 0x0b8 / 0x138 | INTERRUPT_QUERY/56 | INTERRUPT_QUERY/56 | INTERRUPT_QUERY/56 |
| 49 | `kercall::INTERRUPT_DETACH` | 0x039 / 0x0b9 / 0x139 | INTERRUPT_DETACH/57 | INTERRUPT_DETACH/57 | KER_INTERRUPT_DETACH/57 |
| 50 | `kercall::INTERRUPT_WAIT` | 0x03a / 0x0ba / 0x13a | INTERRUPT_WAIT/58 | INTERRUPT_WAIT/58 | KER_INTERRUPT_WAIT/58 |
| 51 | `kercall::INTERRUPT_MASK` | 0x03b / 0x0bb / 0x13b | INTERRUPT_MASK/59 | INTERRUPT_MASK/59 | KER_INTERRUPT_MASK/59 |
| 52 | `kercall::INTERRUPT_UNMASK` | 0x03c / 0x0bc / 0x13c | INTERRUPT_UNMASK/60 | INTERRUPT_UNMASK/60 | KER_INTERRUPT_UNMASK/60 |
| 53 | `kercall::INTERRUPT_UNBLOCK` | 0x03d / 0x0bd / 0x13d | INTERRUPT_UNBLOCK/61 | INTERRUPT_UNBLOCK/61 | INTERRUPT_UNBLOCK/61 |
| 54 | `kercall::INTERRUPT_CTL` | 0x03e / 0x0be / 0x13e | INTERRUPT_CTL/62 | INTERRUPT_CTL/62 | INTERRUPT_CTL/62 |
| 55 | `kercall::CLOCK_TIME` | 0x041 / 0x0c1 / 0x141 | CLOCK_TIME/65 | CLOCK_TIME/65 | KER_CLOCK_TIME/65 |
| 56 | `kercall::CLOCK_ADJUST` | 0x042 / 0x0c2 / 0x142 | CLOCK_ADJUST/66 | CLOCK_ADJUST/66 | KER_CLOCK_ADJUST/66 |
| 57 | `kercall::CLOCK_PERIOD` | 0x043 / 0x0c3 / 0x143 | CLOCK_PERIOD/67 | CLOCK_PERIOD/67 | KER_CLOCK_PERIOD/67 |
| 58 | `kercall::CLOCK_ID` | 0x044 / 0x0c4 / 0x144 | CLOCK_ID/68 | CLOCK_ID/68 | KER_CLOCK_ID/68 |
| 59 | `kercall::TIMER_CREATE` | 0x046 / 0x0c6 / 0x146 | TIMER_CREATE/70 | TIMER_CREATE/70 | KER_TIMER_CREATE/70 |
| 60 | `kercall::TIMER_DESTROY` | 0x047 / 0x0c7 / 0x147 | TIMER_DESTROY/71 | TIMER_DESTROY/71 | KER_TIMER_DESTROY/71 |
| 61 | `kercall::TIMER_SETTIME` | 0x048 / 0x0c8 / 0x148 | TIMER_SETTIME/72 | TIMER_SETTIME/72 | KER_TIMER_SETTIME/72 |
| 62 | `kercall::TIMER_INFO` | 0x049 / 0x0c9 / 0x149 | TIMER_INFO/73 | TIMER_INFO/73 | KER_TIMER_INFO/73 |
| 63 | `kercall::TIMER_ALARM` | 0x04a / 0x0ca / 0x14a | TIMER_ALARM/74 | TIMER_ALARM/74 | KER_TIMER_ALARM/74 |
| 64 | `kercall::TIMER_TIMEOUT` | 0x04b / 0x0cb / 0x14b | TIMER_TIMEOUT/75 | TIMER_TIMEOUT/75 | KER_TIMER_TIMEOUT/75 |
| 65 | `kercall::TIMER_DELEGATE` | 0x04c / 0x0cc / 0x14c | TIMER_DELEGATE/76 | TIMER_DELEGATE/76 | TIMER_DELEGATE/76 |
| 66 | `kercall::SYNC_RWLOCK` | 0x04d / 0x0cd / 0x14d | SYNC_RWLOCK/77 | SYNC_RWLOCK/77 | SYNC_RWLOCK/77 |
| 67 | `kercall::SYNC_CREATE` | 0x04e / 0x0ce / 0x14e | SYNC_CREATE/78 | SYNC_CREATE/78 | KER_SYNC_CREATE/78 |
| 68 | `kercall::SYNC_DESTROY` | 0x04f / 0x0cf / 0x14f | SYNC_DESTROY/79 | SYNC_DESTROY/79 | KER_SYNC_DESTROY/79 |
| 69 | `kercall::SYNC_MUTEX_LOCK` | 0x050 / 0x0d0 / 0x150 | SYNC_MUTEX_LOCK/80 | SYNC_MUTEX_LOCK/80 | KER_SYNC_MUTEX_LOCK/80 |
| 70 | `kercall::SYNC_MUTEX_UNLOCK` | 0x051 / 0x0d1 / 0x151 | SYNC_MUTEX_UNLOCK/81 | SYNC_MUTEX_UNLOCK/81 | KER_SYNC_MUTEX_UNLOCK/81 |
| 71 | `kercall::SYNC_CONDVAR_WAIT` | 0x052 / 0x0d2 / 0x152 | SYNC_CONDVAR_WAIT/82 | SYNC_CONDVAR_WAIT/82 | KER_SYNC_CONDVAR_WAIT/82 |
| 72 | `kercall::SYNC_CONDVAR_SIGNAL` | 0x053 / 0x0d3 / 0x153 | SYNC_CONDVAR_SIGNAL/83 | SYNC_CONDVAR_SIG/83 | KER_SYNC_CONDVAR_SIGNAL/83 |
| 73 | `kercall::SYNC_SEM_POST` | 0x054 / 0x0d4 / 0x154 | SYNC_SEM_POST/84 | SYNC_SEM_POST/84 | KER_SYNC_SEM_POST/84 |
| 74 | `kercall::SYNC_SEM_WAIT` | 0x055 / 0x0d5 / 0x155 | SYNC_SEM_WAIT/85 | SYNC_SEM_WAIT/85 | KER_SYNC_SEM_WAIT/85 |
| 75 | `kercall::SYNC_CTL` | 0x056 / 0x0d6 / 0x156 | SYNC_CTL/86 | SYNC_CTL/86 | KER_SYNC_CTL/86 |
| 76 | `kercall::SYNC_BARRIER_WAIT` | 0x057 / 0x0d7 / 0x157 | SYNC_BARRIER_WAIT/87 | SYNC_BARRIER_WAIT/87 | SYNC_BARRIER_WAIT/87 |
| 77 | `kercall::SCHED_GET` | 0x058 / 0x0d8 / 0x158 | SCHED_GET/88 | SCHED_GET/88 | KER_SCHED_GET/88 |
| 78 | `kercall::SCHED_SET` | 0x059 / 0x0d9 / 0x159 | SCHED_SET/89 | SCHED_SET/89 | KER_SCHED_SET/89 |
| 79 | `kercall::SCHED_YIELD` | 0x05a / 0x0da / 0x15a | SCHED_YIELD/90 | SCHED_YIELD/90 | KER_SCHED_YIELD/90 |
| 80 | `kercall::SCHED_INFO` | 0x05b / 0x0db / 0x15b | SCHED_INFO/91 | SCHED_INFO/91 | KER_SCHED_INFO/91 |
| 81 | `kercall::SCHED_CTL` | 0x05c / 0x0dc / 0x15c | SCHED_CTL/92 | SCHED_CTL/92 | KER_SCHED_CTL/92 |
| 82 | `kercall::MSG_QUEUE_OPEN` | 0x05d / 0x0dd / 0x15d | MSG_QUEUE_OPEN/93 | MSG_QUEUE_OPEN/93 | MSG_QUEUE_OPEN/93 |
| 83 | `kercall::MSG_QUEUE_SEND` | 0x05e / 0x0de / 0x15e | MSG_QUEUE_SEND/94 | MSG_QUEUE_SEND/94 | MSG_QUEUE_SEND/94 |
| 84 | `kercall::MSG_QUEUE_RECEIVE` | 0x05f / 0x0df / 0x15f | MSG_QUEUE_RECEIVE/95 | MSG_QUEUE_RECEIVE/95 | MSG_QUEUE_RECEIVE/95 |
| 85 | `kercall::MSG_QUEUE_CLOSE` | 0x060 / 0x0e0 / 0x160 | MSG_QUEUE_CLOSE/96 | MSG_QUEUE_CLOSE/96 | MSG_QUEUE_CLOSE/96 |
| 86 | `kercall::MSG_QUEUE_CTL` | 0x061 / 0x0e1 / 0x161 | MSG_QUEUE_CTL/97 | MSG_QUEUE_CTL/97 | MSG_QUEUE_CTL/97 |
| 87 | `kercall::PIPE_OPEN` | 0x062 / 0x0e2 / 0x162 | PIPE_OPEN/98 | PIPE_OPEN/98 | PIPE_OPEN/98 |
| 88 | `kercall::PIPE_CTL` | 0x063 / 0x0e3 / 0x163 | PIPE_CTL/99 | PIPE_CTL/99 | PIPE_CTL/99 |
| 89 | `kercall::PIPE_CLOSE` | 0x064 / 0x0e4 / 0x164 | PIPE_CLOSE/100 | PIPE_CLOSE/100 | PIPE_CLOSE/100 |
| 90 | `kercall::SYNC_TYPE_DESTROY` | 0x068 / 0x0e8 / 0x168 | SYNC_TYPE_DESTROY/104 | SYNC_TYPE_DESTROY/104 | KER_SYNC_TYPE_DESTROY/104 |
| 91 | `kercall::TEST` | 0x069 / 0x0e9 / 0x169 | TEST/105 | TEST/105 | TEST/105 |
| 92 | `kercall::SYS_CUSTOM` | 0x06a / 0x0ea / 0x16a | SYS_CUSTOM/106 | SYS_CUSTOM/106 | SYS_CUSTOM/106 |
| 93 | `kercall::BAD` | 0x06b / 0x0eb / 0x16b | BAD/107 | BAD/107 | KER_BAD/107 |
| 94 | `kercall::NOP_64` | 0x200 / 0x280 / 0x300 | NOP/00 | NOP/00 | NOP/00 |
| 95 | `kercall::TRACE_EVENT_64` | 0x201 / 0x281 / 0x301 | TRACE_EVENT/01 | TRACE_EVENT/01 | TRACE_EVENT/01 |
| 96 | `kercall::RING0_64` | 0x202 / 0x282 / 0x302 | RING0/02 | RING0/02 | RING0/02 |
| 97 | `kercall::CACHE_CONTROL_64` | 0x203 / 0x283 / 0x303 | CACHE_CONTROL/03 | CACHE_CONTROL/03 | CACHE_CONTROL/03 |
| 98 | `kercall::SYS_SRANDOM_64` | 0x204 / 0x284 / 0x304 | SYS_SRANDOM/04 | SYS_SRANDOM/04 | SYS_SRANDOM/04 |
| 99 | `kercall::MSG_REGISTER_EVENT_64` | 0x205 / 0x285 / 0x305 | MSG_REGISTER_EVENT/05 | MSG_REGISTER_EVENT/05 | MSG_REGISTER_EVENT/05 |
| 100 | `kercall::MSG_SEND_PULSEPTR_64` | 0x206 / 0x286 / 0x306 | MSG_SEND_PULSEPTR/06 | MSG_SEND_PULSEPTR/06 | MSG_SEND_PULSEPTR/06 |
| 101 | `kercall::SYS_CPUPAGE_GET_64` | 0x207 / 0x287 / 0x307 | SYS_CPUPAGE_GET/07 | SYS_CPUPAGE_GET/07 | SYS_CPUPAGE_GET/07 |
| 102 | `kercall::SYS_CPUPAGE_SET_64` | 0x208 / 0x288 / 0x308 | SYS_CPUPAGE_SET/08 | SYS_CPUPAGE_SET/08 | SYS_CPUPAGE_SET/08 |
| 103 | `kercall::MSG_PAUSE_64` | 0x209 / 0x289 / 0x309 | MSG_PAUSE/09 | MSG_PAUSE/09 | MSG_PAUSE/09 |
| 104 | `kercall::MSG_CURRENT_64` | 0x20a / 0x28a / 0x30a | MSG_CURRENT/10 | MSG_CURRENT/10 | MSG_CURRENT/10 |
| 105 | `kercall::MSG_SENDV_64` | 0x20b / 0x28b / 0x30b | MSG_SENDV/11 | MSG_SENDV/11 | MSG_SENDV/11 |
| 106 | `kercall::MSG_SENDVNC_64` | 0x20c / 0x28c / 0x30c | MSG_SENDVNC/12 | MSG_SENDVNC/12 | MSG_SENDVNC/12 |
| 107 | `kercall::MSG_ERROR_64` | 0x20d / 0x28d / 0x30d | MSG_ERROR/13 | MSG_ERROR/13 | MSG_ERROR/13 |
| 108 | `kercall::MSG_RECEIVEV_64` | 0x20e / 0x28e / 0x30e | MSG_RECEIVEV/14 | MSG_RECEIVEV/14 | MSG_RECEIVEV/14 |
| 109 | `kercall::MSG_REPLYV_64` | 0x20f / 0x28f / 0x30f | MSG_REPLYV/15 | MSG_REPLYV/15 | MSG_REPLYV/15 |
| 110 | `kercall::MSG_READV_64` | 0x210 / 0x290 / 0x310 | MSG_READV/16 | MSG_READV/16 | MSG_READV/16 |
| 111 | `kercall::MSG_WRITEV_64` | 0x211 / 0x291 / 0x311 | MSG_WRITEV/17 | MSG_WRITEV/17 | MSG_WRITEV/17 |
| 112 | `kercall::MSG_INFO_64` | 0x213 / 0x293 / 0x313 | MSG_INFO/19 | MSG_INFO/19 | MSG_INFO/19 |
| 113 | `kercall::MSG_SEND_PULSE_64` | 0x214 / 0x294 / 0x314 | MSG_SEND_PULSE/20 | MSG_SEND_PULSE/20 | MSG_SEND_PULSE/20 |
| 114 | `kercall::MSG_DELIVER_EVENT_64` | 0x215 / 0x295 / 0x315 | MSG_DELIVER_EVENT/21 | MSG_DELIVER_EVENT/21 | MSG_DELIVER_EVENT/21 |
| 115 | `kercall::MSG_RECEIVEPULSEV_64` | 0x218 / 0x298 / 0x318 | MSG_RECEIVEPULSEV/24 | MSG_RECEIVEPULSEV/24 | MSG_RECEIVEPULSEV/24 |
| 116 | `kercall::MSG_VERIFY_EVENT_64` | 0x219 / 0x299 / 0x319 | MSG_VERIFY_EVENT/25 | MSG_VERIFY_EVENT/25 | MSG_VERIFY_EVENT/25 |
| 117 | `kercall::SIGNAL_KILL_64` | 0x21a / 0x29a / 0x31a | SIGNAL_KILL/26 | SIGNAL_KILL/26 | SIGNAL_KILL/26 |
| 118 | `kercall::SIGNAL_RETURN_64` | 0x21b / 0x29b / 0x31b | SIGNAL_RETURN/27 | SIGNAL_RETURN/27 | SIGNAL_RETURN/27 |
| 119 | `kercall::SIGNAL_ACTION_64` | 0x21d / 0x29d / 0x31d | SIGNAL_ACTION/29 | SIGNAL_ACTION/29 | SIGNAL_ACTION/29 |
| 120 | `kercall::SIGNAL_PROCMASK_64` | 0x21e / 0x29e / 0x31e | SIGNAL_PROCMASK/30 | SIGNAL_PROCMASK/30 | SIGNAL_PROCMASK/30 |
| 121 | `kercall::SIGNAL_SUSPEND_64` | 0x21f / 0x29f / 0x31f | SIGNAL_SUSPEND/31 | SIGNAL_SUSPEND/31 | SIGNAL_SUSPEND/31 |
| 122 | `kercall::SIGNAL_WAITINFO_64` | 0x220 / 0x2a0 / 0x320 | SIGNAL_WAITINFO/32 | SIGNAL_WAITINFO/32 | SIGNAL_WAITINFO/32 |
| 123 | `kercall::SIGNAL_KILL_SIGVAL_64` | 0x221 / 0x2a1 / 0x321 | SIGNAL_KILL/33 | SIGNAL_KILL/33 | SIGNAL_KILL/33 |
| 124 | `kercall::CHANNEL_CREATE_64` | 0x223 / 0x2a3 / 0x323 | CHANNEL_CREATE/35 | CHANNEL_CREATE/35 | CHANNEL_CREATE/35 |
| 125 | `kercall::CHANNEL_DESTROY_64` | 0x224 / 0x2a4 / 0x324 | CHANNEL_DESTROY/36 | CHANNEL_DESTROY/36 | CHANNEL_DESTROY/36 |
| 126 | `kercall::CHANNEL_CTL_64` | 0x225 / 0x2a5 / 0x325 | CHANNEL_CTL/37 | CHANNEL_CTL/37 | CHANNEL_CTL/37 |
| 127 | `kercall::CONNECT_ATTACH_64` | 0x227 / 0x2a7 / 0x327 | CONNECT_ATTACH/39 | CONNECT_ATTACH/39 | CONNECT_ATTACH/39 |
| 128 | `kercall::CONNECT_DETACH_64` | 0x228 / 0x2a8 / 0x328 | CONNECT_DETACH/40 | CONNECT_DETACH/40 | CONNECT_DETACH/40 |
| 129 | `kercall::CONNECT_SERVER_INFO_64` | 0x229 / 0x2a9 / 0x329 | CONNECT_SERVER_INFO/41 | CONNECT_SERVER_INFO/41 | CONNECT_SERVER_INFO/41 |
| 130 | `kercall::CONNECT_CLIENT_INFO_64` | 0x22a / 0x2aa / 0x32a | CONNECT_CLIENT_INFO/42 | CONNECT_CLIENT_INFO/42 | CONNECT_CLIENT_INFO/42 |
| 131 | `kercall::CONNECT_FLAGS_64` | 0x22b / 0x2ab / 0x32b | CONNECT_FLAGS/43 | CONNECT_FLAGS/43 | CONNECT_FLAGS/43 |
| 132 | `kercall::CONNECT_DUPFD_64` | 0x22c / 0x2ac / 0x32c | CONNECT_DUPFD/44 | CONNECT_DUPFD/44 | CONNECT_DUPFD/44 |
| 133 | `kercall::THREAD_CREATE_64` | 0x22e / 0x2ae / 0x32e | THREAD_CREATE/46 | THREAD_CREATE/46 | THREAD_CREATE/46 |
| 134 | `kercall::THREAD_DESTROY_64` | 0x22f / 0x2af / 0x32f | THREAD_DESTROY/47 | THREAD_DESTROY/47 | THREAD_DESTROY/47 |
| 135 | `kercall::THREAD_DETACH_64` | 0x231 / 0x2b1 / 0x331 | THREAD_DETACH/49 | THREAD_DETACH/49 | THREAD_DETACH/49 |
| 136 | `kercall::THREAD_JOIN_64` | 0x232 / 0x2b2 / 0x332 | THREAD_JOIN/50 | THREAD_JOIN/50 | THREAD_JOIN/50 |
| 137 | `kercall::THREAD_CANCEL_64` | 0x233 / 0x2b3 / 0x333 | THREAD_CANCEL/51 | THREAD_CANCEL/51 | THREAD_CANCEL/51 |
| 138 | `kercall::THREAD_CTL_64` | 0x234 / 0x2b4 / 0x334 | THREAD_CTL/52 | THREAD_CTL/52 | THREAD_CTL/52 |
| 139 | `kercall::THREAD_CTLEXT_64` | 0x235 / 0x2b5 / 0x335 | THREAD_CTLEXT/53 | THREAD_CTLEXT/53 | THREAD_CTLEXT/53 |
| 140 | `kercall::INTERRUPT_ATTACH_64` | 0x237 / 0x2b7 / 0x337 | INTERRUPT_ATTACH/55 | INTERRUPT_ATTACH/55 | INTERRUPT_ATTACH/55 |
| 141 | `kercall::INTERRUPT_QUERY_64` | 0x238 / 0x2b8 / 0x338 | INTERRUPT_QUERY/56 | INTERRUPT_QUERY/56 | INTERRUPT_QUERY/56 |
| 142 | `kercall::INTERRUPT_DETACH_64` | 0x239 / 0x2b9 / 0x339 | INTERRUPT_DETACH/57 | INTERRUPT_DETACH/57 | INTERRUPT_DETACH/57 |
| 143 | `kercall::INTERRUPT_WAIT_64` | 0x23a / 0x2ba / 0x33a | INTERRUPT_WAIT/58 | INTERRUPT_WAIT/58 | INTERRUPT_WAIT/58 |
| 144 | `kercall::INTERRUPT_MASK_64` | 0x23b / 0x2bb / 0x33b | INTERRUPT_MASK/59 | INTERRUPT_MASK/59 | INTERRUPT_MASK/59 |
| 145 | `kercall::INTERRUPT_UNMASK_64` | 0x23c / 0x2bc / 0x33c | INTERRUPT_UNMASK/60 | INTERRUPT_UNMASK/60 | INTERRUPT_UNMASK/60 |
| 146 | `kercall::INTERRUPT_UNBLOCK_64` | 0x23d / 0x2bd / 0x33d | INTERRUPT_UNBLOCK/61 | INTERRUPT_UNBLOCK/61 | INTERRUPT_UNBLOCK/61 |
| 147 | `kercall::INTERRUPT_CTL_64` | 0x23e / 0x2be / 0x33e | INTERRUPT_CTL/62 | INTERRUPT_CTL/62 | INTERRUPT_CTL/62 |
| 148 | `kercall::CLOCK_TIME_64` | 0x241 / 0x2c1 / 0x341 | CLOCK_TIME/65 | CLOCK_TIME/65 | CLOCK_TIME/65 |
| 149 | `kercall::CLOCK_ADJUST_64` | 0x242 / 0x2c2 / 0x342 | CLOCK_ADJUST/66 | CLOCK_ADJUST/66 | CLOCK_ADJUST/66 |
| 150 | `kercall::CLOCK_PERIOD_64` | 0x243 / 0x2c3 / 0x343 | CLOCK_PERIOD/67 | CLOCK_PERIOD/67 | CLOCK_PERIOD/67 |
| 151 | `kercall::CLOCK_ID_64` | 0x244 / 0x2c4 / 0x344 | CLOCK_ID/68 | CLOCK_ID/68 | CLOCK_ID/68 |
| 152 | `kercall::TIMER_CREATE_64` | 0x246 / 0x2c6 / 0x346 | TIMER_CREATE/70 | TIMER_CREATE/70 | TIMER_CREATE/70 |
| 153 | `kercall::TIMER_DESTROY_64` | 0x247 / 0x2c7 / 0x347 | TIMER_DESTROY/71 | TIMER_DESTROY/71 | TIMER_DESTROY/71 |
| 154 | `kercall::TIMER_SETTIME_64` | 0x248 / 0x2c8 / 0x348 | TIMER_SETTIME/72 | TIMER_SETTIME/72 | TIMER_SETTIME/72 |
| 155 | `kercall::TIMER_INFO_64` | 0x249 / 0x2c9 / 0x349 | TIMER_INFO/73 | TIMER_INFO/73 | TIMER_INFO/73 |
| 156 | `kercall::TIMER_ALARM_64` | 0x24a / 0x2ca / 0x34a | TIMER_ALARM/74 | TIMER_ALARM/74 | TIMER_ALARM/74 |
| 157 | `kercall::TIMER_TIMEOUT_64` | 0x24b / 0x2cb / 0x34b | TIMER_TIMEOUT/75 | TIMER_TIMEOUT/75 | TIMER_TIMEOUT/75 |
| 158 | `kercall::TIMER_DELEGATE_64` | 0x24c / 0x2cc / 0x34c | TIMER_DELEGATE/76 | TIMER_DELEGATE/76 | TIMER_DELEGATE/76 |
| 159 | `kercall::SYNC_RWLOCK_64` | 0x24d / 0x2cd / 0x34d | SYNC_RWLOCK/77 | SYNC_RWLOCK/77 | SYNC_RWLOCK/77 |
| 160 | `kercall::SYNC_CREATE_64` | 0x24e / 0x2ce / 0x34e | SYNC_CREATE/78 | SYNC_CREATE/78 | SYNC_CREATE/78 |
| 161 | `kercall::SYNC_DESTROY_64` | 0x24f / 0x2cf / 0x34f | SYNC_DESTROY/79 | SYNC_DESTROY/79 | SYNC_DESTROY/79 |
| 162 | `kercall::SYNC_MUTEX_LOCK_64` | 0x250 / 0x2d0 / 0x350 | SYNC_MUTEX_LOCK/80 | SYNC_MUTEX_LOCK/80 | SYNC_MUTEX_LOCK/80 |
| 163 | `kercall::SYNC_MUTEX_UNLOCK_64` | 0x251 / 0x2d1 / 0x351 | SYNC_MUTEX_UNLOCK/81 | SYNC_MUTEX_UNLOCK/81 | SYNC_MUTEX_UNLOCK/81 |
| 164 | `kercall::SYNC_CONDVAR_WAIT_64` | 0x252 / 0x2d2 / 0x352 | SYNC_CONDVAR_WAIT/82 | SYNC_CONDVAR_WAIT/82 | SYNC_CONDVAR_WAIT/82 |
| 165 | `kercall::SYNC_CONDVAR_SIGNAL_64` | 0x253 / 0x2d3 / 0x353 | SYNC_CONDVAR_SIGNAL/83 | SYNC_CONDVAR_SIG/83 | SYNC_CONDVAR_SIGNAL/83 |
| 166 | `kercall::SYNC_SEM_POST_64` | 0x254 / 0x2d4 / 0x354 | SYNC_SEM_POST/84 | SYNC_SEM_POST/84 | SYNC_SEM_POST/84 |
| 167 | `kercall::SYNC_SEM_WAIT_64` | 0x255 / 0x2d5 / 0x355 | SYNC_SEM_WAIT/85 | SYNC_SEM_WAIT/85 | SYNC_SEM_WAIT/85 |
| 168 | `kercall::SYNC_CTL_64` | 0x256 / 0x2d6 / 0x356 | SYNC_CTL/86 | SYNC_CTL/86 | SYNC_CTL/86 |
| 169 | `kercall::SYNC_BARRIER_WAIT_64` | 0x257 / 0x2d7 / 0x357 | SYNC_BARRIER_WAIT/87 | SYNC_BARRIER_WAIT/87 | SYNC_BARRIER_WAIT/87 |
| 170 | `kercall::SCHED_GET_64` | 0x258 / 0x2d8 / 0x358 | SCHED_GET/88 | SCHED_GET/88 | SCHED_GET/88 |
| 171 | `kercall::SCHED_SET_64` | 0x259 / 0x2d9 / 0x359 | SCHED_SET/89 | SCHED_SET/89 | SCHED_SET/89 |
| 172 | `kercall::SCHED_YIELD_64` | 0x25a / 0x2da / 0x35a | SCHED_YIELD/90 | SCHED_YIELD/90 | SCHED_YIELD/90 |
| 173 | `kercall::SCHED_INFO_64` | 0x25b / 0x2db / 0x35b | SCHED_INFO/91 | SCHED_INFO/91 | SCHED_INFO/91 |
| 174 | `kercall::SCHED_CTL_64` | 0x25c / 0x2dc / 0x35c | SCHED_CTL/92 | SCHED_CTL/92 | SCHED_CTL/92 |
| 175 | `kercall::MSG_QUEUE_OPEN_64` | 0x25d / 0x2dd / 0x35d | MSG_QUEUE_OPEN/93 | MSG_QUEUE_OPEN/93 | MSG_QUEUE_OPEN/93 |
| 176 | `kercall::MSG_QUEUE_SEND_64` | 0x25e / 0x2de / 0x35e | MSG_QUEUE_SEND/94 | MSG_QUEUE_SEND/94 | MSG_QUEUE_SEND/94 |
| 177 | `kercall::MSG_QUEUE_RECEIVE_64` | 0x25f / 0x2df / 0x35f | MSG_QUEUE_RECEIVE/95 | MSG_QUEUE_RECEIVE/95 | MSG_QUEUE_RECEIVE/95 |
| 178 | `kercall::MSG_QUEUE_CLOSE_64` | 0x260 / 0x2e0 / 0x360 | MSG_QUEUE_CLOSE/96 | MSG_QUEUE_CLOSE/96 | MSG_QUEUE_CLOSE/96 |
| 179 | `kercall::MSG_QUEUE_CTL_64` | 0x261 / 0x2e1 / 0x361 | MSG_QUEUE_CTL/97 | MSG_QUEUE_CTL/97 | MSG_QUEUE_CTL/97 |
| 180 | `kercall::PIPE_OPEN_64` | 0x262 / 0x2e2 / 0x362 | PIPE_OPEN/98 | PIPE_OPEN/98 | PIPE_OPEN/98 |
| 181 | `kercall::PIPE_CTL_64` | 0x263 / 0x2e3 / 0x363 | PIPE_CTL/99 | PIPE_CTL/99 | PIPE_CTL/99 |
| 182 | `kercall::PIPE_CLOSE_64` | 0x264 / 0x2e4 / 0x364 | PIPE_CLOSE/100 | PIPE_CLOSE/100 | PIPE_CLOSE/100 |
| 183 | `kercall::SYNC_TYPE_DESTROY_64` | 0x268 / 0x2e8 / 0x368 | SYNC_TYPE_DESTROY/104 | SYNC_TYPE_DESTROY/104 | SYNC_TYPE_DESTROY/104 |
| 184 | `kercall::TEST_64` | 0x269 / 0x2e9 / 0x369 | TEST/105 | TEST/105 | TEST/105 |
| 185 | `kercall::SYS_CUSTOM_64` | 0x26a / 0x2ea / 0x36a | SYS_CUSTOM/106 | SYS_CUSTOM/106 | SYS_CUSTOM/106 |
| 186 | `kercall::BAD_64` | 0x26b / 0x2eb / 0x36b | BAD/107 | BAD/107 | BAD/107 |

### `int`: INT のイベント

クラス: `INT`。1 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `int::DELIVER` | 0x007 | INT_DELIVER |

### `thread`: THREAD のイベント(スレッドの状態と仮想スレッド)

クラス: `THREAD`。31 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `thread::DEAD` | 0x000 | THDEAD |
| 2 | `thread::RUNNING` | 0x001 | THRUNNING |
| 3 | `thread::READY` | 0x002 | THREADY |
| 4 | `thread::STOPPED` | 0x003 | THSTOPPED |
| 5 | `thread::SEND` | 0x004 | THSEND |
| 6 | `thread::RECEIVE` | 0x005 | THRECEIVE |
| 7 | `thread::REPLY` | 0x006 | THREPLY |
| 8 | `thread::MQ_SEND` | 0x007 | THMQ_SEND |
| 9 | `thread::MQ_RECEIVE` | 0x008 | THMQ_RECEIVE |
| 10 | `thread::WAITPAGE` | 0x009 | THWAITPAGE |
| 11 | `thread::SIGSUSPEND` | 0x00a | THSIGSUSPEND |
| 12 | `thread::SIGWAITINFO` | 0x00b | THSIGWAITINFO |
| 13 | `thread::NANOSLEEP` | 0x00c | THNANOSLEEP |
| 14 | `thread::MUTEX` | 0x00d | THMUTEX |
| 15 | `thread::CONDVAR` | 0x00e | THCONDVAR |
| 16 | `thread::JOIN` | 0x00f | THJOIN |
| 17 | `thread::INTR` | 0x010 | THINTR |
| 18 | `thread::SEM` | 0x011 | THSEM |
| 19 | `thread::WAITCTX` | 0x012 | THWAITCTX |
| 20 | `thread::RWLOCK_READ` | 0x013 | THRWLOCK_READ |
| 21 | `thread::RWLOCK_WRITE` | 0x014 | THRWLOCK_WRITE |
| 22 | `thread::BARRIER` | 0x015 | unknown |
| 23 | `thread::PIPE` | 0x016 | unknown |
| 24 | `thread::CREATE` | 0x018 | THCREATE |
| 25 | `thread::DESTROY` | 0x019 | THDESTROY |
| 26 | `thread::MUON_MUTEX` | 0x01a | THMUON_MUTEX |
| 27 | `thread::TRACEBUFFER` | 0x01b | THTRACEBUFFER |
| 28 | `thread::INTR_ATTACH_EV` | 0x01c | THINTR_ATTACH_EV |
| 29 | `thread::TIMER_DELEGATE` | 0x01d | THTIMER_DELEGATE |
| 30 | `thread::CREATE_VT` | 0x057 | THCREATE_VT |
| 31 | `thread::DESTROY_VT` | 0x058 | THDESTROY_VT |

### `process`: PROCESS のイベント

クラス: `PROCESS`。5 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `process::CREATE_P` | 0x040 | PROCCREATE |
| 2 | `process::DESTROY_P` | 0x080 | PROCDESTROY |
| 3 | `process::CREATE_P_NAME` | 0x0c0 | PROCCREATE_NAME |
| 4 | `process::DESTROY_P_NAME` | 0x100 | PROCDESTROY_NAME |
| 5 | `process::NAME_T` | 0x140 | PROCTHREAD_NAME |

### `system`: SYSTEM のイベント

クラス: `SYSTEM`。36 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `system::RESERVED` | 0x001 | — |
| 2 | `system::PATHMGR` | 0x002 | PATHMGR_OPEN |
| 3 | `system::APS_NAME` | 0x003 | APS_NAME |
| 4 | `system::APS_BUDGETS` | 0x004 | APS_NEW_BUDGET |
| 5 | `system::APS_BNKR` | 0x005 | APS_BANKRUPTCY |
| 6 | `system::MMAP` | 0x006 | MMAP |
| 7 | `system::MUNMAP` | 0x007 | MUNMAP |
| 8 | `system::MAPNAME` | 0x008 | MAPNAME |
| 9 | `system::ADDRESS` | 0x009 | ADDRESS |
| 10 | `system::FUNC_ENTER` | 0x00a | FUNC_ENTER |
| 11 | `system::FUNC_EXIT` | 0x00b | FUNC_EXIT |
| 12 | `system::SLOG` | 0x00c | SLOG |
| 13 | `system::DEFRAG_START` | 0x00d | DEFRAG_START |
| 14 | `system::RUNSTATE` | 0x00e | RUNSTATE |
| 15 | `system::POWER` | 0x00f | POWER |
| 16 | `system::IPI` | 0x010 | IPI |
| 17 | `system::PAGEWAIT` | 0x011 | PAGEWAIT |
| 18 | `system::TIMER` | 0x012 | TIMER |
| 19 | `system::DEFRAG_END` | 0x013 | DEFRAG_END |
| 20 | `system::PROFILE` | 0x014 | PROFILE |
| 21 | `system::MAPNAME_64` | 0x015 | MAPNAME_64 |
| 22 | `system::APS_PSTATS` | 0x016 | APS_PSTATS |
| 23 | `system::APS_OSTATS` | 0x017 | APS_OSTATS |
| 24 | `system::APS_INFO` | 0x018 | APS_INFO |
| 25 | `system::APS_JOIN` | 0x019 | APS_JOIN |
| 26 | `system::APS_THREAD` | 0x01a | APS_THREAD |
| 27 | `system::APS_PROCESS` | 0x01b | APS_PROCESS |
| 28 | `system::SCHED_CONF` | 0x01c | SCHED_CONF |
| 29 | `system::IST_ATTACH` | 0x01d | IST_ATTACH |
| 30 | `system::IST_DETACH` | 0x01e | IST_DETACH |
| 31 | `system::PCTRACE` | 0x01f | PCTRACE |
| 32 | `system::ADDRESS_64` | 0x209 | ADDRESS |
| 33 | `system::FUNC_ENTER_64` | 0x20a | FUNC_ENTER |
| 34 | `system::FUNC_EXIT_64` | 0x20b | FUNC_EXIT |
| 35 | `system::IPI_64` | 0x210 | IPI_64 |
| 36 | `system::PROFILE_64` | 0x214 | PROFILE_64 |

### `comm`: COMM のイベント

クラス: `COMM`。12 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `comm::SMSG` | 0x000 | SND_MESSAGE |
| 2 | `comm::SPULSE` | 0x001 | SND_PULSE |
| 3 | `comm::RMSG` | 0x002 | REC_MESSAGE |
| 4 | `comm::RPULSE` | 0x003 | REC_PULSE |
| 5 | `comm::SPULSE_EXE` | 0x004 | SND_PULSE_EXE |
| 6 | `comm::SPULSE_DIS` | 0x005 | SND_PULSE_DIS |
| 7 | `comm::SPULSE_DEA` | 0x006 | SND_PULSE_DEA |
| 8 | `comm::SPULSE_UN` | 0x007 | SND_PULSE_UN |
| 9 | `comm::SPULSE_QUN` | 0x008 | SND_PULSE_QUN |
| 10 | `comm::SIGNAL` | 0x009 | SIGNAL |
| 11 | `comm::REPLY` | 0x00a | REPLY_MESSAGE |
| 12 | `comm::ERROR` | 0x00b | MSG_ERROR |

### `sec`: SEC のイベント

クラス: `SEC`。8 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `sec::ABLE` | 0x000 | ABLE |
| 2 | `sec::ABLE_LOOKUP` | 0x001 | ABLE_LOOKUP |
| 3 | `sec::PATH_ATTACH` | 0x002 | PATH_ATTACH |
| 4 | `sec::QNET_CONNECT` | 0x003 | unknown(3) |
| 5 | `sec::PERM_LOOKUP` | 0x004 | PERM_LOOKUP |
| 6 | `sec::PERM_TEST` | 0x005 | PERM_TEST |
| 7 | `sec::UNREG_EVENT` | 0x006 | UNREG_EVENT |
| 8 | `sec::OPEN_EVENT` | 0x007 | unknown(7) |

### `qvm`: QVM のイベント

クラス: `QVM`。8 個。

| event_id | 定数 | QNX 8.0: 内部イベント番号 | QNX 8.0: 表示名 |
|---|---|---|---|
| 1 | `qvm::GUEST_ENTER` | 0x000 | GUEST_ENTER |
| 2 | `qvm::GUEST_EXIT` | 0x001 | GUEST_EXIT |
| 3 | `qvm::CREATE_VCPU_THREAD` | 0x002 | CREATE_VCPU_THREAD |
| 4 | `qvm::RAISE_INTR` | 0x003 | INTR_RAISE |
| 5 | `qvm::LOWER_INTR` | 0x004 | INTR_LOWER |
| 6 | `qvm::TIMER_CREATE` | 0x005 | TIMER_CREATE |
| 7 | `qvm::TIMER_FIRE` | 0x006 | TIMER_FIRE |
| 8 | `qvm::CYCLES` | 0x007 | CYCLES |

イベントの定数 291 個、クラスとの組み合わせ 663 個(USER・QUIP の任意番号を除く)。
<!-- 生成部分ここまで -->
