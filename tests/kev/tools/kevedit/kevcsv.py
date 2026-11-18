"""kev(QNX のカーネルトレース)と CSV を相互に変換する。kevedit.html と同じ CSV の書式(CSV_FORMAT.md)。

    python kevcsv.py kev2csv  IN.kev [-o OUT.csv]   kev → CSV(UTF-8、BOM 付き、CRLF)
    python kevcsv.py csv2kev  IN.csv [-o OUT.kev]   CSV → kev
    python kevcsv.py check    IN.kev | IN.csv        ヘッダとトレースバッファの点検(書き換えない)

-o を省くと、入力の拡張子を変えた名前で書く。注意は標準エラーに出し、誤りがあれば終了コード 1。
標準ライブラリだけで動く(Python 3.8 以降)。
"""

from __future__ import annotations

import argparse
import json
import re
import struct
import sys
from pathlib import Path

# 定義(QNX 8.0 の sys/trace.h などから取った値)。kevedit.html の <script id="defs"> と同じ内容にしておく
DEFS = json.loads(r'''
{
 "release": "8.0",
 "slot_bytes": 16,
 "layout": {"event": {"shift": 0, "mask": 1023}, "class": {"shift": 10, "mask": 31}, "flags": {"shift": 16, "mask": 255}, "cpu": {"shift": 24, "mask": 63}, "struct": {"shift": 30, "mask": 3}},
 "structs": [[0, "S"], [1, "CB"], [2, "CC"], [3, "CE"]],
 "header": {"prefix": "TRACE_", "postfix": "::", "begin": "TRACE_HEADER_BEGIN", "end": "TRACE_HEADER_END", "little_endian": "TRACE_LITTLE_ENDIAN", "big_endian": "TRACE_BIG_ENDIAN", "syspage_len": "TRACE_SYSPAGE_LEN", "ver_major": "TRACE_VER_MAJOR", "ver_minor": "TRACE_VER_MINOR"},
 "new_header": [["TRACE_HEADER_BEGIN", ""], ["TRACE_VER_MAJOR", "2"], ["TRACE_VER_MINOR", "01"], ["TRACE_LITTLE_ENDIAN", "TRUE"], ["TRACE_ENCODING", "16 byte events"], ["TRACE_CYCLES_PER_SEC", "19200000"], ["TRACE_CPU_NUM", "1"], ["TRACE_SYS_RELEASE", "8.0.0"], ["TRACE_SYSPAGE_LEN", "0"], ["TRACE_HEADER_END", ""]],
 "control": {"class": 1, "time": 1, "buffer": 2, "buffer_end": 4, "words": {"time_msb": 0, "time_lsb": 1, "buffer_sequence": 0, "buffer_num_events": 1, "dropped_count": 0}},
 "cookie": 3873892070,
 "wrap_threshold": 2147483648,
 "classes": [
  {"value": 1, "name": "CONTROL", "bit64": 0, "events": [
   [1, "TIME"], [2, "BUFFER"], [4, "BUFFER_END"], [8, "DROPPED"]
  ]},
  {"value": 2, "name": "KER_CALL", "bit64": 512, "events": [
   [0, "NOP:enter"], [128, "NOP:exit"], [256, "NOP:int"], [1, "TRACE_EVENT:enter"], [129, "TRACE_EVENT:exit"], [257, "TRACE_EVENT:int"],
   [2, "RING0:enter"], [130, "RING0:exit"], [258, "RING0:int"], [3, "CACHE_CONTROL:enter"], [131, "CACHE_CONTROL:exit"], [259, "CACHE_CONTROL:int"],
   [4, "SYS_SRANDOM:enter"], [132, "SYS_SRANDOM:exit"], [260, "SYS_SRANDOM:int"], [5, "MSG_REGISTER_EVENT:enter"], [133, "MSG_REGISTER_EVENT:exit"], [261, "MSG_REGISTER_EVENT:int"],
   [6, "MSG_SEND_PULSEPTR:enter"], [134, "MSG_SEND_PULSEPTR:exit"], [262, "MSG_SEND_PULSEPTR:int"], [7, "SYS_CPUPAGE_GET:enter"], [135, "SYS_CPUPAGE_GET:exit"], [263, "SYS_CPUPAGE_GET:int"],
   [8, "SYS_CPUPAGE_SET:enter"], [136, "SYS_CPUPAGE_SET:exit"], [264, "SYS_CPUPAGE_SET:int"], [9, "MSG_PAUSE:enter"], [137, "MSG_PAUSE:exit"], [265, "MSG_PAUSE:int"],
   [10, "MSG_CURRENT:enter"], [138, "MSG_CURRENT:exit"], [266, "MSG_CURRENT:int"], [11, "MSG_SENDV:enter"], [139, "MSG_SENDV:exit"], [267, "MSG_SENDV:int"],
   [12, "MSG_SENDVNC:enter"], [140, "MSG_SENDVNC:exit"], [268, "MSG_SENDVNC:int"], [13, "MSG_ERROR:enter"], [141, "MSG_ERROR:exit"], [269, "MSG_ERROR:int"],
   [14, "MSG_RECEIVEV:enter"], [142, "MSG_RECEIVEV:exit"], [270, "MSG_RECEIVEV:int"], [15, "MSG_REPLYV:enter"], [143, "MSG_REPLYV:exit"], [271, "MSG_REPLYV:int"],
   [16, "MSG_READV:enter"], [144, "MSG_READV:exit"], [272, "MSG_READV:int"], [17, "MSG_WRITEV:enter"], [145, "MSG_WRITEV:exit"], [273, "MSG_WRITEV:int"],
   [19, "MSG_INFO:enter"], [147, "MSG_INFO:exit"], [275, "MSG_INFO:int"], [20, "MSG_SEND_PULSE:enter"], [148, "MSG_SEND_PULSE:exit"], [276, "MSG_SEND_PULSE:int"],
   [21, "MSG_DELIVER_EVENT:enter"], [149, "MSG_DELIVER_EVENT:exit"], [277, "MSG_DELIVER_EVENT:int"], [24, "MSG_RECEIVEPULSEV:enter"], [152, "MSG_RECEIVEPULSEV:exit"], [280, "MSG_RECEIVEPULSEV:int"],
   [25, "MSG_VERIFY_EVENT:enter"], [153, "MSG_VERIFY_EVENT:exit"], [281, "MSG_VERIFY_EVENT:int"], [26, "SIGNAL_KILL:enter"], [154, "SIGNAL_KILL:exit"], [282, "SIGNAL_KILL:int"],
   [27, "SIGNAL_RETURN:enter"], [155, "SIGNAL_RETURN:exit"], [283, "SIGNAL_RETURN:int"], [29, "SIGNAL_ACTION:enter"], [157, "SIGNAL_ACTION:exit"], [285, "SIGNAL_ACTION:int"],
   [30, "SIGNAL_PROCMASK:enter"], [158, "SIGNAL_PROCMASK:exit"], [286, "SIGNAL_PROCMASK:int"], [31, "SIGNAL_SUSPEND:enter"], [159, "SIGNAL_SUSPEND:exit"], [287, "SIGNAL_SUSPEND:int"],
   [32, "SIGNAL_WAITINFO:enter"], [160, "SIGNAL_WAITINFO:exit"], [288, "SIGNAL_WAITINFO:int"], [33, "SIGNAL_KILL_SIGVAL:enter"], [161, "SIGNAL_KILL_SIGVAL:exit"], [289, "SIGNAL_KILL_SIGVAL:int"],
   [35, "CHANNEL_CREATE:enter"], [163, "CHANNEL_CREATE:exit"], [291, "CHANNEL_CREATE:int"], [36, "CHANNEL_DESTROY:enter"], [164, "CHANNEL_DESTROY:exit"], [292, "CHANNEL_DESTROY:int"],
   [37, "CHANNEL_CTL:enter"], [165, "CHANNEL_CTL:exit"], [293, "CHANNEL_CTL:int"], [39, "CONNECT_ATTACH:enter"], [167, "CONNECT_ATTACH:exit"], [295, "CONNECT_ATTACH:int"],
   [40, "CONNECT_DETACH:enter"], [168, "CONNECT_DETACH:exit"], [296, "CONNECT_DETACH:int"], [41, "CONNECT_SERVER_INFO:enter"], [169, "CONNECT_SERVER_INFO:exit"], [297, "CONNECT_SERVER_INFO:int"],
   [42, "CONNECT_CLIENT_INFO:enter"], [170, "CONNECT_CLIENT_INFO:exit"], [298, "CONNECT_CLIENT_INFO:int"], [43, "CONNECT_FLAGS:enter"], [171, "CONNECT_FLAGS:exit"], [299, "CONNECT_FLAGS:int"],
   [44, "CONNECT_DUPFD:enter"], [172, "CONNECT_DUPFD:exit"], [300, "CONNECT_DUPFD:int"], [46, "THREAD_CREATE:enter"], [174, "THREAD_CREATE:exit"], [302, "THREAD_CREATE:int"],
   [47, "THREAD_DESTROY:enter"], [175, "THREAD_DESTROY:exit"], [303, "THREAD_DESTROY:int"], [49, "THREAD_DETACH:enter"], [177, "THREAD_DETACH:exit"], [305, "THREAD_DETACH:int"],
   [50, "THREAD_JOIN:enter"], [178, "THREAD_JOIN:exit"], [306, "THREAD_JOIN:int"], [51, "THREAD_CANCEL:enter"], [179, "THREAD_CANCEL:exit"], [307, "THREAD_CANCEL:int"],
   [52, "THREAD_CTL:enter"], [180, "THREAD_CTL:exit"], [308, "THREAD_CTL:int"], [53, "THREAD_CTLEXT:enter"], [181, "THREAD_CTLEXT:exit"], [309, "THREAD_CTLEXT:int"],
   [55, "INTERRUPT_ATTACH:enter"], [183, "INTERRUPT_ATTACH:exit"], [311, "INTERRUPT_ATTACH:int"], [56, "INTERRUPT_QUERY:enter"], [184, "INTERRUPT_QUERY:exit"], [312, "INTERRUPT_QUERY:int"],
   [57, "INTERRUPT_DETACH:enter"], [185, "INTERRUPT_DETACH:exit"], [313, "INTERRUPT_DETACH:int"], [58, "INTERRUPT_WAIT:enter"], [186, "INTERRUPT_WAIT:exit"], [314, "INTERRUPT_WAIT:int"],
   [59, "INTERRUPT_MASK:enter"], [187, "INTERRUPT_MASK:exit"], [315, "INTERRUPT_MASK:int"], [60, "INTERRUPT_UNMASK:enter"], [188, "INTERRUPT_UNMASK:exit"], [316, "INTERRUPT_UNMASK:int"],
   [61, "INTERRUPT_UNBLOCK:enter"], [189, "INTERRUPT_UNBLOCK:exit"], [317, "INTERRUPT_UNBLOCK:int"], [62, "INTERRUPT_CTL:enter"], [190, "INTERRUPT_CTL:exit"], [318, "INTERRUPT_CTL:int"],
   [65, "CLOCK_TIME:enter"], [193, "CLOCK_TIME:exit"], [321, "CLOCK_TIME:int"], [66, "CLOCK_ADJUST:enter"], [194, "CLOCK_ADJUST:exit"], [322, "CLOCK_ADJUST:int"],
   [67, "CLOCK_PERIOD:enter"], [195, "CLOCK_PERIOD:exit"], [323, "CLOCK_PERIOD:int"], [68, "CLOCK_ID:enter"], [196, "CLOCK_ID:exit"], [324, "CLOCK_ID:int"],
   [70, "TIMER_CREATE:enter"], [198, "TIMER_CREATE:exit"], [326, "TIMER_CREATE:int"], [71, "TIMER_DESTROY:enter"], [199, "TIMER_DESTROY:exit"], [327, "TIMER_DESTROY:int"],
   [72, "TIMER_SETTIME:enter"], [200, "TIMER_SETTIME:exit"], [328, "TIMER_SETTIME:int"], [73, "TIMER_INFO:enter"], [201, "TIMER_INFO:exit"], [329, "TIMER_INFO:int"],
   [74, "TIMER_ALARM:enter"], [202, "TIMER_ALARM:exit"], [330, "TIMER_ALARM:int"], [75, "TIMER_TIMEOUT:enter"], [203, "TIMER_TIMEOUT:exit"], [331, "TIMER_TIMEOUT:int"],
   [76, "TIMER_DELEGATE:enter"], [204, "TIMER_DELEGATE:exit"], [332, "TIMER_DELEGATE:int"], [77, "SYNC_RWLOCK:enter"], [205, "SYNC_RWLOCK:exit"], [333, "SYNC_RWLOCK:int"],
   [78, "SYNC_CREATE:enter"], [206, "SYNC_CREATE:exit"], [334, "SYNC_CREATE:int"], [79, "SYNC_DESTROY:enter"], [207, "SYNC_DESTROY:exit"], [335, "SYNC_DESTROY:int"],
   [80, "SYNC_MUTEX_LOCK:enter"], [208, "SYNC_MUTEX_LOCK:exit"], [336, "SYNC_MUTEX_LOCK:int"], [81, "SYNC_MUTEX_UNLOCK:enter"], [209, "SYNC_MUTEX_UNLOCK:exit"], [337, "SYNC_MUTEX_UNLOCK:int"],
   [82, "SYNC_CONDVAR_WAIT:enter"], [210, "SYNC_CONDVAR_WAIT:exit"], [338, "SYNC_CONDVAR_WAIT:int"], [83, "SYNC_CONDVAR_SIGNAL:enter"], [211, "SYNC_CONDVAR_SIGNAL:exit"], [339, "SYNC_CONDVAR_SIGNAL:int"],
   [84, "SYNC_SEM_POST:enter"], [212, "SYNC_SEM_POST:exit"], [340, "SYNC_SEM_POST:int"], [85, "SYNC_SEM_WAIT:enter"], [213, "SYNC_SEM_WAIT:exit"], [341, "SYNC_SEM_WAIT:int"],
   [86, "SYNC_CTL:enter"], [214, "SYNC_CTL:exit"], [342, "SYNC_CTL:int"], [87, "SYNC_BARRIER_WAIT:enter"], [215, "SYNC_BARRIER_WAIT:exit"], [343, "SYNC_BARRIER_WAIT:int"],
   [88, "SCHED_GET:enter"], [216, "SCHED_GET:exit"], [344, "SCHED_GET:int"], [89, "SCHED_SET:enter"], [217, "SCHED_SET:exit"], [345, "SCHED_SET:int"],
   [90, "SCHED_YIELD:enter"], [218, "SCHED_YIELD:exit"], [346, "SCHED_YIELD:int"], [91, "SCHED_INFO:enter"], [219, "SCHED_INFO:exit"], [347, "SCHED_INFO:int"],
   [92, "SCHED_CTL:enter"], [220, "SCHED_CTL:exit"], [348, "SCHED_CTL:int"], [93, "MSG_QUEUE_OPEN:enter"], [221, "MSG_QUEUE_OPEN:exit"], [349, "MSG_QUEUE_OPEN:int"],
   [94, "MSG_QUEUE_SEND:enter"], [222, "MSG_QUEUE_SEND:exit"], [350, "MSG_QUEUE_SEND:int"], [95, "MSG_QUEUE_RECEIVE:enter"], [223, "MSG_QUEUE_RECEIVE:exit"], [351, "MSG_QUEUE_RECEIVE:int"],
   [96, "MSG_QUEUE_CLOSE:enter"], [224, "MSG_QUEUE_CLOSE:exit"], [352, "MSG_QUEUE_CLOSE:int"], [97, "MSG_QUEUE_CTL:enter"], [225, "MSG_QUEUE_CTL:exit"], [353, "MSG_QUEUE_CTL:int"],
   [98, "PIPE_OPEN:enter"], [226, "PIPE_OPEN:exit"], [354, "PIPE_OPEN:int"], [99, "PIPE_CTL:enter"], [227, "PIPE_CTL:exit"], [355, "PIPE_CTL:int"],
   [100, "PIPE_CLOSE:enter"], [228, "PIPE_CLOSE:exit"], [356, "PIPE_CLOSE:int"], [104, "SYNC_TYPE_DESTROY:enter"], [232, "SYNC_TYPE_DESTROY:exit"], [360, "SYNC_TYPE_DESTROY:int"],
   [105, "TEST:enter"], [233, "TEST:exit"], [361, "TEST:int"], [106, "SYS_CUSTOM:enter"], [234, "SYS_CUSTOM:exit"], [362, "SYS_CUSTOM:int"],
   [107, "BAD:enter"], [235, "BAD:exit"], [363, "BAD:int"]
  ]},
  {"value": 3, "name": "INT", "bit64": 0, "events": [
   [7, "DELIVER"]
  ]},
  {"value": 4, "name": "PR_TH", "bit64": 0, "events": [
   [0, "THDEAD"], [1, "THRUNNING"], [2, "THREADY"], [3, "THSTOPPED"], [4, "THSEND"], [5, "THRECEIVE"],
   [6, "THREPLY"], [7, "THMQ_SEND"], [8, "THMQ_RECEIVE"], [9, "THWAITPAGE"], [10, "THSIGSUSPEND"], [11, "THSIGWAITINFO"],
   [12, "THNANOSLEEP"], [13, "THMUTEX"], [14, "THCONDVAR"], [15, "THJOIN"], [16, "THINTR"], [17, "THSEM"],
   [18, "THWAITCTX"], [19, "THRWLOCK_READ"], [20, "THRWLOCK_WRITE"], [21, "THBARRIER"], [22, "THPIPE"], [24, "THCREATE"],
   [25, "THDESTROY"], [26, "THMUON_MUTEX"], [27, "THTRACEBUFFER"], [28, "THINTR_ATTACH_EV"], [29, "THTIMER_DELEGATE"], [64, "CREATE_P"],
   [87, "CREATE_VT"], [88, "DESTROY_VT"], [128, "DESTROY_P"], [192, "CREATE_P_NAME"], [256, "DESTROY_P_NAME"], [320, "NAME_T"]
  ]},
  {"value": 5, "name": "SYSTEM", "bit64": 512, "events": [
   [1, "RESERVED"], [2, "PATHMGR"], [3, "APS_NAME"], [4, "APS_BUDGETS"], [5, "APS_BNKR"], [6, "MMAP"],
   [7, "MUNMAP"], [8, "MAPNAME"], [9, "ADDRESS"], [10, "FUNC_ENTER"], [11, "FUNC_EXIT"], [12, "SLOG"],
   [13, "DEFRAG_START"], [14, "RUNSTATE"], [15, "POWER"], [16, "IPI"], [17, "PAGEWAIT"], [18, "TIMER"],
   [19, "DEFRAG_END"], [20, "PROFILE"], [21, "MAPNAME_64"], [22, "APS_PSTATS"], [23, "APS_OSTATS"], [24, "APS_INFO"],
   [25, "APS_JOIN"], [26, "APS_THREAD"], [27, "APS_PROCESS"], [28, "SCHED_CONF"], [29, "IST_ATTACH"], [30, "IST_DETACH"],
   [31, "PCTRACE"]
  ]},
  {"value": 6, "name": "USER", "bit64": 0, "events": [
  ]},
  {"value": 7, "name": "COMM", "bit64": 0, "events": [
   [0, "SMSG"], [1, "SPULSE"], [2, "RMSG"], [3, "RPULSE"], [4, "SPULSE_EXE"], [5, "SPULSE_DIS"],
   [6, "SPULSE_DEA"], [7, "SPULSE_UN"], [8, "SPULSE_QUN"], [9, "SIGNAL"], [10, "REPLY"], [11, "ERROR"]
  ]},
  {"value": 8, "name": "QUIP", "bit64": 0, "events": [
  ]},
  {"value": 9, "name": "SEC", "bit64": 0, "events": [
   [0, "ABLE"], [1, "ABLE_LOOKUP"], [2, "PATH_ATTACH"], [3, "QNET_CONNECT"], [4, "PERM_LOOKUP"], [5, "PERM_TEST"],
   [6, "UNREG_EVENT"], [7, "OPEN_EVENT"]
  ]},
  {"value": 10, "name": "QVM", "bit64": 0, "events": [
   [0, "GUEST_ENTER"], [1, "GUEST_EXIT"], [2, "CREATE_VCPU_THREAD"], [3, "RAISE_INTR"], [4, "LOWER_INTR"], [5, "TIMER_CREATE"],
   [6, "TIMER_FIRE"], [7, "CYCLES"]
  ]}
 ]
}
''')

