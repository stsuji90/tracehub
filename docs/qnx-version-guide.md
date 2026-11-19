# QNX の版の定義をそろえる手順

tracehub-kev に QNX の新しい版を足す、今の版の定義を直す、版のサポートを切るための作業の手順。
作業する人(AI を含む)が、この文書と設計書(design.md)・セマンティック ID の資料(semantic-id.md)・試験の資料(testing.md)だけで進められることを目標にしている。
版に固有の値はここに書かない。値は必ず、その版の資料と traceprinter の出力から決める。

例では、足す版を QNX 7.1(`_NTO_VERSION` = 710 → ディレクトリ名 `qnx710`)、前の版を QNX 8.0(`qnx800`)とする。

## 0. 原則

- **traceprinter の出力が正**。資料(ヘッダ、TableOfEvents)と traceprinter の表示が食い違ったら、traceprinter に合わせる。
- **定義とロジックを分ける**。数値・名前・表示文字列は def.rs・args.rs・semantic_id.rs に置き、ロジックは名前で参照する。
- **生成ファイルは手で直さない**(def.rs・kevedit_defs.json・kevedit への埋め込み・semantic-id.md の一覧)。直すときは生成スクリプトか入力を直す。
- **セマンティック ID は追加だけ**(semantic_id.rs)。既存の番号は変えない・消さない。
- **QNX の配布物と traceprinter の出力はリポジトリに入れない**(`inputs/`・`target/` に置く。`.gitignore` 済み)。
- 検証は合成データで行い、最後に実機の kev で確かめる。
- 前の版の出力を変えない。共通の部分(tp_format.rs・kev_header.rs など)を直したら、前の版の比較もすべて一致することを確かめる。

## 1. 全体の流れ

```
SDP のヘッダ・TableOfEvents ──(gen_qnx_def.py)──▶ qnx710/def.rs、tests/kev/qnx710/kevedit_defs.json、kevedit への埋め込み   §3
                                                    │ ビルドが「semantic_id::… が無い」で失敗 → semantic_id.rs に足す
前の版の qnx800/ を複製 ──────────────────────────▶ qnx710/(args.rs・event_parser.rs・event_decoder.rs・traceprinter.rs・test_support.rs) §4
                                                    │ lib.rs・model.rs・kevtool に版を足す
TableOfEvents・traceprinter の出力 ───────────────▶ qnx710/args.rs を直す                                                      §5
traceprinter(7.1)───────(compare.py --qnx 710)──▶ 合成データで全件一致まで直す                                              §6
実機の kev ─────────────────(compare.py --kev-dir)─▶ 実機でも一致                                                                §7
```

### 版で変わりうるものと、直す所

| 変わりうるもの | 気づき方 | 直す所 |
|---|---|---|
| header のビット配置、構造の値、フラグ | def.rs の差分 | 生成されるので直さない。ロジックが名前で参照しているかだけ確かめる |
| クラスの増減(例: 7.1 の RESERVEDCLASS1) | def.rs の `CLASS_*` の差分 | semantic_id.rs(`ClassId` と `ClassId::ALL`)、`qnx710/event_parser.rs` の `semantic_ids`・`internal_ids`・`internal_class`・`DEFINED_CLASSES`、args.rs の `CLASS_DISPLAY`・`DIAG_CLASS_NAMES`・EVENTS |
| イベントの増減・番号の付け替え | def.rs の表の差分、ビルドの失敗 | semantic_id.rs、args.rs の EVENTS |
| CONTROL のイベント(例: 7.1 に BUFFER_END が無い可能性) | def.rs の `CONTROL_*` の差分 | `qnx710/event_parser.rs` の CONTROL の点検(§5.4)、test_support.rs の生成器の枠、試験データ |
| 引数の並び・ラベル・16 進 / 10 進 | 合成データの比較 | args.rs の EVENTS |
| 表示名・表示クラス名 | 合成データの比較 | args.rs の `CLASS_DISPLAY`・`KERCALL_NAMES*`・`.name(...)` |
| 出力の枠(traceprinter のバージョンの行など) | 合成データの比較(1 行目から違う) | args.rs の出力の枠 |
| 組み立ての規則(窓の大きさ、診断の行) | combine の場面の比較 | args.rs の `COMBINE_WINDOW`・`DIAG_*`、必要なら event_parser.rs |
| ヘッダの書式・エラーの基準 | 小さな場面の比較、壊したヘッダ | kev_header.rs(版に共通。前の版も確かめる) |

