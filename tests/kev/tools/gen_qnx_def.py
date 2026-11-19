"""QNX SDP のヘッダと TableOfEvents から、版ごとの def.rs(内部 ID の定義)を生成する。

    python tests/kev/tools/gen_qnx_def.py --sdp <SDP の include ディレクトリ> --table <TableOfEvents.txt> [--out <def.rs>] [--check]

--sdp     sys/trace.h・sys/kercalls.h・sys/states.h・sys/nto_version.h を含むディレクトリ
--out     省略時は crates/tracehub-kev/src/qnx<版>/def.rs(版は _NTO_VERSION から決める。800 → qnx800)
--check   書き込まずに、置いてあるファイルが生成結果と同じかを確かめる(違えば終了コード 1)

def.rs に書くもの:
  - header のビット配置(フラグを含む)、構造の値
  - 内部クラス ID、CONTROL のイベント番号と payloads の位置、cookie
  - KER_CALL のイベント番号の作り(コール番号のマスク、exit / int / 64 ビットのビット)
  - クラスごとの (内部イベント番号, 名前, セマンティック ID) の表(KER_CALL・SYSTEM は 64 ビット版の表も)
セマンティック ID は crates/tracehub-kev/src/semantic_id.rs の定数を名前で参照する。
semantic_id.rs に無い名前があると def.rs がビルドできないので、semantic_id.rs に追加する(番号は追加だけ)。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HEADERS = ["sys/trace.h", "sys/kercalls.h", "sys/states.h", "sys/nto_version.h"]


class Headers:
    """#define と enum の値を集めて評価する(関数形式のマクロは本文だけ保持する)"""

    def __init__(self, texts: list[str]) -> None:
        self.raw: dict[str, str] = {}
        self.func: dict[str, str] = {}
        self.order: list[str] = []
        for text in texts:
            text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
            text = re.sub(r"//[^\n]*", " ", text)
            text = text.replace("\\\n", " ")
            for m in re.finditer(r"^[ \t]*#[ \t]*define[ \t]+(\w+)(\([^)]*\))?[ \t]*(.*)$", text, re.M):
                name, args, body = m.group(1), m.group(2), m.group(3).strip()
                if args is not None:
                    self.func[name] = body
                elif body:
                    self.raw[name] = body
                    self.order.append(name)
            for m in re.finditer(r"\benum\s*\w*\s*\{(.*?)\}", text, re.S):
                cur = -1
                for item in m.group(1).split(","):
                    item = item.strip()
                    if not item or not re.match(r"^[A-Za-z_]", item):
                        continue
                    if "=" in item:
                        name, expr = (x.strip() for x in item.split("=", 1))
                        cur = self.eval(expr)
                    else:
                        name = item
                        cur += 1
                    self.raw[name] = str(cur)
                    self.order.append(name)
        self.cache: dict[str, int] = {}

    def eval(self, expr: str, depth: int = 0) -> int:
        if depth > 20:
            raise ValueError(f"再帰が深すぎます: {expr}")
        e = re.sub(r"\(\s*_U?[Ii]nt\d+t\s*\)", "", expr)
        e = re.sub(r"\(\s*unsigned\s*\)", "", e)
        e = re.sub(r"\b(0[xX][0-9a-fA-F]+|\d+)[uUlL]+\b", r"\1", e)

        def sub(m: re.Match) -> str:
            if m.group(0) in self.raw:
                return f"({self.value(m.group(0), depth + 1)})"
            raise KeyError(m.group(0))

        return int(eval(re.sub(r"\b[A-Za-z_]\w*\b", sub, e), {"__builtins__": {}}))

    def value(self, name: str, depth: int = 0) -> int:
        if name not in self.cache:
            self.cache[name] = self.eval(self.raw[name], depth)
        return self.cache[name]

    def has(self, name: str) -> bool:
        try:
            self.value(name)
            return True
        except (KeyError, ValueError, SyntaxError, NameError, TypeError):
            return False

    def is_alias(self, name: str) -> bool:
        """本文が別の定義を参照している(別名や合成値。..._LAST、..._IPI_64 など)"""
        body = re.sub(r"\(\s*_U?[Ii]nt\d+t\s*\)", "", self.raw.get(name, ""))
        return re.search(r"\b[A-Za-z_]\w*\b", re.sub(r"\b(0[xX][0-9a-fA-F]+|\d+)[uUlL]*\b", "", body)) is not None

    def prefixed(self, prefix: str, exclude=(), literal_only: bool = False) -> list[tuple[str, int]]:
        """接頭辞の付いた定義を、ファイル中の順に (接頭辞を除いた名前, 値) で返す"""
        out, seen = [], set()
        for n in self.order:
            if n.startswith(prefix) and n not in seen:
                seen.add(n)
                short = n[len(prefix):]
                if any(re.fullmatch(x, short) for x in exclude):
                    continue
                if literal_only and self.is_alias(n):
                    continue
                if self.has(n):
                    out.append((short, self.value(n)))
        return out


