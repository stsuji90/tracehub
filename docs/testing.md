# 試験

tracehub の試験の種類・動かし方・結果の読み方・試験データの足し方。設計書(design.md)§14 の詳細にあたる。

---

## 1. 方針

- **traceprinter の出力が正解**。kev の解釈は公式に公開されていないので、tracehub の正しさは「traceprinter と同じ kev を読ませて、出力がバイト単位で同じか」で確かめる。
- 期待値をリポジトリに持たない。合成データ(CSV)だけをリポジトリに置き、比べるたびに traceprinter を動かす(traceprinter の出力は QNX の配布物の振る舞いそのものなので、リポジトリに入れない)。
- 実機で取りにくいイベント(めったに起きないもの、語の足りない組など)は、合成データで作る。実機の kev は仕上げに使う。
- 版ごとに分ける。試験データは `tests/kev/qnx<版>/`、道具は版に共通で `--qnx <版>` で選ぶ(省略時は最新の版)。

---

## 2. 全体像

| 種類 | 何を確かめるか | 動かし方 | 要るもの |
|---|---|---|---|
| 単体試験 | ヘッダ・組み立て・書式・SLOG2・セマンティック ID・定義の網羅・資料の一覧 | `cargo test` | Rust |
| 合成データの比較 | 生成器の場面と小さな場面の CSV を kev にして、traceprinter と出力を比べる | `python tests/kev/tools/compare.py` | Python、traceprinter(または正解の組) |
| 実機の kev の比較 | 実機で取った kev で同じく比べる | `python tests/kev/tools/compare.py --kev-dir <DIR>` | 実機の kev、traceprinter(または正解) |
| 確認用のイベント一覧 | 全セマンティック ID の定義・書式・TableOfEvents との違い・比較の結果を 1 表にする | `python tests/kev/tools/event_catalog.py --table <TableOfEvents>` | Python、TableOfEvents |
| 生成ファイルの確認 | def.rs・kevedit の定義・その埋め込みが生成結果と同じか | `python tests/kev/tools/gen_qnx_def.py ... --check` | SDP のヘッダ、TableOfEvents |
| kevedit の試験 | kevedit.html と kevcsv.py の往復・一致・版の選び方 | `node tests/kev/tools/kevedit/tests/test_kevedit.mjs` | Node.js(Python があれば kevcsv.py も) |

```
                     ┌─ tests/kev/qnx800/data/*.csv(小さな場面。リポジトリにある)
                     │
kevtool scenarios ───┼─▶ CSV ──(kevcsv.py csv2kev)──▶ kev ─┬─(traceprinter -n -o)──▶ X.tp.txt ─┐
(生成器: 全イベント  │                                       │                                    ├─ バイト単位で比べる ─▶ ok / NG
 などの大きな場面)   │                                       └─(tracehub traceprinter)──▶ X.th.txt ┘
                     └─ --kev-dir の実機の kev ────────────────▲
```

---

## 3. 単体試験(`cargo test`)