# CSV の書式(kevedit.html の CONF.csv と同じ)
CSV_ON, CSV_OFF = "on", "off"
SECTIONS = {"header": "[header]", "syspage": "[syspage]", "slots": "[slots]"}
HEADER_COLS = ["key", "value"]
SYSPAGE_COLS = ["offset", "bytes"]
SYSPAGE_ROW_BYTES = 16
SLOT_COLS = ["header", "stamp", "payload0", "payload1"]
SLOT_INFO_COLS = [
    "(struct)", "(cpu)", "(flags)", "(class_name)", "(class)", "(event_name)", "(event)",
    "(payload0_str)", "(payload0_dec)", "(payload1_str)", "(payload1_dec)",
]
BUFFER_SLOTS = 1011  # 1 トレースバッファのスロット数の上限(_TRACELEMENTS。QNX 8.0 の値)

L = DEFS["layout"]
H = DEFS["header"]
SB = DEFS["slot_bytes"]
CT = DEFS["control"]
CW = CT["words"]
STRUCT_NAMES = {v: n for v, n in DEFS["structs"]}
CLASSES = {c["value"]: c for c in DEFS["classes"]}
for _c in CLASSES.values():
    _c["by_val"] = {v: n for v, n in _c["events"]}


# ---------------------------------------------------------------- ヘッダ語・数値・文字列