def mask_shift(body: str) -> tuple[int, int]:
    """((h)&0x3f000000u)>>24 や ((c)&(0x1fu<<10)) から (マスク, シフト) を取り出す"""
    nums = [int(x, 0) for x in re.findall(r"0[xX][0-9a-fA-F]+|\b\d+\b", re.sub(r"\b(0[xX][0-9a-fA-F]+|\d+)[uU]\b", r"\1", body))]
    if ">>" in body:
        return nums[0] >> nums[1], nums[1]
    if "<<" in body:
        return nums[0], nums[1]
    m = nums[0]
    s = (m & -m).bit_length() - 1
    return m >> s, s


def table_args(text: str, event: str) -> list[str]:
    """TableOfEvents の、あるイベントの Fast: の引数名の列"""
    for block in re.split(r"\n\s*\n", text):
        if re.search(rf"^Event:\s*{re.escape(event)}\s*$", block, re.M):
            m = re.search(r"^Fast:\s*(.*)$", block, re.M)
            if m:
                return [a.strip() for a in m.group(1).split(",")]
    raise KeyError(f"TableOfEvents に {event} がありません")


def word_of(args: list[str], keyword: str) -> int:
    for i, a in enumerate(args):
        if keyword in a:
            return i
    raise KeyError(f"{args} に {keyword} がありません")