## 2. 用意するもの

| もの | 用途 | 置き場所の例 |
|---|---|---|
| その版の SDP のヘッダ: `sys/trace.h`、`sys/kercalls.h`、`sys/states.h`、`sys/nto_version.h` | def.rs と kevedit の定義の生成 | `inputs/sdp-headers/sys/` |
| その版の TableOfEvents(イベントごとの引数の一覧) | def.rs の生成(CONTROL の payloads の位置)、args.rs の下書き、確認用のイベント一覧 | `inputs/docs/TableOfEvents.txt` |
| その版の traceprinter(ホストツール) | args.rs の確認(正解) | SDP のホストツールのディレクトリ |
| Python 3.11 以降、Rust(ワークスペースの edition が通るもの)、Node.js(kevedit の試験) | 生成・ビルド・比較 | |

traceprinter の場所は、版ごとの環境変数 `TRACEHUB_TRACEPRINTER_710` に入れておくと、compare.py に毎回渡さなくてよい。
traceprinter が手元に無いときは、ほかの環境で取った正解(`X.tp.txt`)と比べられる(testing.md §4.6)。

## 3. def.rs と kevedit の定義を作る

```
python tests/kev/tools/gen_qnx_def.py --sdp inputs/sdp-headers --table inputs/docs/TableOfEvents.txt
```

- 版はヘッダの `_NTO_VERSION` から決まる(例: 710 → `qnx710`)。書くファイル:
  - `crates/tracehub-kev/src/qnx710/def.rs`(内部 ID の定義)
  - `tests/kev/qnx710/kevedit_defs.json`(kevedit の定義: ビット配置・構造・既定のヘッダ・名前の一覧)
  - `tests/kev/tools/kevedit/kevedit.html` と `kevcsv.py`(すべての版の kevedit_defs.json を埋め込み直す)
- def.rs は semantic_id.rs の定数を名前で参照する(64 ビット版は `名前_64`)。

### 3.1 前の版との差分を見る

```
git diff --no-index crates/tracehub-kev/src/qnx800/def.rs crates/tracehub-kev/src/qnx710/def.rs
```

| 差分の場所 | 見ること |
|---|---|
| ビット配置(`*_SHIFT`・`*_MASK`) | 変わっていれば、ロジックが def.rs の名前で参照していることを確かめる(数値を書いている所があれば直す) |
| `CLASS_*` | クラスの増減・番号の付け替え。§1 の表の「クラスの増減」 |
| `CONTROL_*`・payloads の位置・`BUFFER_COOKIE` | トレースバッファの枠の違い。§5.4 |
| `KERCALL_*`・`BIT64` | カーネルコールの種別・64 ビットのビット |
| クラスごとの表 | イベントの増減・名前の変更・番号の付け替え |

### 3.2 セマンティック ID を足す

ビルド(`cargo build`)で「`semantic_id::xxx::NAME` が無い」と出たら、その名前は前の版に無かったもの。

