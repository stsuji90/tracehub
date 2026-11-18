# QNX の版の定義をそろえる手順

tracehub-kev に QNX の新しい版を足す(または今の版の定義を直す)ための作業の手順。
作業する人(AI を含む)が、この文書と設計書(design.md)だけで進められることを目標にしている。
版に固有の値はここに書かない。値は必ず、その版の資料と traceprinter の出力から決める。

## 0. 原則

- **traceprinter の出力が正**。資料(ヘッダ、TableOfEvents)と traceprinter の表示が食い違ったら、traceprinter に合わせる。
- **定義とロジックを分ける**。数値・名前・表示文字列は def.rs・args.rs・ids.rs に置き、ロジックは名前で参照する。
- **生成ファイルは手で直さない**(def.rs)。直すときは生成スクリプトか入力を直す。
- **セマンティック ID は追加だけ**(ids.rs)。既存の番号は変えない・消さない。
- **QNX の配布物と traceprinter の出力はリポジトリに入れない**(`inputs/`・`target/` に置く。`.gitignore` 済み)。
- 検証は合成データで行い、最後に実機の kev で確かめる。

## 1. 用意するもの

| もの | 用途 | 置き場所の例 |
|---|---|---|
| その版の SDP のヘッダ: `sys/trace.h`、`sys/kercalls.h`、`sys/states.h`、`sys/nto_version.h` | def.rs の生成 | `inputs/sdp-headers/sys/` |
| その版の TableOfEvents(イベントごとの引数の一覧) | def.rs の生成(CONTROL の payloads の位置)、args.rs の下書き | `inputs/docs/TableOfEvents.txt` |
| その版の traceprinter(ホストツール) | args.rs の確認 | SDP のホストツールのディレクトリ |
| Python 3.11 以降、Rust(ワークスペースの edition が通るもの) | 生成・ビルド・比較 | |

## 2. def.rs を作る

```
python tools/gen_qnx_def.py --sdp inputs/sdp-headers --table inputs/docs/TableOfEvents.txt
```

- 版はヘッダの `_NTO_VERSION` から決まり、`crates/tracehub-kev/src/qnx<版>/def.rs` に書かれる(例: 800 → `qnx800`)。
- def.rs は ids.rs の定数を名前で参照する。**ビルドで「ids::xxx::NAME が無い」と出たら、ids.rs の該当モジュールの末尾に定数を足す**(番号はそのモジュールの最大 + 1)。
- 前の版の def.rs と差分をとり、変わった点(番号の付け替え、名前の追加・削除、ビット配置)を把握する。名前が同じで意味が変わったものがあれば、ids.rs に新しい ID を足すかを判断する(意味が同じなら同じ ID)。
- ヘッダの書き方が変わって生成スクリプトが値を取れないときは、スクリプトを直す(ヘッダの定義名で引く。値を書き込まない)。

## 3. 版のモジュールを作る

1. 前の版のディレクトリ(例: `qnx800/`)を複製して `qnx<版>/` を作る。def.rs は 2 で作ったものを使う。
2. `model.rs` の `QnxVersion` に版を足し、`from_release()`(`TRACE_SYS_RELEASE` の値 → 版)と `name()` を足す。
3. `lib.rs` の `match version` に新しい版の腕を足す(`parse`・`decode`・`class_name`・`event_name`・`traceprinter::write`)。
4. `qnx<版>/mod.rs` の `class_code`・`semantic_ids`・`internal_ids`・`class_display` を確かめる。クラスの増減があれば直す。
5. ビルドが通るまで直す(`cargo build`)。

2 つ目の版を足したときに、event_parser.rs・event_decoder.rs・traceprinter.rs のうち版で変わらない部分は、共通のモジュールに移してよい。

## 4. args.rs を作る

args.rs は traceprinter の表示の定義で、前の版のものを出発点にする。

### 4.1 前の版から変わりやすい所