def get_f(h: int, f: str) -> int:
    return (h >> L[f]["shift"]) & L[f]["mask"]


def class_name(cv: int) -> str | None:
    c = CLASSES.get(cv)
    return c["name"] if c else None


def event_name(cv: int, ev: int) -> str | None:
    c = CLASSES.get(cv)
    if not c:
        return None
    n = c["by_val"].get(ev)
    if n:
        return n
    if c["bit64"] and ev & c["bit64"]:
        n = c["by_val"].get(ev & ~c["bit64"])
        if n:
            return n + ":64"
    return None


def parse_u32(s: str) -> int:
    """10 進、0x 付き 16 進、負の 10 進(2 の補数)。'_' は無視"""
    t = s.strip().replace("_", "")
    if re.fullmatch(r"0[xX][0-9a-fA-F]+", t):
        v = int(t[2:], 16)
    elif re.fullmatch(r"-?\d+", t):
        v = int(t)
    else:
        raise ValueError(f'数値ではありません: "{t}"')
    if v < -0x80000000 or v > 0xFFFFFFFF:
        raise ValueError(f"32 ビットに収まりません: {t}")
    return v & 0xFFFFFFFF


def hex8(v: int) -> str:
    return f"0x{v & 0xFFFFFFFF:08x}"


def hex2(v: int) -> str:
    return f"0x{v:02x}"