| 試験 | ファイル | 確かめること |
|---|---|---|
| `reads_keywords_in_order` | kev_header.rs | ヘッダのキーと値を kev の順に読む。キーの形でない `::` は値に含まれる |
| `values_are_printable_ascii` | kev_header.rs | ヘッダの値は NUL の手前までの印字可能な ASCII だけを残す |
| `errors` | kev_header.rs | ヘッダの誤り(HEADER_END・VER・LITTLE_ENDIAN が無い、対応していない版、2048 バイト以上の項目)をエラーにする。SYS_RELEASE が無ければ 8.0 として読む |
| `skips` | kev_syspage.rs | syspage を読み飛ばす。足りなければエラー |
| `combine_and_merge` | qnx800/event_parser.rs | merge の順と組の組み立て(Complete・Truncated・Orphan) |
| `buffer_checks` | qnx800/event_parser.rs | CONTROL の点検(通番の飛び・スロット数・cookie・BUFFER_END の欠け・途中の別 CPU・外のスロット) |
| `window_truncates` | qnx800/event_parser.rs | 組み立て中の組の後ろに 1022 項目たまると途切れる |
| `semantic_round_trip` | qnx800/event_parser.rs | 内部 ID ⇔ セマンティック ID の変換の往復と UNKNOWN の扱い |
| `spec_compiles` | qnx800/event_decoder.rs | args.rs の定義が読める(行の重複などが無い) |
| `formats` | qnx800/event_decoder.rs | 代表的なイベントの書式(fast / wide の選び方、KER_CALL の名前など。期待値は traceprinter の出力) |
| `formats_like_traceprinter` | qnx800/event_decoder.rs の slog2 | SLOG2 の printf の当てはめ(期待値は traceprinter の出力) |
| `record_rules` | qnx800/event_decoder.rs の slog2 | SLOG2 のレコードの規則(長さ・種別・NUL) |
| `args_and_missing` | tp_format.rs | 書式の要素と、欠けた語の表し方(Traceprinter / Marked) |
| `custom_items` | tp_format.rs | custom の関数(`Out` の読み出しと `out!`) |
| `unknown_events_have_unknown_display_status` | qnx800/test_support.rs | 定義の無いイベントは Unknown(SEC のように traceprinter が行を出すものは Display) |
| `scenarios_are_well_framed_and_decodable` | qnx800/test_support.rs | 生成器の場面が正しい枠で、全イベントを両方の `MissingStyle` で decode できる |
| `args_rows_cover_all_semantic_ids` | qnx800/test_support.rs | args.rs の EVENTS が、版の def.rs にあるセマンティック ID のすべてを 1 行ずつ持つ |
| `catalog_covers_all_semantic_ids` | qnx800/test_support.rs | 全イベント網羅の場面(catalog)に、セマンティック ID のすべてがある。`--nocapture` で件数を出す |
| `class_ids_are_unique` | semantic_id.rs | `ClassId::ALL` の名前と番号に重複が無く、UNKNOWN と重ならない |
| `semantic_id_doc_is_up_to_date` | lib.rs | docs/semantic-id.md の一覧が semantic_id.rs と版の定義に合っている |

- 期待値の文字列は traceprinter の出力から写したもの(試験の中にコメントで書いてある)。
- `semantic_id_doc_is_up_to_date` が失敗したら、`TRACEHUB_UPDATE_DOCS=1 cargo test` で一覧を書き直す(PowerShell では `$env:TRACEHUB_UPDATE_DOCS=1; cargo test`)。
- `cargo clippy --all-targets` と `cargo fmt --all --check` も通ること。

---

## 4. 合成データの比較(compare.py)

### 4.1 動かし方

```
python tests/kev/tools/compare.py [--qnx 800] [--traceprinter PATH] [--kev-dir DIR ...] [--synthetic-dir DIR] [--work DIR]
```

| オプション | 省略時 | 内容 |
|---|---|---|
| `--qnx` | 最新の版(`tests/kev/qnx*/` の最大) | 版 |
| `--traceprinter` | 環境変数 `TRACEHUB_TRACEPRINTER_<版>`、無ければ `TRACEHUB_TRACEPRINTER` | その版の traceprinter(SDP のホストツール) |
| `--kev-dir` | 環境変数 `TRACEHUB_KEV_DIR_<版>`、無ければ `TRACEHUB_KEV_DIR`(`;` 区切り) | 実機の kev の置き場所(複数可) |
| `--synthetic-dir` | なし | 合成データの kev と正解の組(§4.6) |
| `--work` | `target/compare/qnx<版>` | 作業ディレクトリ(実行のたびに消して作り直す) |

手順の中身:

1. `cargo build --release -p tracehub` で CLI を作る。
2. `cargo run -p kevtool -- --qnx <版> scenarios <work>/csv` で生成器の場面を書き、`tests/kev/qnx<版>/data/*.csv` を写す。
3. `kevcsv.py csv2kev` で各 CSV を `<work>/kev/X.kev` にする。
4. 各 kev について、正解(traceprinter の出力)と `tracehub traceprinter X.kev` の出力を比べる。