- 前の版にあった意味のイベントが名前を変えただけなら、def.rs の生成で読み替えるか(gen_qnx_def.py)、新しい ID を足すかを判断する。迷ったら新しい ID を足す(ID は追加しかできないが、足すのは安全)。
- 新しい意味のイベントなら、semantic_id.rs の該当モジュールの末尾に定数を足す(番号はそのモジュールの最大 + 1)。
- クラスを足すときは `ClassId` の定数と `ClassId::ALL` の両方に足す。docs/semantic-id.md の一覧の生成(lib.rs の `semantic_id_doc` の `classes_of`)にも、新しいモジュールのクラスを足す。
- 名前が同じで意味が変わったものがあれば、新しい ID を足すかを判断する(意味が同じなら同じ ID)。
- 規則の詳細は semantic-id.md §3・§6。

### 3.3 生成スクリプトが値を取れないとき

ヘッダの書き方が変わって生成スクリプトが値を取れない(KeyError など)ときは、スクリプトを直す。ヘッダの定義名で引くようにし、値を書き込まない。前の版で `--check` が通ることも確かめる(前の版の生成結果を変えないため)。

## 4. 版のモジュールを作る

1. `crates/tracehub-kev/src/qnx800/` を複製して `qnx710/` を作る。def.rs は §3 で作ったものを使う(複製で上書きしない)。
2. `model.rs`: `QnxVersion` に `V710` を足し、`from_release()`(`TRACE_SYS_RELEASE` の値 → 版)と `name()` を足す。
3. `lib.rs`:
   - `pub mod qnx710 { ... }` でモジュールを宣言する(`qnx800` の宣言を写す)。
   - `match version` に `V710` の腕を足す(`parse`・`decode`・`class_name`・`event_name`・`traceprinter::write`)。
   - 試験 `semantic_id_doc` の `VERSIONS` と `internal`・`internal_class` に版を足す(semantic-id.md の一覧に版の列ができる)。
4. `qnx710/event_parser.rs`: `semantic_ids`・`internal_ids`・`internal_class`・`DEFINED_CLASSES` を確かめる。クラスの増減があれば直す。
5. `qnx710/` の中の `crate::qnx800::…` の参照を `crate::qnx710::…` に直す(test_support.rs など)。
6. 試験データ: `tests/kev/qnx710/data/` を作る。前の版の CSV を写し、次をその版に合わせる。
   - ヘッダの `TRACE_SYS_RELEASE`
   - header のビット配置・クラスとイベントの番号(def.rs の差分のとおり)
   - トレースバッファの枠(BUFFER_END などの有無)
   - `unknown_class.csv`・`unknown_event.csv` の「定義の無い」番号
   同じディレクトリに README.md(場面の一覧)を置く。
7. kevtool: `tests/kev/tools/kevtool/src/main.rs` の `VERSIONS` と `main` の match に、`qnx710::test_support` の腕を足す。
8. `cargo build` が通るまで直す。

compare.py・event_catalog.py・kevedit は版に共通なので、直さなくてよい(`--qnx 710` で選ぶ。kevedit は `TRACE_SYS_RELEASE` で選ぶ)。

2 つ目の版を足したときに、event_parser.rs・event_decoder.rs・traceprinter.rs のうち版で変わらない部分は、共通のモジュールに移してよい。

## 5. args.rs を作る

args.rs は traceprinter の表示の定義で、前の版のものを出発点にする。構成と書き方は design.md §7.1.2。

### 5.1 前の版から変わりやすい所

| 定義 | 確かめ方 |
|---|---|
| `BANNER`(traceprinter のバージョンの行) | traceprinter の出力の先頭 2 行 |
| `EVENTS`(セマンティック ID ごとの書式・表示名・INT_CALL の形) | 新しいイベント・引数の変わったイベント。下の 5.2 |
| `CLASS_DISPLAY`・`DIAG_CLASS_NAMES` | クラスの増減 |
| `KERCALL_NAMES*`(def.rs の名前と違うカーネルコールの表示名) | 合成データの比較(§6)で名前の違う行を探す |
| `THREAD_MAX_PRINTED` | スレッドの状態の追加 |
| `COMBINE_WINDOW`・診断の行(`DIAG_*`) | 組の異常の場面の比較(§6) |
| `SLOG2_*` | slog2 の場面の比較 |