def word_to_str(d: int, le: bool) -> str:
    """1 語のバイト列(ファイルのバイト順)。末尾の NUL は省き、表示できないバイトは \\xHH、\\ は \\\\"""
    b = struct.pack("<I" if le else ">I", d)
    b = b.rstrip(b"\0")
    out = []
    for c in b:
        if c == 0x5C:
            out.append("\\\\")
        elif 0x20 <= c < 0x7F:
            out.append(chr(c))
        else:
            out.append(f"\\x{c:02x}")
    return "".join(out)


# ---------------------------------------------------------------- 文書
# doc = {"header": [{"on", "key", "value"}], "syspage": [{"on", "bytes"}], "slots": [{"on", "h", "stamp", "d1", "d2"}]}
# ヘッダの key は区切りまで含む(例: "TRACE_VER_MAJOR::")。kev のヘッダは有効な行の key + value をつないだ文字列

KEY_RE = re.compile(re.escape(H["prefix"]) + r"[A-Z0-9_]+" + re.escape(H["postfix"]))


def header_text(header: list[dict]) -> str:
    return "".join(x["key"] + x["value"] for x in header if x["on"])


def split_header(text: str) -> tuple[list[dict], int]:
    """ヘッダの文字列を項目に分ける(HEADER_END まで)。end = HEADER_END の直後の位置(無ければ -1)"""
    keys = []
    end = -1
    for m in KEY_RE.finditer(text):
        key = m.group(0)[: -len(H["postfix"])]
        keys.append((key, m.start(), m.end()))
        if key == H["end"]:
            end = m.end()
            break
    items = []
    for i, (key, start, vstart) in enumerate(keys):
        vend = keys[i + 1][1] if i + 1 < len(keys) else vstart
        items.append({"key": key, "start": start, "value": text[vstart:vend]})
    return items, end