### 4.2 合成データの場面

生成器(`qnx<版>/test_support.rs` の scenarios。大きいのでリポジトリには CSV を置かず、毎回作る):

| 場面 | 内容 |
|---|---|
| catalog | args.rs と def.rs の全イベント(定義の無い番号・クラスを含む)。payloads の語数・文字列・値の名前を変えた変種 |
| short_words | 語が足りないイベント(S・CB だけ・CB CC で途切れた組) |
| combine_window | 組み立て中の組の後ろにたまる項目の数の境目(窓) |
| orphan_lookahead | 多数のイベントの後の orphan(ORPHAN 行の位置) |
| slog2 | SLOG2 のレコードの規則と printf の変換 |
| sched_wrap | 4 CPU・下位 32 ビットの一周(TIME あり)・CPU 間の同時刻 |
| sched_wrap_inferred | 同じ列で、一周のときの TIME が無い |

小さな場面(`tests/kev/qnx800/data/`。手で作った CSV。内容は同じディレクトリの README.md):
minimal、combine_edges、combine_eof、combine_interleave、combine_keys、flags_diag、injected、reversal、seq_gap、unknown_class、unknown_event。

### 4.3 結果の読み方

画面には kev ごとに 1 行と、最後にまとめの 1 行が出る。

```
ok   合成 minimal.kev(1520 バイト)
NG   実機 foo.kev: 120 行目
  traceprinter: b't:0x... KER_CALL :MSG_SENDV/11 ...'
  tracehub    : b't:0x... KER_CALL :MSG_SENDV/11 ...'
SKIP 実機 bar.kev: 正解(bar.tp.txt)も traceprinter も無い
QNX 800: 17 件中 1 件が違います(SKIP 1 件)
```

| 表示 | 意味 |
|---|---|
| `ok` | 出力が完全に一致。括弧の中は出力の大きさ |
| `NG` | 違いがある。最初に違う行の行番号と、両方のその行(バイト列)。tracehub が失敗したときも NG |
| `SKIP` | 比べていない(traceprinter の失敗、または正解も traceprinter も無い) |
| `合成` / `実機` | 合成データ(生成器・data・`--synthetic-dir`)/ `--kev-dir` の kev |

| 終了コード | 意味 |
|---|---|
| 0 | NG が無い(SKIP はあってもよい) |
| 1 | NG がある、またはビルド・kev への変換が失敗した |
| 2 | 引数の誤り(作業ディレクトリの中のディレクトリを渡した、など) |

作業ディレクトリ(`target/compare/qnx<版>/`):

| もの | 中身 |
|---|---|
| `csv/` | 比べた場面の CSV |
| `kev/X.kev`・`kev/X.tp.txt` | kev と正解(traceprinter の出力)の組。そのまま `--synthetic-dir` に使える |
| `out/X.tp.txt`・`out/X.th.txt` | traceprinter と tracehub の出力 |
| `result.json` | kev ごとの `kind`(合成 / 実機)・`kev`・`result`(ok / ng / skip)。event_catalog.py が読む |

### 4.4 NG のとき

1. `out/X.tp.txt` と `out/X.th.txt` の差分を見る(行の改行は CRLF)。
2. どのイベントかを、行の表示クラス名・表示名から決める。合成データなら `csv/X.csv` の該当スロットも見られる。
3. 直す所の目安(詳しくは qnx-version-guide.md §5):

| 違い | 直す所 |
|---|---|
| 引数のラベル・16 進 / 10 進・並び | args.rs の EVENTS の書式 |
| イベント名 | EVENTS の `.name(...)`、`KERCALL_NAMES*` |
| traceprinter は行を出さない | `.fast_silent()`・`.wide_silent()`・`.min_words(n)` |
| 行の数・位置(TRUNC・ORPHAN) | `COMBINE_WINDOW`、event_parser.rs の組み立て |
| ヘッダの行・枠 | args.rs の出力の枠、traceprinter.rs |

4. 直したら compare.py を流し直し、全件 ok を確かめる(ほかのイベントを壊していないか)。