### 5.2 イベントの書式の下書き(TableOfEvents から)

TableOfEvents のブロック(`Class:`・`Event:`・`Fast:`・`Wide:`)を、`ev(クラス ID, イベント ID)` の書式に移す。
EVENTS はセマンティック ID のすべてを 1 行ずつ持つ(足りない・余分な行は試験 `args_rows_cover_all_semantic_ids` が見つける)。
traceprinter が行を出さないイベントも、書式の無い行 `ev(クラス ID, イベント ID),` として置く。

| TableOfEvents | args.rs |
|---|---|
| `Class: _NTO_TRACE_KERCALLENTER` / `EXIT` / `INT` | `ev(ClassId::KER_CALL_ENTER, kercall::名前)` / `KER_CALL_EXIT` / `KER_CALL_INT` |
| `Event: __KER_X\|_NTO_TRACE_KERCALL64` | `kercall::名前_64` |
| def.rs の名前と違う表示名 | `.name("表示名")` |
| 引数の無い INT_CALL の形 | `.intcall()` |
| `Fast:` | `.fast(&[...])`(S と、CB だけの組) |
| `Wide:` | `.wide(&[...])`(組)。Fast と同じなら `.args(&[...])` |
| `Fast: N/A` | `.fast_silent()` |
| 引数 1 つ(32 ビット) | `h(名前, 語)`(16 進)か `d(名前, 語)`(符号付き 10 進)か `u(名前, 語)`(符号なし 10 進) |
| `名前(64)` | `q(名前, 語)` か `e(名前, 語)`(2 語を使う) |
| 文字列(名前・パス) | `sz(名前, 語)` |
| メッセージの先頭 | `s(名前, 語, 語数)` |
| ビットの名前 | `flags(語, TABLE_…, 区切り)` |

例(QNX 8.0 の MSG_SENDV。S は 2 語、組は 6 語):

```rust
ev(ClassId::KER_CALL_ENTER, kercall::MSG_SENDV)
    .fast(&[h("coid", 0), s("msg[0]", 1, 1)])
    .wide(&[h("coid", 0), d("sparts", 1), d("rparts", 2), s("msg[0]", 3, 3)]),
```

- 語の番号は payloads の中の位置(0 から)。64 ビットの引数の後ろは 2 つずれる。
- TableOfEvents が `msg[0]`・`msg[1]`・`msg[2]` と分けて書くところを、traceprinter は 3 語まとめて `msg[0]` として出すことが多い(`s("msg[0]", 3, 3)`)。
- 16 進か 10 進か、ラベルの綴り、条件付きの部分は TableOfEvents からは分からない。合成データの比較で traceprinter に合わせる。
- 書き方の詳細は `tp_format.rs` の先頭のコメント。値の名前の表は `TABLE_*` として足し、要素や関数から直接参照する。
- TableOfEvents の誤記は args.rs で読み替え、コメントに残す。

### 5.3 要素で書けない書式(custom の関数)

条件付きの部分(語数で出す引数が変わる)、区切りの違う書式、値で名前の変わる引数などは、args.rs の中に普通の Rust の関数で書き、`custom(関数)` で使う。

```rust
// 語数で出す引数が変わる(スレッドの状態)
pub const THREAD_BODY: &[Item] = &[custom(|a| {
    out!(a, " pid:{} tid:{} ", a.d(0), a.d(1));
    if a.words() >= 4 {
        out!(a, "priority:{} policy:{}", a.d(2), a.d(3));
    }
})];
```