def header_value(header: list[dict], key: str) -> str | None:
    v = None
    for x in split_header(header_text(header))[0]:
        if x["key"] == key:
            v = x["value"]
    return v


def little_endian(header: list[dict]) -> bool:
    if header_value(header, H["little_endian"]) is not None:
        return True
    if header_value(header, H["big_endian"]) is not None:
        return False
    return True


def syspage_len(header: list[dict]) -> int | None:
    """None = 数値ではない"""
    v = header_value(header, H["syspage_len"])
    if v is None or v.strip() == "":
        return 0
    if not re.fullmatch(r"\s*\d+\s*", v):
        return None
    return int(v)


def parse_kev(buf: bytes, notes: list) -> dict:
    text = buf.decode("latin-1")
    items, end = split_header(text)
    if end < 0:
        raise ValueError(f"ヘッダの終わり({H['end']}{H['postfix']})が見つかりません")
    header = [{"on": True, "key": x["key"] + H["postfix"], "value": x["value"]} for x in items]
    if items[0]["start"] > 0:  # 最初のキーより前のバイトも 1 行として持つ
        header.insert(0, {"on": True, "key": "", "value": text[: items[0]["start"]]})
        notes.append(("warn", f"ヘッダの最初のキーより前に {items[0]['start']} バイトあります(1 行目)"))
    sp = syspage_len(header)
    if sp is None:
        notes.append(("warn", f"{H['syspage_len']} が数値ではないので 0 として読みます"))
        sp = 0
    if end + sp > len(buf):
        notes.append(("warn", f"syspage が {sp - (len(buf) - end)} バイト足りません"))
        sp = len(buf) - end
    syspage = [{"on": True, "bytes": buf[end + o: end + min(sp, o + SYSPAGE_ROW_BYTES)]} for o in range(0, sp, SYSPAGE_ROW_BYTES)]
    base = end + sp
    n, tail = divmod(len(buf) - base, SB)
    if tail:
        notes.append(("warn", f"末尾の {tail} バイト(スロットに満たない端数)は読み込みません"))
    fmt = ("<" if little_endian(header) else ">") + "4I"
    slots = []
    for i in range(n):
        h, st, d1, d2 = struct.unpack_from(fmt, buf, base + i * SB)
        slots.append({"on": True, "h": h, "stamp": st, "d1": d1, "d2": d2})
    return {"header": header, "syspage": syspage, "slots": slots}


