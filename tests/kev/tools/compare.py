"""traceprinter と tracehub の出力をバイト単位で比べる。

    python tests/kev/tools/compare.py [--qnx 800] [--traceprinter PATH] [--kev-dir DIR ...] [--work DIR]

比べるもの:
  1. 合成データ: 生成器(cargo run -p kevtool -- --qnx <版> scenarios)の CSV と tests/kev/qnx<版>/data/*.csv を
     tests/kev/tools/kevedit/kevcsv.py で kev にしたもの
  2. --kev-dir の *.kev(実機で取った kev など。リポジトリには入れない)

--qnx           版(tests/kev/qnx<版>/ の <版>。例: 800)。省略時は最新の版
--traceprinter  その版の traceprinter。省略時は環境変数 TRACEHUB_TRACEPRINTER_<版>、無ければ TRACEHUB_TRACEPRINTER
--kev-dir       省略時は環境変数 TRACEHUB_KEV_DIR_<版>、無ければ TRACEHUB_KEV_DIR(; 区切りで複数)。無ければ合成データだけ
--work          作業ディレクトリ(省略時は target/compare/qnx<版>)

traceprinter の出力(-o のファイル)と `tracehub traceprinter` の出力を比べ、違えば最初の違う行を出して終了コード 1。
結果は <work>/result.json にも書く(tests/kev/tools/event_catalog.py が使う)。
traceprinter はまれに 0xC0000005 で落ちることがあるので、そのときは 3 回まで実行し直す。
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TOOLS = Path(__file__).resolve().parent
ACCESS_VIOLATION = 0xC0000005


def versions() -> list[str]:
    """試験のある版(tests/kev/qnx<版>/ のディレクトリ。例: "800")"""
    return sorted((p.name[3:] for p in (ROOT / "tests" / "kev").glob("qnx*") if p.is_dir()), key=int)


def env(name: str, qnx: str) -> str | None:
    """版ごとの環境変数(例: TRACEHUB_TRACEPRINTER_800)、無ければ版の付かないもの"""
    return os.environ.get(f"{name}_{qnx}") or os.environ.get(name)


def run(cmd: list, **kw) -> subprocess.CompletedProcess:
    return subprocess.run([str(c) for c in cmd], capture_output=True, **kw)


def traceprinter(tp: Path, kev: Path, out: Path) -> str | None:
    """traceprinter -n を実行する。失敗したら理由を返す"""
    for _ in range(3):
        r = run([tp, "-n", "-f", kev, "-o", out])
        if r.returncode & 0xFFFFFFFF == ACCESS_VIOLATION:
            continue
        if r.returncode != 0:
            return f"traceprinter が失敗しました(終了コード {r.returncode})"
        return None
    return "traceprinter が 3 回とも 0xC0000005 で落ちました"


def first_diff(a: bytes, b: bytes) -> str:
    la, lb = a.split(b"\n"), b.split(b"\n")
    for i, (x, y) in enumerate(zip(la, lb)):
        if x != y:
            return f"{i + 1} 行目\n  traceprinter: {x!r}\n  tracehub    : {y!r}"
    return f"行数が違います(traceprinter {len(la)} 行、tracehub {len(lb)} 行)"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--qnx", choices=versions(), default=versions()[-1])
    ap.add_argument("--traceprinter", type=Path)
    ap.add_argument("--kev-dir", type=Path, action="append")
    ap.add_argument("--work", type=Path)
    a = ap.parse_args()
    tp = a.traceprinter or env("TRACEHUB_TRACEPRINTER", a.qnx)
    if not tp:
        print(f"QNX {a.qnx} の traceprinter の場所を --traceprinter か TRACEHUB_TRACEPRINTER_{a.qnx} で指定してください", file=sys.stderr)
        return 2
    tp = Path(tp)
    kev_dirs = a.kev_dir or [Path(p) for p in (env("TRACEHUB_KEV_DIR", a.qnx) or "").split(";") if p]

    r = run(["cargo", "build", "--release", "-q", "-p", "tracehub"], cwd=ROOT)
    if r.returncode != 0:
        print(r.stderr.decode(errors="replace"), file=sys.stderr)
        return 1
    exe = ROOT / "target" / "release" / ("tracehub.exe" if os.name == "nt" else "tracehub")

    work = a.work or ROOT / "target" / "compare" / f"qnx{a.qnx}"
    if work.exists():
        shutil.rmtree(work)
    csv_dir, kev_dir, out_dir = work / "csv", work / "kev", work / "out"
    for d in (csv_dir, kev_dir, out_dir):
        d.mkdir(parents=True)
    r = run(["cargo", "run", "--release", "-q", "-p", "kevtool", "--", "--qnx", a.qnx, "scenarios", csv_dir], cwd=ROOT)
    if r.returncode != 0:
        print(r.stderr.decode(errors="replace"), file=sys.stderr)
        return 1
    for f in (ROOT / "tests" / "kev" / f"qnx{a.qnx}" / "data").glob("*.csv"):
        shutil.copy(f, csv_dir / f.name)
    kevs = []
    for f in sorted(csv_dir.glob("*.csv")):
        k = kev_dir / (f.stem + ".kev")
        r = run([sys.executable, TOOLS / "kevedit" / "kevcsv.py", "csv2kev", f, "-o", k])
        if not k.exists():
            print(f"{f.name}: kev にできません\n{r.stderr.decode(errors='replace')}", file=sys.stderr)
            return 1
        kevs.append(("合成", k))
    for d in kev_dirs:
        kevs += [("実機", k) for k in sorted(d.glob("*.kev"))]

    fail = 0
    results = []
    for kind, k in kevs:
        tp_out, th_out = out_dir / (k.stem + ".tp.txt"), out_dir / (k.stem + ".th.txt")
        err = traceprinter(tp, k, tp_out)
        if err:
            print(f"SKIP {kind} {k.name}: {err}")
            results.append({"kind": kind, "kev": str(k), "result": "skip"})
            continue
        r = run([exe, "traceprinter", k, th_out])
        if r.returncode != 0:
            print(f"NG   {kind} {k.name}: tracehub が失敗しました: {r.stderr.decode(errors='replace').strip()}")
            results.append({"kind": kind, "kev": str(k), "result": "ng"})
            fail += 1
            continue
        a_, b_ = tp_out.read_bytes(), th_out.read_bytes()
        if a_ == b_:
            print(f"ok   {kind} {k.name}({len(a_)} バイト)")
        else:
            print(f"NG   {kind} {k.name}: {first_diff(a_, b_)}")
            fail += 1
        results.append({"kind": kind, "kev": str(k), "result": "ok" if a_ == b_ else "ng"})
    (work / "result.json").write_text(json.dumps(results, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"QNX {a.qnx}: " + (f"{len(kevs)} 件中 {fail} 件が違います" if fail else f"{len(kevs)} 件すべて一致"))
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