- 読み出し: `a.u(i)`・`a.d(i)`・`a.q(i)`(64 ビット)・`a.words()`・`a.nul_str(i)`・`a.raw_str(i)`・`a.raw_next()`・`a.enum_name(表, i, 既定)`。無い語は 0 として読める。
- 書き出し: `out!(a, "書式", 値…)`(`format!` と同じ書き方)。
- 合成データの生成器は、関数を語数を変えて動かして読む語を調べる(`Usage`)。関数が語数で分岐していても、生成器は分岐ごとの場面を作る。

### 5.4 トレースバッファの枠が違うとき

CONTROL のイベント(TIME・BUFFER・BUFFER_END・DROPPED)の有無や payloads が前の版と違うとき(例: BUFFER_END が無い)は、args.rs だけでは足りない。

1. 7.1 の traceprinter で、枠の扱いを確かめる: トレースバッファの終わりの決め方、BUFFER のスロット数の意味、枠が崩れたときの診断の行。小さな場面の CSV を kevedit で作って traceprinter に読ませるとよい。
2. `qnx710/event_parser.rs` の CONTROL の点検(design.md §6.4)を、その版の規則に合わせる。DiagKind(model.rs)は版に共通なので、その版で出ないものは出さないだけにする。新しい種類が要るときは足す。
3. `qnx710/test_support.rs` の生成器の `pack()`(スロットをトレースバッファの枠に詰める)を合わせる。
4. 設計書 §3 の「版に共通(と仮定する)」から外し、「版に依存する」に書く。

## 6. traceprinter と比べて直す

```
python tests/kev/tools/compare.py --qnx 710 --traceprinter <7.1 の traceprinter の場所>
```

- 生成器(`qnx710/test_support.rs` の scenarios)は args.rs と def.rs の全イベントについて、語数・文字列・値の名前を変えた変種を作る。kevcsv.py で kev にして、traceprinter と tracehub の出力をバイト単位で比べる。
- 違いがあると、最初の違う行が出る。`target/compare/qnx710/out/` に両方の出力(`X.tp.txt`・`X.th.txt`)があるので、差分を見て args.rs を直す。結果の読み方は testing.md §4.3。
- よくある違いと直し方:

| 違い | 直す所 |
|---|---|
| 1 行目から違う(バージョンの行) | args.rs の `BANNER` |
| ヘッダの行が違う | args.rs の `HEADER_*`、kev_header.rs(版に共通。前の版も確かめる) |
| イベント名が違う | EVENTS の `.name(...)`・`KERCALL_NAMES*` |
| 表示クラス名が違う | `CLASS_DISPLAY` |
| 引数のラベル・16 進 / 10 進・並び | `EVENTS` の書式 |
| traceprinter は行を出さない | `.fast_silent()` / `.wide_silent()` / `.min_words(n)`、`THREAD_MAX_PRINTED` |
| tracehub が行を出さない | EVENTS の行に書式が無い(`ev(...)` だけ)、`FAST_FALLBACK_WIDE` |
| 語が足りないときだけ違う | `custom` の関数で `a.words()` を見る |
| 行の数・位置が違う(TRUNC・ORPHAN) | `COMBINE_WINDOW`、event_parser.rs の組み立て |
| 空行の位置が違う | `BLANK_AFTER` |
| 診断の行が違う | `DIAG_*` |

- 直すたびに全件を流し直し、ほかのイベントを壊していないかを確かめる。catalog の場面は大きいので、まず違いの多いものから直すとよい。
- 1 つのイベントの振る舞いを詳しく調べたいときは、kevedit.html でそのイベントだけの小さな kev を作り、traceprinter に読ませる(`traceprinter -n -f X.kev -o X.txt`)。語数・値を変えて境目を探す。
- traceprinter がスロットの外を読む書式(必要な語より短いイベント)は、生成器が作らないようにしている。生成器を直すときもこの点を守る。
- 新しい書式の要素が要るときは、tp_format.rs に足す(版に共通なので、前の版の出力が変わらないことを確かめる)。

## 7. 実機で確かめる