def build_kev(doc: dict) -> bytes:
    hs = header_text(doc["header"])
    bad = [c for c in hs if ord(c) > 0xFF]
    if bad:
        raise ValueError(f"ヘッダに latin-1 の範囲外の文字があります: {bad[0]}")
    out = bytearray(hs.encode("latin-1"))
    for x in doc["syspage"]:
        if x["on"]:
            out += x["bytes"]
    fmt = ("<" if little_endian(doc["header"]) else ">") + "4I"
    for s in doc["slots"]:
        if s["on"]:
            out += struct.pack(fmt, s["h"], s["stamp"], s["d1"], s["d2"])
    return bytes(out)


# ---------------------------------------------------------------- CSV

def csv_cell(s) -> str:
    s = str(s)
    return '"' + s.replace('"', '""') + '"' if re.search(r'[",\r\n]|^\s|\s$', s) else s


def csv_line(cells) -> str:
    return ",".join(csv_cell(c) for c in cells)


def parse_csv(text: str) -> list[tuple[int, list[str]]]:
    """(行番号, セルの列) の列。引用符の中の改行・コンマ・"" を扱う"""
    rows, row, cell = [], [], []
    q = False
    line = row_line = 1
    i = 0
    while i < len(text):
        c = text[i]
        if q:
            if c == '"':
                if i + 1 < len(text) and text[i + 1] == '"':
                    cell.append('"')
                    i += 1
                else:
                    q = False
            else:
                if c == "\n":
                    line += 1
                cell.append(c)
        elif c == '"':
            q = True
        elif c == ",":
            row.append("".join(cell))
            cell = []
        elif c in "\r\n":
            if c == "\r" and i + 1 < len(text) and text[i + 1] == "\n":
                i += 1
            row.append("".join(cell))
            rows.append((row_line, row))
            row, cell = [], []
            line += 1
            row_line = line
        else:
            cell.append(c)
        i += 1
    if cell or row:
        row.append("".join(cell))
        rows.append((row_line, row))
    return rows


