"""traceprinter と tracehub の出力をバイト単位で比べる。

    python tests/kev/tools/compare.py [--qnx 800] [--traceprinter PATH] [--kev-dir DIR ...] [--synthetic-dir DIR] [--work DIR]

比べるもの:
  1. 合成データ: 生成器(cargo run -p kevtool -- --qnx <版> scenarios)の CSV と tests/kev/qnx<版>/data/*.csv を
     tests/kev/tools/kevedit/kevcsv.py で kev にしたもの。--synthetic-dir があれば、作らずにその *.kev を使う
  2. --kev-dir の *.kev(実機で取った kev など。リポジトリには入れない)

--qnx            版(tests/kev/qnx<版>/ の <版>。例: 800)。省略時は最新の版
--traceprinter   その版の traceprinter。省略時は環境変数 TRACEHUB_TRACEPRINTER_<版>、無ければ TRACEHUB_TRACEPRINTER。
                 無ければ、正解のファイルのある kev だけを比べる
--kev-dir        省略時は環境変数 TRACEHUB_KEV_DIR_<版>、無ければ TRACEHUB_KEV_DIR(; 区切りで複数)。無ければ合成データだけ
--synthetic-dir  合成データの kev と正解の組(traceprinter の無い環境で、ほかの環境で作った合成データを比べるとき。
                 traceprinter のある環境で compare.py を動かしたあとの <work>/kev/ をそのまま持ってくればよい)
--work           作業ディレクトリ(省略時は target/compare/qnx<版>)

正解(traceprinter の出力): kev の横に同じ名前の X.tp.txt(`traceprinter -n -f X.kev -o X.tp.txt` で取ったもの)があれば、
それを使い traceprinter を動かさない。無ければ traceprinter を動かし、<work>/kev/ の合成データの横にも X.tp.txt を書く。
正解も traceprinter も無い kev は SKIP。標準出力をリダイレクトしたものは正解にならない(末尾の集計の行・改行が違う)。

正解と `tracehub traceprinter` の出力を比べ、違えば最初の違う行を出して終了コード 1。
結果は <work>/result.json にも書く(tests/kev/tools/event_catalog.py が使う)。
traceprinter はまれに 0xC0000005 で落ちることがあるので、そのときは 3 回まで実行し直す。
traceprinter の出力と、それを含む kev と正解の組は、リポジトリに入れない。
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
    ap.add_argument("--synthetic-dir", type=Path)
    ap.add_argument("--work", type=Path)
    a = ap.parse_args()
    tp = a.traceprinter or env("TRACEHUB_TRACEPRINTER", a.qnx)
    tp = Path(tp) if tp else None
    if tp is None:
        print(f"QNX {a.qnx} の traceprinter の指定(--traceprinter・TRACEHUB_TRACEPRINTER_{a.qnx})が無いので、"
              "正解のファイル(X.tp.txt)のある kev だけを比べます")
    kev_dirs = a.kev_dir or [Path(p) for p in (env("TRACEHUB_KEV_DIR", a.qnx) or "").split(";") if p]

    r = run(["cargo", "build", "--release", "-q", "-p", "tracehub"], cwd=ROOT)
    if r.returncode != 0:
        print(r.stderr.decode(errors="replace"), file=sys.stderr)
        return 1
    exe = ROOT / "target" / "release" / ("tracehub.exe" if os.name == "nt" else "tracehub")

    work = a.work or ROOT / "target" / "compare" / f"qnx{a.qnx}"
    for d in [a.synthetic_dir, *kev_dirs]:
        if d and d.resolve().is_relative_to(work.resolve()):
            print(f"{d} は作業ディレクトリ {work} の中にあります(作業ディレクトリは消して作り直すので、外に写してから渡してください)", file=sys.stderr)
            return 2
    if work.exists():
        shutil.rmtree(work)
    csv_dir, kev_dir, out_dir = work / "csv", work / "kev", work / "out"
    for d in (csv_dir, kev_dir, out_dir):
        d.mkdir(parents=True)
    kevs = []
    if a.synthetic_dir:
        kevs += [("合成", k) for k in sorted(a.synthetic_dir.glob("*.kev"))]
    elif tp is None:
        print("合成データは正解が無いので比べません(--synthetic-dir で正解の組を渡せます)")
    else:
        r = run(["cargo", "run", "--release", "-q", "-p", "kevtool", "--", "--qnx", a.qnx, "scenarios", csv_dir], cwd=ROOT)
        if r.returncode != 0:
            print(r.stderr.decode(errors="replace"), file=sys.stderr)
            return 1
        for f in (ROOT / "tests" / "kev" / f"qnx{a.qnx}" / "data").glob("*.csv"):
            shutil.copy(f, csv_dir / f.name)
        for f in sorted(csv_dir.glob("*.csv")):
            k = kev_dir / (f.stem + ".kev")
            r = run([sys.executable, TOOLS / "kevedit" / "kevcsv.py", "csv2kev", f, "-o", k])
            if not k.exists():
                print(f"{f.name}: kev にできません\n{r.stderr.decode(errors='replace')}", file=sys.stderr)
                return 1
            kevs.append(("合成", k))
    for d in kev_dirs:
        kevs += [("実機", k) for k in sorted(d.glob("*.kev"))]

    fail = skip = 0
    results = []
    for kind, k in kevs:
        tp_out, th_out = out_dir / (k.stem + ".tp.txt"), out_dir / (k.stem + ".th.txt")
        given = k.with_name(k.stem + ".tp.txt")
        if given.exists():
            shutil.copy(given, tp_out)
            err = None
        elif tp is None:
            err = f"正解({given.name})も traceprinter も無い"
        else:
            err = traceprinter(tp, k, tp_out)
            if err is None and k.parent == kev_dir:  # 持ち出せるように、作った kev の横にも正解を置く
                shutil.copy(tp_out, given)
        if err:
            print(f"SKIP {kind} {k.name}: {err}")
            results.append({"kind": kind, "kev": str(k), "result": "skip"})
            skip += 1
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
    n = len(kevs) - skip
    summary = f"{n} 件中 {fail} 件が違います" if fail else (f"{n} 件すべて一致" if n else "比べた kev がありません")
    print(f"QNX {a.qnx}: {summary}" + (f"(SKIP {skip} 件)" if skip else ""))
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