1. 実機でカーネルコールなどを呼ぶプログラムを動かし、tracelogger で fast と wide の 2 種類の kev を取る。
2. `python tests/kev/tools/compare.py --qnx 710 --kev-dir <DIR>` で比べる。traceprinter の無い環境なら、正解(`X.tp.txt`)を kev の横に置く(testing.md §4.6)。
3. 確認用のイベント一覧で、実機に出たイベントと TableOfEvents との違いを見直す。

```
python tests/kev/tools/event_catalog.py --qnx 710 --table inputs/docs/TableOfEvents.txt --kev-dir <DIR>
```

## 8. 仕上げ

1. `cargo fmt --all`、`cargo clippy --all-targets`、`cargo test` が通る。
2. semantic_id.rs を変えたら、`TRACEHUB_UPDATE_DOCS=1 cargo test` で docs/semantic-id.md の一覧を書き直す。
3. `python tests/kev/tools/gen_qnx_def.py --sdp ... --table ... --check` で def.rs・kevedit の定義・その埋め込みが生成結果と同じ。
4. `python tests/kev/tools/compare.py --qnx 710` と、前の版の `--qnx 800` がすべて一致する。
5. `node tests/kev/tools/kevedit/tests/test_kevedit.mjs` が通る。
6. 実機で取った kev(fast と wide)を `--kev-dir` で比べ、すべて一致する。
7. 確認用のイベント一覧(event_catalog.py)を見直す: TableOfEvents に対応の取れないブロック、行を出すのに TableOfEvents に無いもの。
8. 設計書の「版に依存する部分」に変わったことがあれば書き足す。

## 9. 版のサポートを切る

1. `crates/tracehub-kev/src/qnx<版>/` と `tests/kev/qnx<版>/` を消す。
2. `lib.rs` の `pub mod qnx<版>`、`match version` の腕、試験 `semantic_id_doc` の `VERSIONS`・`internal`・`internal_class` の版を消す。
3. `model.rs` の `QnxVersion` の版と `from_release()`・`name()` の腕を消す。
4. kevtool の `VERSIONS` と match の腕を消す。
5. `python tests/kev/tools/gen_qnx_def.py --embed-only` で kevedit の定義を埋め込み直す。
6. `TRACEHUB_UPDATE_DOCS=1 cargo test` で docs/semantic-id.md の一覧から版の列を消す。
7. semantic_id.rs の ID は消さない(番号を使い回さないため)。
8. §8 の仕上げを流す。

## 10. よくある問題

| 症状 | 原因と対処 |
|---|---|
| ビルドで「`semantic_id::xxx::NAME` が無い」 | 新しいイベント。§3.2 |
| 試験 `args_rows_cover_all_semantic_ids` が「args.rs に行が無い」 | EVENTS に新しい ID の行を足す(行を出さないものも `ev(...)` だけの行を置く) |
| 試験 `args_rows_cover_all_semantic_ids` が「def.rs に無い行」 | その版に無いイベントの行が残っている。消す |
| 試験 `semantic_id_doc_is_up_to_date` が失敗 | `TRACEHUB_UPDATE_DOCS=1 cargo test` |
| `gen_qnx_def.py --check` が「生成結果と違います」 | 生成ファイルを手で直した、またはスクリプトを直した後に生成し直していない。生成し直す |
| compare.py が `SKIP …: traceprinter が 3 回とも 0xC0000005 で落ちました` | traceprinter がその kev を読めない。場面を見直す(traceprinter が失敗する入力は試験にしない) |
| compare.py が `kev にできません` | CSV の書式の誤り。`kevcsv.py check X.csv` で行番号を見る |
| 版を選べない(`unsupported QNX version`) | kevtool の `VERSIONS` に版が無い。§4 の 7 |
| kevedit で名前が前の版のもの | `TRACE_SYS_RELEASE` の値が版と合っていないか、kevedit への埋め込みが古い(`gen_qnx_def.py --embed-only`) |
