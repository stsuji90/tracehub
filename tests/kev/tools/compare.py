"""traceprinter と tracehub の出力をバイト単位で比べる。

    python tests/kev/tools/compare.py [--traceprinter PATH] [--kev-dir DIR ...] [--work DIR]

比べるもの:
  1. 合成データ: 生成器(cargo run --example scenarios)の CSV と tests/kev/data/*.csv を
     tests/kev/tools/kevedit/kevcsv.py で kev にしたもの
  2. --kev-dir の *.kev(実機で取った kev など。リポジトリには入れない)

--traceprinter  省略時は環境変数 TRACEHUB_TRACEPRINTER
--kev-dir       省略時は環境変数 TRACEHUB_KEV_DIR(; 区切りで複数)。無ければ合成データだけ
--work          作業ディレクトリ(省略時は target/compare)

traceprinter の出力(-o のファイル)と `tracehub traceprinter` の出力を比べ、違えば最初の違う行を出して終了コード 1。
traceprinter はまれに 0xC0000005 で落ちることがあるので、そのときは 3 回まで実行し直す。
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TOOLS = Path(__file__).resolve().parent
ACCESS_VIOLATION = 0xC0000005


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
    ap.add_argument("--traceprinter", type=Path, default=os.environ.get("TRACEHUB_TRACEPRINTER"))
    ap.add_argument("--kev-dir", type=Path, action="append")
    ap.add_argument("--work", type=Path, default=ROOT / "target" / "compare")
    a = ap.parse_args()
    if not a.traceprinter:
        print("traceprinter の場所を --traceprinter か TRACEHUB_TRACEPRINTER で指定してください", file=sys.stderr)
        return 2
    kev_dirs = a.kev_dir or [Path(p) for p in os.environ.get("TRACEHUB_KEV_DIR", "").split(";") if p]

    r = run(["cargo", "build", "--release", "-q", "-p", "tracehub"], cwd=ROOT)
    if r.returncode != 0:
        print(r.stderr.decode(errors="replace"), file=sys.stderr)
        return 1
    exe = ROOT / "target" / "release" / ("tracehub.exe" if os.name == "nt" else "tracehub")

    work = a.work
    if work.exists():
        shutil.rmtree(work)
    csv_dir, kev_dir, out_dir = work / "csv", work / "kev", work / "out"
    for d in (csv_dir, kev_dir, out_dir):
        d.mkdir(parents=True)
    r = run(["cargo", "run", "--release", "-q", "-p", "tracehub-kev", "--example", "scenarios", "--", csv_dir], cwd=ROOT)
    if r.returncode != 0:
        print(r.stderr.decode(errors="replace"), file=sys.stderr)
        return 1
    for f in (ROOT / "tests" / "kev" / "data").glob("*.csv"):
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
    for kind, k in kevs:
        tp_out, th_out = out_dir / (k.stem + ".tp.txt"), out_dir / (k.stem + ".th.txt")
        err = traceprinter(a.traceprinter, k, tp_out)
        if err:
            print(f"SKIP {kind} {k.name}: {err}")
            continue
        r = run([exe, "traceprinter", k, th_out])
        if r.returncode != 0:
            print(f"NG   {kind} {k.name}: tracehub が失敗しました: {r.stderr.decode(errors='replace').strip()}")
            fail += 1
            continue
        a_, b_ = tp_out.read_bytes(), th_out.read_bytes()
        if a_ == b_:
            print(f"ok   {kind} {k.name}({len(a_)} バイト)")
        else:
            print(f"NG   {kind} {k.name}: {first_diff(a_, b_)}")
            fail += 1
    print(f"{len(kevs)} 件中 {fail} 件が違います" if fail else f"{len(kevs)} 件すべて一致")
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