def rows(name: str, items: list[tuple[int, str, str]], doc: str) -> str:
    """(内部イベント番号, 名前, セマンティック ID の式) の表"""
    out = [f"/// {doc}", f"pub const {name}: &[(u16, &str, EventId)] = &["]
    out += [f'    ({code}, "{n}", {sem}),' for code, n, sem in items]
    out.append("];")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sdp", required=True, type=Path)
    ap.add_argument("--table", required=True, type=Path)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()

    H = Headers([(a.sdp / h).read_text(encoding="utf-8", errors="replace") for h in HEADERS])
    table = a.table.read_text(encoding="utf-8", errors="replace")
    ver = H.value("_NTO_VERSION")
    module = f"qnx{ver}"
    out_path = a.out or ROOT / "crates" / "tracehub-kev" / "src" / module / "def.rs"

    ev_mask, ev_shift = mask_shift(H.func["_NTO_TRACE_GETEVENT"])
    cl_mask, cl_shift = mask_shift(H.func["_NTO_TRACE_GETEVENT_C"])
    cpu_mask, cpu_shift = mask_shift(H.func["_NTO_TRACE_GETCPU"])
    st_mask, st_shift = mask_shift(H.func["_TRACE_GET_STRUCT"])
    structs = [(n, H.value(f"_TRACE_STRUCT_{n}") >> st_shift) for n in ("S", "CB", "CC", "CE")]
    # フラグ: _NTO_TRACE_PROCESS_INJECTED のビットから CPU の手前まで
    fl_shift = H.value("_NTO_TRACE_PROCESS_INJECTED").bit_length() - 1
    fl_mask = (1 << (cpu_shift - fl_shift)) - 1
    classes = [(n[: -len("_C")], v >> cl_shift) for n, v in H.prefixed("_TRACE_")
               if n.endswith("_C") and n not in ("EMPTY_C", "CONTAINER_C")]
    control = [(n, v) for n, v in H.prefixed("_TRACE_CONTROL_") if n != "C"]
    max_kercall = H.value("_TRACE_MAX_KER_CALL_NUM")
    kercalls = H.prefixed("__KER_")
    states = H.prefixed("STATE_", exclude=[r".*MAX"])
    pr_th = {n: v for n, v in H.prefixed("_TRACE_PR_TH_")}
    proc_names = ["CREATE_P", "DESTROY_P", "CREATE_P_NAME", "DESTROY_P_NAME", "NAME_T"]
    vt_names = ["CREATE_VT", "DESTROY_VT"]
    system = H.prefixed("_NTO_TRACE_SYS_", literal_only=True)
    # 64 ビット版がある SYSTEM のイベント(TableOfEvents の `|_NTO_TRACE_KERCALL64` と、ヘッダの *_64 = X|_NTO_TRACE_KERCALL64)
    bit64 = H.value("_NTO_TRACE_KERCALL64")
    # KER_CALL は全コールに 64 ビット版を置く(TableOfEvents に記載の無い 64 ビット版もカーネルは記録し、traceprinter は表示する)
    sys_nums = dict(system)
    sys64_names = set(re.findall(r"^Event:\s*_NTO_TRACE_SYS_(\w+)\|_NTO_TRACE_KERCALL64\s*$", table, re.M))
    for n, v in H.prefixed("_NTO_TRACE_SYS_"):
        if n.endswith("_64") and n[:-3] in sys_nums and v == sys_nums[n[:-3]] | bit64:
            sys64_names.add(n[:-3])
    system64 = [(n, v | bit64) for n, v in system if n in sys64_names]
    comm = H.prefixed("_NTO_TRACE_COMM_", literal_only=True)
    sec = [(n, v) for n, v in H.prefixed("_NTO_TRACE_SEC_", exclude=[r".*FIRST", r".*LAST"])
           if not n.startswith("ABLE_") or n == "ABLE_LOOKUP"]
    qvm = H.prefixed("_NTO_TRACE_QVM_", exclude=[r".*FIRST", r".*LAST"])
    ints = [(n, v) for n, v in H.prefixed("_TRACE_INT_") if n != "C"]

    t_time = table_args(table, "_NTO_TRACE_CONTROLTIME")
    t_buf = table_args(table, "_NTO_TRACE_CONTROLBUFFER")
    t_drop = table_args(table, "_NTO_TRACE_CONTROLDROPPED")

    def cls(name: str) -> int:
        return dict(classes)[name]

    out = [
        f"//! QNX {ver // 100}.{ver % 100 // 10} の内部 ID の定義。",
        "//!",
        "//! 生成ファイル(tests/kev/tools/gen_qnx_def.py)。手で編集しない。",
        "//! 入力: SDP のヘッダ(" + "、".join(HEADERS) + ")と TableOfEvents。",
        "",
        "use crate::semantic_id::{self, EventId};",
        "",
        "// ---- header のビット配置(_NTO_TRACE_GETEVENT・_NTO_TRACE_GETEVENT_C・_NTO_TRACE_GETCPU・_TRACE_GET_STRUCT)",
        f"pub const EVENT_SHIFT: u32 = {ev_shift};",
        f"pub const EVENT_MASK: u32 = {ev_mask:#x};",
        f"pub const CLASS_SHIFT: u32 = {cl_shift};",
        f"pub const CLASS_MASK: u32 = {cl_mask:#x};",
        f"pub const CPU_SHIFT: u32 = {cpu_shift};",
        f"pub const CPU_MASK: u32 = {cpu_mask:#x};",
        f"pub const STRUCT_SHIFT: u32 = {st_shift};",
        f"pub const STRUCT_MASK: u32 = {st_mask:#x};",
        "/// フラグ(_NTO_TRACE_PROCESS_INJECTED のビットから CPU の手前まで)",
        f"pub const FLAGS_SHIFT: u32 = {fl_shift};",
        f"pub const FLAGS_MASK: u32 = {fl_mask:#x};",
        f"pub const FLAG_PROCESS_INJECTED: u8 = {H.value('_NTO_TRACE_PROCESS_INJECTED') >> fl_shift:#x};",
        "",
        "// ---- 構造(_TRACE_STRUCT_*。シフト後の値)",
    ]
    out += [f"pub const STRUCT_{n}: u32 = {v};" for n, v in structs]
    out += ["", "// ---- 内部クラス ID(_TRACE_*_C。シフト後の値)"]
    out += [f"pub const CLASS_{n}: u8 = {v};" for n, v in classes]
    out += ["", "// ---- CONTROL のイベント番号(_TRACE_CONTROL_*)"]
    out += [f"pub const CONTROL_{n}: u16 = {v};" for n, v in control]
    out += [
        "",
        "// ---- CONTROL の payloads の位置(TableOfEvents)",
        f"pub const TIME_MSB_WORD: usize = {word_of(t_time, 'msb')};",
        f"pub const TIME_LSB_WORD: usize = {word_of(t_time, 'lsb')};",
        f"pub const BUFFER_SEQ_WORD: usize = {word_of(t_buf, 'sequence')};",
        f"pub const BUFFER_COUNT_WORD: usize = {word_of(t_buf, 'num events')};",
        f"pub const DROPPED_COUNT_WORD: usize = {word_of(t_drop, 'dropped')};",
        "/// BUFFER_END の payloads(_TRACECOOKIE)",
        f"pub const BUFFER_COOKIE: u32 = {H.value('_TRACECOOKIE'):#x};",
        "",
        "// ---- KER_CALL のイベント番号 = コール番号 | 種別 | 64 ビット版(_TRACE_MAX_KER_CALL_NUM・_NTO_TRACE_KERCALL64)",
        f"pub const KERCALL_NUM_MASK: u16 = {H.value('_NTO_TRACE_KERCALLMASK'):#x};",
        f"pub const KERCALL_EXIT: u16 = {max_kercall:#x};",
        f"pub const KERCALL_INT: u16 = {max_kercall << 1:#x};",
        "/// 64 ビット版のビット(KER_CALL と SYSTEM で共通)",
        f"pub const BIT64: u16 = {H.value('_NTO_TRACE_KERCALL64'):#x};",
        "",
        "// ---- PR_TH のイベント番号: これより小さければスレッドの状態(_TRACE_PR_TH_CREATE_P)",
        f"pub const PR_TH_PROCESS_BASE: u16 = {pr_th['CREATE_P']};",
        "",
        rows("KERCALLS", [(v, n, f"semantic_id::kercall::{n}") for n, v in kercalls], "カーネルコール(__KER_*): (コール番号, 名前, セマンティック ID)"),
        "",
        rows("KERCALLS_64", [(v, n, f"semantic_id::kercall::{n}_64") for n, v in kercalls],
             "カーネルコールの 64 ビット版(全コール): (コール番号, 名前, セマンティック ID)"),
        "",
        rows("THREAD_STATES", [(v, n, f"semantic_id::thread::{n}") for n, v in states], "スレッドの状態(STATE_*)。PR_TH のイベント番号"),
        "",
        rows("PR_TH_VTHREAD", [(pr_th[n], n, f"semantic_id::thread::{n}") for n in vt_names], "仮想スレッドの生成・破棄(_TRACE_PR_TH_*_VT)"),
        "",
        rows("PR_TH_PROCESS", [(pr_th[n], n, f"semantic_id::process::{n}") for n in proc_names], "プロセスのイベント(_TRACE_PR_TH_*)"),
        "",
        rows("CONTROL_EVENTS", [(v, n, f"semantic_id::control::{n}") for n, v in control], "CONTROL のイベント"),
        "",
        rows("INT_EVENTS", [(v, n, f"semantic_id::int::{n}") for n, v in ints], "INT のイベント(_TRACE_INT_*)"),
        "",
        rows("SYSTEM_EVENTS", [(v, n, f"semantic_id::system::{n}") for n, v in system], "SYSTEM のイベント(_NTO_TRACE_SYS_*)"),
        "",
        rows("SYSTEM_EVENTS_64", [(v, n, f"semantic_id::system::{n}_64") for n, v in system64],
             "SYSTEM の 64 ビット版(_NTO_TRACE_SYS_*_64、TableOfEvents の |_NTO_TRACE_KERCALL64): (BIT64 を足した番号, 名前, セマンティック ID)"),
        "",
        rows("COMM_EVENTS", [(v, n, f"semantic_id::comm::{n}") for n, v in comm], "COMM のイベント(_NTO_TRACE_COMM_*)"),
        "",
        rows("SEC_EVENTS", [(v, n, f"semantic_id::sec::{n}") for n, v in sec], "SEC のイベント(_NTO_TRACE_SEC_*)"),
        "",
        rows("QVM_EVENTS", [(v, n, f"semantic_id::qvm::{n}") for n, v in qvm], "QVM のイベント(_NTO_TRACE_QVM_*)"),
        "",
    ]
    text = "\n".join(out)
    if a.check:
        if not out_path.exists() or out_path.read_text(encoding="utf-8") != text:
            print(f"{out_path} が生成結果と違います", file=sys.stderr)
            return 1
        return 0
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(text, encoding="utf-8", newline="\n")
    print(out_path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
