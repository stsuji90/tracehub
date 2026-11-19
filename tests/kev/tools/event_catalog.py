"""確認用のイベント一覧を作る(セマンティック ID ごとの定義・表示・TableOfEvents・試験の結果)。

    python tests/kev/tools/event_catalog.py [--table TableOfEvents.txt] [--kev-dir DIR ...] [--out DIR]

--table    QNX の TableOfEvents.txt。省略時は環境変数 TRACEHUB_TABLE_OF_EVENTS(無ければ TableOfEvents の列は空)
--kev-dir  実機の kev の置き場所(*.kev)。省略時は環境変数 TRACEHUB_KEV_DIR(; 区切り)
--out      出力先(省略時は target/event_catalog)

出力(TableOfEvents の文を含むので、リポジトリに入れない):
  events.csv   セマンティック ID ごとに 1 行(Excel で開ける UTF-8)
  summary.md   件数のまとめ、TableOfEvents との違い、試験の結果

traceprinter との比較の結果は、先に tests/kev/tools/compare.py を動かしておくと target/compare/result.json から読む。
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

# TableOfEvents のクラス → クラス ID の定数名
TOE_CLASS = {
    "_NTO_TRACE_KERCALLENTER": "KER_CALL_ENTER",
    "_NTO_TRACE_KERCALLEXIT": "KER_CALL_EXIT",
    "_NTO_TRACE_KERCALLINT": "KER_CALL_INT",
    "_NTO_TRACE_CONTROL": "CONTROL",
    "_NTO_TRACE_INT": "INT",
    "_NTO_TRACE_THREAD": "THREAD",
    "_NTO_TRACE_VTHREAD": "THREAD",
    "_NTO_TRACE_PROCESS": "PROCESS",
    "_NTO_TRACE_SYSTEM": "SYSTEM",
    "_NTO_TRACE_COMM": "COMM",
    "_NTO_TRACE_SEC": "SEC",
    "_NTO_TRACE_QVM": "QVM",
}
# イベント名の前置き(長いものから)
TOE_PREFIX = ["__KER_", "_NTO_TRACE_SYS_", "_NTO_TRACE_COMM_", "_NTO_TRACE_SEC_", "_NTO_TRACE_QVM_",
              "_NTO_TRACE_CONTROL", "_NTO_TRACE_VTH", "_NTO_TRACE_TH", "_NTO_TRACE_PROC", "_NTO_TRACE_"]
# 前置きを除いた名前が def.rs の名前と違うもの
TOE_ALIAS = {
    ("PROCESS", "CREATE"): "CREATE_P",
    ("PROCESS", "DESTROY"): "DESTROY_P",
    ("PROCESS", "CREATE_NAME"): "CREATE_P_NAME",
    ("PROCESS", "DESTROY_NAME"): "DESTROY_P_NAME",
    ("PROCESS", "THREAD_NAME"): "NAME_T",
    ("THREAD", "CREATE_VT"): "CREATE_VT",
    # TableOfEvents の誤記
    ("KER_CALL_ENTER", "TYPE_SYNC_DESTROY"): "SYNC_TYPE_DESTROY",
    ("KER_CALL_EXIT", "TYPE_SYNC_DESTROY"): "SYNC_TYPE_DESTROY",
}
BIT64 = "|_NTO_TRACE_KERCALL64"


def norm(name: str) -> str:
    return name.replace("_", "").upper()


def parse_table(text: str) -> list[dict]:
    """TableOfEvents のブロック(Class・Event・Fast・Wide・Call)"""
    blocks, cur, key = [], None, None
    for line in text.splitlines():
        m = re.match(r"^(Class|Event|Fast|Wide|Call|#Args):\s*(.*)$", line)
        if m:
            key = m.group(1)
            if key == "Class":
                cur = {"Class": m.group(2).strip()}
                blocks.append(cur)
            elif cur is not None:
                cur[key] = m.group(2).strip()
        elif line.strip() and cur is not None and key in ("Fast", "Wide", "Call"):
            cur[key] += " " + line.strip()
        elif not line.strip():
            key = None
    return [b for b in blocks if "Event" in b]


def toe_key(block: dict, events_by_class: dict[str, dict[str, str]]) -> tuple[str, str] | None:
    """ブロック → (クラスの定数名, イベントの定数名)。対応が取れなければ None"""
    cls = TOE_CLASS.get(block["Class"])
    if cls is None:
        return None
    ev = block["Event"].replace(" ", "")
    is64 = ev.endswith(BIT64)
    ev = ev.removesuffix(BIT64)
    for p in TOE_PREFIX:
        if ev.startswith(p):
            ev = ev[len(p):]
            break
    if ev.endswith("_64"):
        is64, ev = True, ev[:-3]
    ev = TOE_ALIAS.get((cls, ev), ev)
    if block["Class"] == "_NTO_TRACE_VTHREAD" and not ev.endswith("_VT"):
        ev += "_VT"
    want = norm(ev) + ("64" if is64 else "")
    return (cls, events_by_class.get(cls, {}).get(want)) if want in events_by_class.get(cls, {}) else None


def toe_labels(s: str) -> list[str]:
    """TableOfEvents の引数名の並び(型や語数の注記を除く)"""
    if not s or s.upper().startswith("N/A"):
        return []
    out = []
    for p in s.split(","):
        p = re.sub(r"\((?:\d+|string|64|32)\)", "", p).strip()
        if p and p.lower() not in ("empty", "dummy"):
            out.append(p)
    return out


def wide_text(b: dict) -> str:
    """Wide の引数名(same as above なら Fast)"""
    w = b.get("Wide", "")
    return b.get("Fast", "") if w.lower().startswith("same as above") else w


def label_key(s: str) -> str:
    return re.sub(r"[\s_]", "", s).lower()


def diff_labels(toe: list[str], ours: list[str]) -> str:
    if not toe and not ours:
        return ""
    a, b = {label_key(x) for x in toe}, {label_key(x) for x in ours}
    if a == b:
        return "一致"
    only_t = [x for x in toe if label_key(x) not in b]
    only_o = [x for x in ours if label_key(x) not in a]
    parts = []
    if only_t:
        parts.append("TableOfEvents だけ: " + ", ".join(only_t))
    if only_o:
        parts.append("tracehub だけ: " + ", ".join(only_o))
    return " / ".join(parts)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--table", type=Path, default=os.environ.get("TRACEHUB_TABLE_OF_EVENTS"))
    ap.add_argument("--kev-dir", type=Path, action="append")
    ap.add_argument("--out", type=Path, default=ROOT / "target" / "event_catalog")
    a = ap.parse_args()
    kev_dirs = a.kev_dir or [Path(p) for p in os.environ.get("TRACEHUB_KEV_DIR", "").split(";") if p]
    real_kevs = [k for d in kev_dirs for k in sorted(d.glob("*.kev"))]

    r = subprocess.run(["cargo", "run", "--release", "-q", "-p", "tracehub-kev", "--example", "test_support", "--", "catalog",
                        *map(str, real_kevs)], cwd=ROOT, capture_output=True)
    if r.returncode != 0:
        print(r.stderr.decode(errors="replace"), file=sys.stderr)
        return 1
    if r.stderr:
        print(r.stderr.decode(errors="replace"), file=sys.stderr)
    rows = list(csv.DictReader(r.stdout.decode("utf-8").splitlines(), delimiter="\t"))

    # TableOfEvents
    events_by_class: dict[str, dict[str, str]] = defaultdict(dict)
    for row in rows:
        events_by_class[row["class"]][norm(row["event"].split("::", 1)[1])] = row["event"]
    toe: dict[tuple[str, str], dict] = {}
    unmapped = []
    if a.table:
        for b in parse_table(a.table.read_text(encoding="utf-8", errors="replace")):
            k = toe_key(b, events_by_class)
            if k is None:
                unmapped.append(b)
            else:
                toe[k] = b

    # traceprinter との比較の結果
    result_path = ROOT / "target" / "compare" / "result.json"
    results = json.loads(result_path.read_text(encoding="utf-8")) if result_path.exists() else []
    by_kind = defaultdict(Counter)
    for x in results:
        by_kind[x["kind"]][x["result"]] += 1
    synthetic_ok = bool(results) and by_kind["合成"]["ok"] > 0 and by_kind["合成"]["ng"] == 0 and by_kind["合成"]["skip"] == 0
    real_ok = by_kind["実機"]["ok"] > 0 and by_kind["実機"]["ng"] == 0 and by_kind["実機"]["skip"] == 0

    out_rows = []
    for row in rows:
        k = (row["class"], row["event"])
        b = toe.get(k, {})
        syn_n, syn_d = int(row["synthetic"]), int(row["synthetic_displayed"])
        real_n, real_d = int(row["real"]), int(row["real_displayed"])
        out_rows.append({
            "クラス": row["class"],
            "イベント": row["event"],
            "class_id": row["class_id"],
            "event_id": row["event_id"],
            "内部 ID": f"{row['int_class']}/{row['int_event']}",
            "表示名": f"{row['display_class']}:{row['display_name']}",
            "fast の書式": row["fast"],
            "wide の書式": row["wide"],
            "出力の例(fast)": row["example_fast"],
            "出力の例(wide)": row["example_wide"],
            "TableOfEvents": "あり" if b else ("" if not a.table else "なし"),
            "TableOfEvents の Fast": b.get("Fast", ""),
            "TableOfEvents の Wide": wide_text(b),
            "引数名(fast)": diff_labels(toe_labels(b.get("Fast", "")), [x for x in row["fast_labels"].split(",") if x]) if b else "",
            "引数名(wide)": diff_labels(toe_labels(wide_text(b)), [x for x in row["wide_labels"].split(",") if x]) if b else "",
            "合成 kev の件数": syn_n,
            "合成 kev で表示": syn_d,
            "合成 kev の比較": ("一致" if synthetic_ok else "未確認・不一致あり") if syn_n else "",
            "実機 kev の件数": real_n,
            "実機 kev で表示": real_d,
            "実機 kev の比較": ("一致" if real_ok else "未確認・不一致あり") if real_n else "",
        })

    a.out.mkdir(parents=True, exist_ok=True)
    with open(a.out / "events.csv", "w", encoding="utf-8-sig", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(out_rows[0]))
        w.writeheader()
        w.writerows(out_rows)

    # まとめ
    per = defaultdict(Counter)
    for o in out_rows:
        c = per[o["クラス"]]
        c["ID"] += 1
        c["表示"] += o["合成 kev で表示"] > 0
        c["合成"] += o["合成 kev の件数"] > 0
        c["TableOfEvents"] += o["TableOfEvents"] == "あり"
        c["引数名の違い"] += any(x not in ("", "一致") for x in (o["引数名(fast)"], o["引数名(wide)"]))
        c["実機"] += o["実機 kev の件数"] > 0
    cols = ["ID", "表示", "合成", "TableOfEvents", "引数名の違い", "実機"]
    lines = ["# イベント一覧のまとめ", "",
             f"- セマンティック ID: {len(out_rows)} 種類(USER・QUIP の任意番号を除く)",
             f"- traceprinter が行を出すもの: {sum(o['合成 kev で表示'] > 0 for o in out_rows)} 種類",
             f"- 合成 kev と traceprinter の比較: " + (f"{dict(by_kind['合成'])}" if results else "未実行(compare.py を先に動かす)"),
             f"- 実機 kev: {len(real_kevs)} 件。比較: " + (f"{dict(by_kind['実機'])}" if by_kind["実機"] else "未実行"),
             f"- TableOfEvents: " + (f"{a.table.name}(ブロック {len(toe) + len(unmapped)}、対応の取れないもの {len(unmapped)})" if a.table else "指定なし"),
             "", "| クラス | " + " | ".join(cols) + " |", "|---|" + "---|" * len(cols)]
    for cls, c in per.items():
        lines.append(f"| {cls} | " + " | ".join(str(c[x]) for x in cols) + " |")
    total = Counter()
    for c in per.values():
        total.update(c)
    lines.append("| 計 | " + " | ".join(str(total[x]) for x in cols) + " |")
    lines += ["", "列: 表示 = 合成 kev で traceprinter が行を出したもの、合成 = 合成 kev に含まれるもの、"
              "引数名の違い = TableOfEvents の引数名と tracehub のラベルが違うもの、実機 = 実機 kev に現れたもの。"]
    shown_no_toe = [o for o in out_rows if o["合成 kev で表示"] > 0 and o["TableOfEvents"] == "なし"]
    if a.table:
        lines += ["", f"## 表示するが TableOfEvents に無いもの({len(shown_no_toe)})", ""]
        lines += [f"- {o['クラス']} {o['イベント']}({o['表示名']})" for o in shown_no_toe]
        lines += ["", f"## TableOfEvents にあるが対応の取れないもの({len(unmapped)})", ""]
        lines += [f"- {b['Class']} {b['Event']}" for b in unmapped]
    (a.out / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(a.out / "events.csv")
    print(a.out / "summary.md")
    return 0


if __name__ == "__main__":
    sys.exit(main())