def to_csv(doc: dict) -> str:
    le = little_endian(doc["header"])
    flag = lambda x: CSV_ON if x["on"] else CSV_OFF  # noqa: E731
    out = [csv_line([SECTIONS["header"], *HEADER_COLS])]
    out += [csv_line([flag(x), x["key"], x["value"]]) for x in doc["header"]]
    out.append(csv_line([SECTIONS["syspage"], *SYSPAGE_COLS]))
    off = 0
    for x in doc["syspage"]:
        out.append(csv_line([flag(x), off, "0x" + x["bytes"].hex()]))
        off += len(x["bytes"])
    out.append(csv_line([SECTIONS["slots"], *SLOT_COLS, *SLOT_INFO_COLS]))
    for s in doc["slots"]:
        h = s["h"]
        st, cv, ev = get_f(h, "struct"), get_f(h, "class"), get_f(h, "event")
        out.append(csv_line([
            flag(s), hex8(h), hex8(s["stamp"]), hex8(s["d1"]), hex8(s["d2"]),
            STRUCT_NAMES.get(st, st), get_f(h, "cpu"), hex2(get_f(h, "flags")),
            class_name(cv) or "", cv, event_name(cv, ev) or "", ev,
            word_to_str(s["d1"], le), s["d1"], word_to_str(s["d2"], le), s["d2"],
        ]))
    return "\r\n".join(out) + "\r\n"


def from_csv(text: str) -> dict:
    if text.startswith("﻿"):
        text = text[1:]
    doc = {"header": [], "syspage": [], "slots": []}
    errs = []
    sec_by_name = {v: k for k, v in SECTIONS.items()}
    sec = None
    for line, cells in parse_csv(text):
        if all(c.strip() == "" for c in cells):
            continue
        cell = lambda i: cells[i] if i < len(cells) else ""  # noqa: E731
        kind = cells[0].strip()
        try:
            if kind.startswith("["):
                sec = sec_by_name.get(kind)
                if not sec:
                    raise ValueError(f"知らないセクションです: {kind}")
                continue
            k = kind.lower()
            if k not in (CSV_ON, CSV_OFF):
                raise ValueError(f'1 列目は {CSV_ON} / {CSV_OFF} / セクション名です: "{kind}"')
            on = k == CSV_ON
            if not sec:
                raise ValueError("セクション名の行より前にデータがあります")
            if sec == "header":
                doc["header"].append({"on": on, "key": cell(1), "value": cell(2)})  # 前後の空白も文字として扱う
            elif sec == "syspage":
                hx = re.sub(r"\s+", "", re.sub(r"^0[xX]", "", cell(2).strip()))
                if not re.fullmatch(r"(?:[0-9a-fA-F]{2})*", hx):
                    raise ValueError(f"bytes は 16 進のバイト列です: {cell(2)}")
                doc["syspage"].append({"on": on, "bytes": bytes.fromhex(hx)})
            else:
                def word(i: int) -> int:
                    if cell(i).strip() == "":
                        raise ValueError(f"{SLOT_COLS[i - 1]} が空です")
                    try:
                        return parse_u32(cell(i))
                    except ValueError as e:
                        raise ValueError(f"{SLOT_COLS[i - 1]}: {e}") from None
                doc["slots"].append({"on": on, "h": word(1), "stamp": word(2), "d1": word(3), "d2": word(4)})
        except ValueError as e:
            errs.append(f"{line} 行目: {e}")
    if errs:
        more = f"\n…ほか {len(errs) - 20} 件" if len(errs) > 20 else ""
        raise ValueError("CSV を読めませんでした\n" + "\n".join(errs[:20]) + more)
    return doc


# ---------------------------------------------------------------- 点検

def check(doc: dict) -> list[tuple[str, str]]:
    """ヘッダの注意(kevedit.html と同じ)"""
    out = []
    text = header_text(doc["header"])
    items, end = split_header(text)
    if end < 0:
        out.append(("err", f"ヘッダに {H['end']}{H['postfix']} がありません(kev として読めません)"))
    elif end < len(text):
        out.append(("warn", f"{H['end']}{H['postfix']} の後ろの {len(text) - end} 文字は syspage として読まれます"))
    if not items or items[0]["key"] != H["begin"] or items[0]["start"] != 0:
        out.append(("warn", f"ヘッダが {H['begin']}{H['postfix']} で始まっていません"))
    for k in (H["ver_major"], H["ver_minor"]):
        if header_value(doc["header"], k) is None:
            out.append(("warn", f"{k} がありません(traceprinter が読みません)"))
    sp = syspage_len(doc["header"])
    actual = sum(len(x["bytes"]) for x in doc["syspage"] if x["on"])
    if sp is None:
        out.append(("warn", f"{H['syspage_len']} が数値ではありません"))
    elif sp != actual:
        out.append(("warn", f"{H['syspage_len']} は {sp} ですが、出力する syspage は {actual} バイトです"))
    return out


def is_ctl(s: dict, ev: int) -> bool:
    return get_f(s["h"], "class") == CT["class"] and get_f(s["h"], "event") == ev


def pv(s: dict, k: int) -> int:
    return s["d1"] if k == 0 else s["d2"]