### 4.5 traceprinter のまれな失敗

traceprinter はまれに 0xC0000005 で落ちる。compare.py は 3 回まで実行し直し、それでも落ちれば SKIP にする。

### 4.6 traceprinter の無い環境

kev の横に同じ名前の正解 `X.tp.txt` があれば、traceprinter を動かさずにそれと比べる。

- 実機の kev: traceprinter のある環境で `traceprinter -n -f X.kev -o X.tp.txt` を取り、kev の横に置いて `--kev-dir` で渡す。
  標準出力をリダイレクトしたものは正解にならない(末尾の集計の行が混ざり、改行も違う)。
- 合成データ: traceprinter のある環境で compare.py を動かしたあとの `target/compare/qnx<版>/kev/` を写し、`--synthetic-dir` で渡す。
  手元で作り直した合成データに、ほかの環境の正解は使えない(生成器を直すと kev の中身が変わる)。
- 作業ディレクトリの中のディレクトリは渡せない(実行のたびに消すため)。外に写してから渡す。

---

## 5. 実機の kev

- 実機でカーネルコールなどを呼ぶプログラムを動かし、tracelogger で fast と wide(`-w`)の 2 種類の kev を取る。
- `compare.py --kev-dir <DIR>` で比べる。正解を別の環境で取ったときは §4.6。
- 実機の kev とその出力はリポジトリに入れない(実機の値を含むため)。
- 実機の kev に出たイベントの件数は、確認用のイベント一覧(§6)の「実機」の列に入る。

---

## 6. 確認用のイベント一覧(event_catalog.py)

tracehub-kev が想定しているイベントの全体を、1 つの表で見直すための道具。次の観点で確かめるときに使う。

| 観点 | 見る所 |
|---|---|
| 全部でどれだけのイベントを想定しているか | summary.md の件数(セマンティック ID の数、traceprinter が行を出すものの数) |
| 各イベントの書式を何と実装しているか | events.csv の fast・wide の書式と出力の例 |
| TableOfEvents との違い | events.csv の TableOfEvents の列と引数名の違い、summary.md の対応の取れないブロック |
| 合成 kev での突き合わせ | events.csv の合成の件数と比較の結果 |
| 実機 kev での突き合わせ | events.csv の実機の件数と比較の結果 |

```
python tests/kev/tools/compare.py                     # 先に比べておく(result.json を読むため)
python tests/kev/tools/event_catalog.py [--qnx 800] --table <TableOfEvents.txt> [--kev-dir <DIR>]
```

| オプション | 省略時 |
|---|---|
| `--table` | 環境変数 `TRACEHUB_TABLE_OF_EVENTS_<版>`、無ければ `TRACEHUB_TABLE_OF_EVENTS`(無ければ TableOfEvents の列は空) |
| `--kev-dir` | 環境変数 `TRACEHUB_KEV_DIR_<版>`、無ければ `TRACEHUB_KEV_DIR` |
| `--out` | `target/event_catalog/qnx<版>/` |

出力(TableOfEvents の文を含むので、リポジトリに入れない):

- `events.csv`(Excel で開ける UTF-8): セマンティック ID ごとに 1 行。

| 列 | 内容 |
|---|---|
| クラス・イベント・class_id・event_id・内部 ID | セマンティック ID と、対応する内部 ID |
| 表示名 | traceprinter の表示クラス名・表示イベント名 |
| fast の書式・wide の書式 | args.rs の書式(要素の並び。fast が無く wide を使うものは「wide と同じ」、custom はラベルを拾って示す) |
| 出力の例 | 合成 kev での出力(traceprinter と一致を確かめたもの) |
| TableOfEvents | 対応するブロックの有無と、Fast・Wide の引数名 |
| 引数名の違い | TableOfEvents の引数名と tracehub のラベルの違い(参考。traceprinter のラベル自体が TableOfEvents と違うことが多い) |
| 合成 kev・実機 kev | 件数、traceprinter が行を出した件数、compare.py の結果 |