| 定義 | 確かめ方 |
|---|---|
| `BANNER`(traceprinter のバージョンの行) | traceprinter の出力の先頭 2 行 |
| `EVENT_NAMES`・`EVENT_NAMES_64`・`KERCALL_NAMES*`(def.rs の名前と違う表示名) | 合成データの比較(§5)で名前の違う行を探す |
| `INT_CALLS`(int の 64 ビットでない版を INT_CALL の形で出すコール) | 同上 |
| `EVENTS`(イベントごとの書式) | 新しいイベント・引数の変わったイベント。下の 4.2 |
| `THREAD_MAX_PRINTED`・`THREAD_HIDDEN` | スレッドの状態の追加 |
| `COMBINE_WINDOW`・診断の行(`DIAG_*`) | 組の異常の場面の比較(§5) |

### 4.2 イベントの書式の下書き(TableOfEvents から)

TableOfEvents のブロック(`Class:`・`Event:`・`Fast:`・`Wide:`)を、`ev(内部クラス名, 内部イベント名)` の書式に移す。

| TableOfEvents | args.rs |
|---|---|
| `Class: _NTO_TRACE_KERCALLENTER` / `EXIT` / `INT` | `ev("KER_CALL", 名前).kind("ENTER")` / `"EXIT"` / `"INT"` |
| `Event: __KER_X\|_NTO_TRACE_KERCALL64` | `.is64()` |
| `Fast:` | `.fast(&[...])`(S と、CB だけの組) |
| `Wide:` | `.wide(&[...])`(組)。Fast と同じなら `.args(&[...])` |
| `Fast: N/A` | `.fast_silent()` |
| 引数 1 つ(32 ビット) | `h(名前, 語)`(16 進)か `d(名前, 語)`(符号付き 10 進) |
| `名前(64)` | `q(名前, 語)` か `e(名前, 語)`(2 語を使う) |
| 文字列(名前・パス) | `sz(名前, 語)` |
| メッセージの先頭 | `s(名前, 語, 語数)` |

- 語の番号は payloads の中の位置(0 から)。64 ビットの引数の後ろは 2 つずれる。
- 16 進か 10 進か、ラベルの綴り、条件付きの部分(`.when(...)`)は TableOfEvents からは分からない。合成データの比較で traceprinter に合わせる。
- 書き方の詳細は `tp_format.rs` の先頭のコメント。値の名前の表は `TABLES` に足す。
- TableOfEvents の誤記は args.rs で読み替え、コメントに残す。

## 5. traceprinter と比べて直す

```
python tools/compare.py --traceprinter <traceprinter の場所>
```

- 生成器(`qnx<版>/scenarios.rs`)は args.rs と def.rs の全イベントについて、語数・文字列・値の名前を変えた変種を作る。kevcsv.py で kev にして、traceprinter と tracehub の出力をバイト単位で比べる。
- 違いがあると、最初の違う行が出る。`target/compare/out/` に両方の出力があるので、差分を見て args.rs を直す。
- よくある違いと直し方:

| 違い | 直す所 |
|---|---|
| イベント名が違う | `EVENT_NAMES`・`KERCALL_NAMES*` |
| 引数のラベル・16 進 / 10 進・並び | `EVENTS` の書式 |
| traceprinter は行を出さない | `.fast_silent()` / `.wide_silent()` / `.min_words(n)`、`THREAD_MAX_PRINTED` |
| 語が足りないときだけ違う | `.when("words >= n")` |
| 行の数・位置が違う(TRUNC・ORPHAN) | `COMBINE_WINDOW`、event_parser.rs の組み立て |

- traceprinter がスロットの外を読む書式(必要な語より短いイベント)は、生成器が作らないようにしている。生成器を直すときもこの点を守る。
- 新しい書式の要素が要るときは、tp_format.rs に足す(版に共通の言語なので、前の版の出力が変わらないことを確かめる)。

## 6. 仕上げ

1. `cargo fmt`、`cargo clippy --all-targets`、`cargo test` が通る。
2. `python tools/gen_qnx_def.py ... --check` で def.rs が生成結果と同じ。
3. `python tools/compare.py` がすべて一致する。
4. 実機で取った kev(fast と wide)を `--kev-dir` で比べ、すべて一致する。
5. 設計書の「版に依存する部分」に変わったことがあれば書き足す。