def analyze_buffers(slots: list[dict]) -> tuple[list[dict], list[str]]:
    """有効なスロットをファイル順に見て、CPU ごとのトレースバッファと気づいた点を返す(kevedit.html と同じ規則)"""
    bufs, issues = [], []
    st: dict[int, dict] = {}
    for i, s in enumerate(slots):
        if not s["on"]:
            continue
        cpu = get_f(s["h"], "cpu")
        c = st.setdefault(cpu, {"open": None, "last_seq": None, "pending_time": -1, "outside": []})
        for oc, o in st.items():
            if oc != cpu and o["open"] and not o["open"]["interleaved"]:
                o["open"]["interleaved"] = True
                o["open"]["issues"].append(f"途中に CPU {cpu} のスロットがある({i + 1} 行)")
        if is_ctl(s, CT["buffer"]):
            if c["open"]:
                c["open"]["issues"].append("BUFFER_END が無いまま次の BUFFER")
                bufs.append(c["open"])
            first, actual = i, 1
            if c["pending_time"] >= 0:  # 直前の TIME から数える
                first, actual = c["pending_time"], 2
                c["outside"].pop()
            seq = pv(s, CW["buffer_sequence"])
            b = {"cpu": cpu, "seq": seq, "declared": pv(s, CW["buffer_num_events"]), "first": first, "row": i, "last": i,
                 "actual": actual, "closed": False, "issues": [], "interleaved": False}
            if c["last_seq"] is not None and seq != (c["last_seq"] + 1) & 0xFFFFFFFF:
                b["issues"].append(f"通番が {c['last_seq']} の次に {seq}")
            c["last_seq"] = seq
            c["open"] = b
        elif c["open"]:
            c["open"]["actual"] += 1
            c["open"]["last"] = i
            if is_ctl(s, CT["buffer_end"]):
                if s["d1"] != DEFS["cookie"] or s["d2"] != DEFS["cookie"]:
                    c["open"]["issues"].append(f"BUFFER_END の cookie が {hex8(DEFS['cookie'])} ではない")
                c["open"]["closed"] = True
                bufs.append(c["open"])
                c["open"] = None
        elif is_ctl(s, CT["buffer_end"]):
            issues.append(f"CPU {cpu}: BUFFER の無い BUFFER_END({i + 1} 行)")
        else:
            c["outside"].append(i)
        c["pending_time"] = i if not c["open"] and is_ctl(s, CT["time"]) else -1
    for cpu, c in st.items():
        if c["open"]:
            c["open"]["issues"].append("BUFFER_END が無いまま終わる")
            bufs.append(c["open"])
        if c["outside"]:
            issues.append(f"CPU {cpu}: トレースバッファの外のスロットが {len(c['outside'])} 個(最初は {c['outside'][0] + 1} 行)")
    for b in bufs:
        if b["closed"] and b["declared"] != b["actual"]:
            b["issues"].append(f"スロット数が BUFFER では {b['declared']}、実際は {b['actual']}")
        if b["actual"] > BUFFER_SLOTS:
            b["issues"].append(f"上限 {BUFFER_SLOTS} スロットを超えている")
    bufs.sort(key=lambda b: b["first"])
    return bufs, issues


# ---------------------------------------------------------------- コマンド

def load(path: Path, notes: list) -> dict:
    if path.suffix.lower() == ".csv":
        return from_csv(path.read_text(encoding="utf-8-sig"))
    return parse_kev(path.read_bytes(), notes)


def main() -> int:
    ap = argparse.ArgumentParser(description="kev と CSV の相互変換(書式は CSV_FORMAT.md)")
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("kev2csv", "csv2kev"):
        p = sub.add_parser(name)
        p.add_argument("input", type=Path)
        p.add_argument("-o", "--output", type=Path)
    p = sub.add_parser("check")
    p.add_argument("input", type=Path)
    a = ap.parse_args()

    notes: list = []
    try:
        if a.cmd == "kev2csv":
            doc = parse_kev(a.input.read_bytes(), notes)
            out = a.output or a.input.with_suffix(".csv")
            out.write_bytes(("﻿" + to_csv(doc)).encode("utf-8"))  # Excel で開けるように BOM を付ける
        elif a.cmd == "csv2kev":
            doc = from_csv(a.input.read_text(encoding="utf-8-sig"))
            out = a.output or a.input.with_suffix(".kev")
            out.write_bytes(build_kev(doc))
        else:
            doc = load(a.input, notes)
    except (ValueError, OSError) as e:
        print(f"エラー: {e}", file=sys.stderr)
        return 1

    notes += check(doc)
    bufs, issues = analyze_buffers(doc["slots"])
    notes += [("warn", m) for m in issues]
    notes += [("warn", f"CPU {b['cpu']} 通番 {b['seq']}({b['first'] + 1}〜{b['last'] + 1} 行): {m}") for b in bufs for m in b["issues"]]
    for kind, m in notes:
        print(f"{'エラー' if kind == 'err' else '注意'}: {m}", file=sys.stderr)
    if a.cmd == "check":
        on = sum(1 for s in doc["slots"] if s["on"])
        print(f"スロット {len(doc['slots'])} 行(有効 {on})、トレースバッファ {len(bufs)} 個、注意 {len(notes)} 件")
    else:
        print(out)
    return 1 if any(k == "err" for k, _ in notes) else 0


if __name__ == "__main__":
    sys.exit(main())