- `summary.md`: 件数のまとめ、TableOfEvents との対応の取れないブロック、行を出すのに TableOfEvents に無いもの、比較の結果。

中身は kevtool の `catalog`(`qnx<版>/test_support.rs` の catalog)が作る TSV に、Python が TableOfEvents と result.json を並べたもの。

---

## 7. 生成ファイルと資料の確認

| 確かめるもの | コマンド |
|---|---|
| def.rs・kevedit_defs.json・kevedit への埋め込みが生成結果と同じ | `python tests/kev/tools/gen_qnx_def.py --sdp <SDP のヘッダ> --table <TableOfEvents> --check` |
| kevedit への埋め込みだけ(SDP が無いとき) | `python tests/kev/tools/gen_qnx_def.py --embed-only --check` |
| docs/semantic-id.md の一覧 | `cargo test`(`semantic_id_doc_is_up_to_date`) |

---

## 8. kevedit の試験

```
node tests/kev/tools/kevedit/tests/test_kevedit.mjs [kev のディレクトリ]
```

- 試験の中で作った kev(リトル / ビッグエンディアン、ヘッダの前のバイト、syspage、末尾の端数)と、渡したディレクトリの `*.kev` について、kev → kev、kev → CSV → kev がバイト単位で元に戻るか。
- kevcsv.py(Python があるとき)が kevedit.html と同じ CSV・kev を作るか。
- 版の選び方(`TRACE_SYS_RELEASE`、定義の無い版、無いとき)、ヘッダの注意、文字列と数値の読み方。
- compare.py の作業ディレクトリの kev を渡すと、合成データ全部で往復を確かめられる: `node ... target/compare/qnx800/kev`。

---

## 9. 試験を足す

### 小さな場面(CSV)を足す

1. `tests/kev/tools/kevedit/CSV_FORMAT.md` の書式で CSV を書く(kevedit.html で作って「CSV を保存」してもよい)。
2. `python tests/kev/tools/kevedit/kevcsv.py check X.csv` でヘッダの注意が無いことを確かめる。
3. `tests/kev/qnx<版>/data/` に置き、同じディレクトリの README.md の表に 1 行足す。
4. compare.py で traceprinter と比べる。traceprinter が読めない形(内部エラー)なら、その場面は試験にしない(design.md §1 の前提)。

### 生成器の場面を足す

大きい場面や、全イベントについて変種を作るものは生成器に足す(`qnx<版>/test_support.rs` の `scenarios()`)。

- 場面は `(名前, CSV)`。スロットを並べ、`pack()` でトレースバッファの枠に詰めて `to_csv()` で CSV にする。
- traceprinter がスロットの外を読む書式(必要な語より短いイベント)は作らない。custom の関数が読む語は `Usage` が調べる。
- 足したら、モジュールの先頭の表に 1 行足す。

### 単体試験を足す

- 版に共通の部分(ヘッダ・tp_format)は、そのファイルの `tests` モジュールに。
- 版に依存する部分は `qnx<版>/` の各ファイルに。期待値は traceprinter の出力から写し、どこから写したかをコメントに書く。

---

## 10. リポジトリに入れないもの

| もの | 置き場所 |
|---|---|
| QNX の配布物(SDP のヘッダ、TableOfEvents、traceprinter) | `inputs/` など(リポジトリの外) |
| traceprinter の出力、正解の組(`X.tp.txt`) | `target/` かリポジトリの外 |
| 実機の kev | リポジトリの外 |
| 確認用のイベント一覧(TableOfEvents の文を含む) | `target/event_catalog/` |

---

## 11. 仕上げのチェックリスト

変更をコミットする前に、次がすべて通ること。

```
cargo fmt --all --check
cargo clippy --all-targets
cargo test
python tests/kev/tools/compare.py
node tests/kev/tools/kevedit/tests/test_kevedit.mjs
python tests/kev/tools/gen_qnx_def.py --embed-only --check
```

定義(def.rs)を作り直したときは、`gen_qnx_def.py --sdp ... --table ... --check` も。実機の kev があれば `compare.py --kev-dir <DIR>` も。
