# tracehub 設計書

## 1. 目的と範囲

tracehub は、いろいろな形式のトレース情報を読み、分析しやすい形式に変換して出力するプログラムである。

第 1 段階では、QNX のカーネルトレース(kev ファイル)だけを扱う。

| 項目 | 第 1 段階 | 後の段階 |
|---|---|---|
| 入力 | kev(QNX 8.0) | ほかの QNX の版、ほかのトレース形式 |
| ライブラリの出力 | `KevFile`(§5)、`decode()` による文字列(§7) | 別アプリ向けのデータ形式 |
| CLI の出力 | traceprinter 互換のテキスト(`-n` 相当) | 人が読むための出力など |

### 前提

kev の解釈は公式に公開されていない。tracehub の解釈は、traceprinter(QNX 公式のツール)の出力を正として、その振る舞いから決めたものである。

- traceprinter が出力するものは、tracehub も出力する(traceprinter が出すのに tracehub が出さない、を作らない)。
- traceprinter が失敗する入力(異常終了・内部エラー)での振る舞いは決めない(実装の成り行きでよい)。

---

## 2. 用語

| 用語 | 意味 |
|---|---|
| スロット(slot) | kev のイベント部の単位。u32 × 4 = 16 バイト |
| word | スロットの u32 の 1 つ。意味を問わないときの呼び名 |
| header | スロットの word[0]。構造・CPU・フラグ・内部クラス・内部イベント番号のビット列 |
| timestamp_cycles_low32 | スロットの word[1]。時刻(クロック値)の下位 32 ビット |
| payloads | 解釈する前の u32 の列。スロットでは word[2]・word[3]。組では各スロットの payloads をつないだもの |
| args | payloads を解釈して意味を持たせたもの(名前と値、文字列) |
| 構造(struct) | header のビット 30〜31。S(単独)/ CB(組の始まり)/ CC(組の途中)/ CE(組の終わり) |
| 組(複合イベント) | CB, CC…, CE の複数スロットで 1 つのイベントになるもの |
| トレースバッファ | CPU ごとのバッファ。kev では、同じ CPU のスロットが連続した範囲で、TIME・BUFFER で始まり BUFFER_END で終わる |
| CPU 別スロット列 | ある CPU のスロットを、ファイルの順に並べたもの |
| reconstruct_timestamp | CPU 別スロット列で、timestamp_cycles_low32 に上位 32 ビットを補って 64 ビットの時刻にする処理(§6.3) |
| merge | CPU 別スロット列を、時刻の順に 1 列にまとめる処理(§6.5) |
| combine | merge した列のスロットから、組を組み立てる処理(§6.6) |
| 項目 | combine の単位。S のイベント・組・orphan のスロットがそれぞれ 1 項目 |
| 内部 ID | kev に書かれている番号。内部クラス ID(`int_class_id`)と内部イベント ID(`int_event_id`)。QNX の版に依存する |
| セマンティック ID | tracehub が決める、QNX の版に依存しない番号。クラス ID(`class_id`)とイベント ID(`event_id`)(§8) |
| 診断(DiagInfo) | 解析の途中で見つかった、組み立ての異常とトレースバッファの不整合(§5.4) |
| traceprinter 互換出力 | `traceprinter -n -f X.kev -o OUT` が OUT に書く内容と、バイト単位で同じ出力(§9) |
| 書式 | traceprinter がイベントの args を表示する形。版ごとの args.rs に書く(§7) |

---

## 3. 構成

```
tracehub/
  Cargo.toml                   ワークスペース(パッケージを持たない)
  apps/
    tracehub/src/main.rs       CLI(tracehub コマンド)
  crates/
    tracehub-kev/              kev に関するすべての処理(ライブラリ)
      src/
        lib.rs                 公開 API。版のモジュール(qnx800 など)の宣言
        model.rs               KevFile・TraceEvent・DiagInfo などの型
        semantic_id.rs         セマンティック ID(版に依存しない。手で保守)
        kev_header.rs          kev のヘッダの読み取り(版に共通)
        kev_syspage.rs         syspage の読み飛ばし(版に共通)
        tp_format.rs           書式の部品と、その当てはめ(版に共通)
        qnx800/                QNX 8.0 に依存する部分
          def.rs               内部 ID の定義(生成ファイル)
          args.rs              traceprinter の表示の定義(手で保守)
          event_parser.rs      スロットの読み取りからイベントの列まで(§6)。内部 ID ⇔ セマンティック ID
          event_decoder.rs     TraceEvent → 表示文字列(§7)。SLOG2(§7.4)を含む
          traceprinter.rs      traceprinter 互換出力(§9)
          test_support.rs      試験・確認用のコード: 合成データの生成と確認用のイベント一覧(§14。tests/kev/tools/kevtool が呼ぶ)
  tests/
    kev/                       kev 用の道具と試験データ(形式ごとにディレクトリを分ける)
      tools/                   道具(版に共通。版は --qnx で選ぶ)
        gen_qnx_def.py         SDP のヘッダと TableOfEvents から def.rs と kevedit の定義を生成する
        compare.py             traceprinter と tracehub の出力を比べる
        event_catalog.py       確認用のイベント一覧を作る(TableOfEvents・比較の結果・実機の kev と並べる)
        kevtool/               合成データ・確認用の一覧を書き出す小さな実行ファイル(compare.py・event_catalog.py が呼ぶ。中身は test_support.rs)
        kevedit/               kev を見る・直す道具(kevedit.html、kevcsv.py、CSV_FORMAT.md)
      qnx800/                  QNX 8.0 の試験データ(版のサポートを切るときはディレクトリごと消す)
        data/                  小さな場面の CSV
        kevedit_defs.json      kevedit の定義(生成ファイル)
  docs/
    design.md                  この文書
    semantic-id.md             セマンティック ID の説明と一覧(一覧は生成部分)
    testing.md                 試験の種類・動かし方・結果の読み方・試験データの足し方
    qnx-version-guide.md       QNX の版を足す・定義を直す・サポートを切る手順(別の作業者向け)
```

- 実行ファイル(CLI、将来の GUI)は `apps/`、ライブラリは `crates/` に置く。
- トレースの形式ごとのライブラリは `crates/tracehub-<形式>`、その道具と試験データは `tests/<形式>/` に置く。

### モジュールの責務と主な関数

| モジュール | 責務 | 主な関数・定義 |
|---|---|---|
| `lib.rs` | 公開 API。版ごとのモジュールへの振り分け(`match version`) | `parse`・`decode`・`class_name`・`event_name`・`traceprinter::write` |
| `model.rs` | 公開する型 | `KevFile`・`KevHeader`・`TraceEvent`・`DiagInfo`・`QnxVersion`・`KevError` |
| `semantic_id.rs` | セマンティック ID の定数 | `ClassId`・`EventId`・`ClassId::ALL`・クラスごとのモジュール |
| `kev_header.rs` | ヘッダの文字列を項目に分け、誤りを見つけ、版を決める | `parse` |
| `kev_syspage.rs` | syspage の長さだけ読み飛ばす | `skip` |
| `tp_format.rs` | 書式の要素(`h`・`d`・`custom` など)と、それを payloads に当てはめる処理。QNX 固有の名前や番号は持たない | `render`・`Out`・`out!`・`Usage` |
| `qnx800/def.rs` | 内部 ID の定義(生成ファイル) | `CLASS_*`・`CONTROL_*`・ビット配置・クラスごとの表 |
| `qnx800/args.rs` | traceprinter の表示の定義(手で保守) | `EVENTS`・`CLASS_DISPLAY`・出力の枠・診断の行・SLOG2 の定数 |
| `qnx800/event_parser.rs` | スロットの読み取りから TraceEvent の列まで。header の各部の取り出し、内部 ID ⇔ セマンティック ID | `parse`・`semantic_ids`・`internal_ids`・`header_bits` |
| `qnx800/event_decoder.rs` | 1 イベントの表示文字列(行を出すか、名前、書式の選択)。SLOG2 | `format`・`names`・`blank_after`・`class_display`・`slog2::render` |
| `qnx800/traceprinter.rs` | traceprinter 互換出力(枠・ヘッダの行・イベントの行・診断の行) | `write` |
| `qnx800/test_support.rs` | 試験・確認用(ライブラリの機能ではない) | `scenarios::scenarios`・`catalog::entries` |

ロジック(event_parser・event_decoder・traceprinter)は、数値と表示文字列を def.rs・args.rs の名前で参照する。版で変わるのは基本的に def.rs と args.rs で、ロジックは版をまたいで同じ形を保つ。

### 版に依存する部分と依存しない部分

| 版に共通(と仮定する) | 版に依存する |
|---|---|
| ヘッダの書式、syspage の位置、スロットの大きさ、トレースバッファの枠、merge と combine の規則、書式の部品(tp_format.rs) | header のビット配置、内部 ID の値、内部 ID → セマンティック ID の対応、書式、traceprinter の表示文字列 |

トレースバッファの枠は、QNX 7.1 では BUFFER_END が無い可能性がある(未確認)。版を足すときに traceprinter で確かめる。

- 定義(def.rs・args.rs)とロジックを分ける。ロジックは定義の名前を参照し、数値や表示文字列を持たない。
- 版を足すときは、`qnxNNN/` を足して、版の enum(`QnxVersion`)に 1 つ加える。
- event_parser.rs・event_decoder.rs・traceprinter.rs のうち版に共通な部分は、2 つ目の版を足すときに共通のモジュールへ移してよい(第 1 段階では qnx800 の中に置く)。

---

## 4. 公開 API

```rust
/// kev を読む。ヘッダの誤りと、対応していない版はエラー
pub fn parse(bytes: &[u8]) -> Result<KevFile, KevError>;

/// TraceEvent 1 件の表示文字列(traceprinter -n と同じ書式)
pub fn decode(version: QnxVersion, event: &TraceEvent, opts: &DecodeOptions) -> DecodedEvent;

/// セマンティック ID の表示文字列(traceprinter が表示するクラス名・イベント名)
pub fn class_name(version: QnxVersion, class_id: ClassId) -> Option<&'static str>;
pub fn event_name(version: QnxVersion, class_id: ClassId, event_id: EventId) -> Option<String>;

/// traceprinter 互換出力
pub mod traceprinter {
    pub fn write(kev: &KevFile, out: &mut impl std::io::Write) -> std::io::Result<()>;
}
```

| 型 | 内容 |
|---|---|
| `QnxVersion` | 対応する QNX の版の enum。第 1 段階は `V800` だけ。`KevHeader` の `TRACE_SYS_RELEASE` から決まる |
| `DecodeOptions` | `missing`: 欠けた payloads の表し方(§7.2) |
| `DecodedEvent` | `class_name`・`event_name`・`args`(いずれも文字列) |
| `KevError` | 読めない理由(§10) |

- ヘッダの値は印字可能な ASCII。イベントの文字列(args など)は payloads のバイトを 1 バイト 1 文字にしたもの(traceprinter はバイトをそのまま出力するので、文字コードは決まっていない)。
- `event_name` が `String` を返すのは、KER_CALL の表示名(`MSG_SENDV/11` など)が番号を含むため。
- ファイルを読む関数は持たない(呼ぶ側がバイト列を渡す)。

---

## 5. データモデル

### 5.1 KevFile

```rust
pub struct KevFile {
    pub header: KevHeader,
    pub events: Vec<TraceEvent>,   // traceprinter の並び(merge と combine の結果の順)
    pub diag: Vec<DiagInfo>,       // 時刻の順
}
```

### 5.2 KevHeader

```rust
pub struct KevHeader {
    pub version: QnxVersion,
    pub cycles_per_sec: u64,              // 無い・数値でなければ 0
    pub cpu_num: u32,                     // 同上
    pub syspage_len: u64,                 // 無い・空なら 0
    pub keywords: Vec<(String, String)>,  // キー(TRACE_ を含み :: を除いた名前。例: TRACE_VER_MAJOR)と値。kev に書かれた順
}
```

- keywords は kev に書かれた順で持つ。traceprinter 互換出力のヘッダの行は、この順に書く。
- 同じキーが 2 回以上あれば、すべて持つ。`get()` は最後のものを返す。
- 値は、traceprinter の表示と同じく、NUL の手前までの印字可能な ASCII(0x20〜0x7E)だけを残す(実測: traceprinter はそれ以外のバイトを表示しない)。
- バイト順: リトルエンディアン(`TRACE_LITTLE_ENDIAN` がある kev)だけを読む。それ以外はエラー(§10)。
  - ビッグエンディアンの kev は traceprinter も読むが、同じ時刻の TIME と BUFFER の並びがリトルエンディアンのときと違う(TIME のスロット自身の時刻に、更新前の上位 32 ビットを使っているように見える。未確認)。QNX 8.0 の対象(x86_64・aarch64)はリトルエンディアンなので、扱わない。
- fast / wide の区別は持たない。traceprinter は `TRACE_TRACELOGGER_ARGS` を見ず、イベントの構造で書式を選ぶ(§7.1)。

### 5.3 TraceEvent

```rust
pub struct TraceEvent {
    pub timestamp_cycles: u64,       // TIME だけで補った 64 ビットの時刻(§6.3)
    pub cpu_id: u8,
    pub class_id: ClassId,           // セマンティック ID(§8)
    pub event_id: EventId,
    pub int_class_id: u8,            // header の内部クラス
    pub int_event_id: u16,           // header の内部イベント番号
    pub flags: u8,                   // header のフラグ(ビット 0 = _NTO_TRACE_PROCESS_INJECTED)。組は CB のフラグ
    pub combine_status: CombineStatus,
    pub display_status: DisplayStatus,
    pub payloads: Vec<u32>,          // 各スロットの payloads(word[2]・word[3])をつないだもの
}

pub enum CombineStatus { Complete, Truncated, Orphan }
pub enum DisplayStatus { Display, Silent, Unknown }
```

| combine_status | 意味 | payloads |
|---|---|---|
| `Complete` | S のスロット 1 つのイベント、または CB から CE までそろった組 | S なら 2 語、組なら 2 × スロット数 |
| `Truncated` | CE が来ないまま途切れた組(§6.6) | 集まったスロットの分 |
| `Orphan` | 組み立て中の組が無いのに来た CC / CE のスロット 1 つ | 2 語 |

| display_status | 意味 |
|---|---|
| `Display` | traceprinter がイベントの行、または TRUNC・ORPHAN の診断の行を出力するもの |
| `Silent` | 定義のあるイベントで、traceprinter が何も出力しないもの(header が 0 のスロット、書式で出力しないと決まっているイベント、語の足りないイベントなど) |
| `Unknown` | 定義の無いイベント(class_id か event_id が UNKNOWN)で、traceprinter がイベントの行を出さないもの。traceprinter は何も出力しない(定義の無いクラスだけは UNKNOW の診断の行を出す)。header が 0 のスロットは Silent |

- 決める順: Orphan → Display、イベントの行を出す → Display、定義の無いイベント → Unknown、TRUNC の行を出す → Display、それ以外 → Silent。
- 定義の無いイベントでも traceprinter がイベントの行を出すもの(SEC の `unknown(番号)`、USER・QUIP の任意番号など)は Display。
- traceprinter は、定義のあるクラスの定義の無いイベント(例: 古い trace.h にある SYSTEM の `_NTO_TRACE_SYS_SEND_IPI`(0x20)とその 64 ビット版)には何も出さない(実測)。tracehub の traceprinter 互換出力も何も書かず、display_status を Unknown にして区別できるようにする。

- traceprinter の診断の行(TRUNC・ORPHAN・UNKNOW)はスロットの words を表示する。tracehub は先頭スロットの words を TraceEvent のフィールド(構造・CPU・フラグ・内部 ID・時刻・payloads)から組み直す。構造は、Orphan なら DiagInfo の `structure`(CC / CE)、組(Complete で 3 語以上、または Truncated)なら CB、それ以外は S。
- 時刻は組では CB(Orphan ではそのスロット)の時刻。

### 5.4 DiagInfo

```rust
pub struct DiagInfo {
    pub timestamp_cycles: u64,   // その異常が確定したスロットの、TIME だけで補った時刻(§6.3)
    pub cpu_id: u8,
    pub kind: DiagKind,
}
```

| DiagKind | 確定するとき | 持つ情報 |
|---|---|---|
| `Orphan` | CC / CE に組み立て中の組が無かった | TraceEvent の番号、traceprinter が表示する番号(merge した順のスロットの通し番号)、スロットの構造(CC / CE)、ORPHAN 行を出す位置(§9.3) |
| `Truncated` | 組が途切れた | TraceEvent の番号、traceprinter が表示する番号(orphan を除くイベントの通し番号)、途切れた理由(同じ鍵の CB / 窓 / 列の終わり) |
| `SeqGap` | BUFFER の通番が、同じ CPU の前の BUFFER の通番 + 1 でなかった | 前の通番、今の通番 |
| `CountMismatch` | BUFFER_END で、BUFFER に書かれたスロット数と実際の数(直前の TIME から BUFFER_END まで)が違った | 通番、BUFFER の数、実際の数 |
| `Dropped` | DROPPED があった | 失われたイベントの数 |
| `MissingBufferEnd` | BUFFER_END が来ないまま次の BUFFER が来た、または列が終わった | 通番 |
| `CookieMismatch` | BUFFER_END の payloads が cookie(`0xe6e6e6e6`)でなかった | 通番、2 つの値 |
| `WrapWithoutTime` | TIME が無いのに timestamp_cycles_low32 が半周より大きく減った | 前と今の timestamp_cycles_low32 |
| `Interleaved` | トレースバッファの途中に、別の CPU のスロットがあった(トレースバッファごとに 1 件) | トレースバッファの通番、別の CPU の番号 |
| `OutsideBuffer` | トレースバッファの外(BUFFER の前、BUFFER_END の後)にスロットがあった | 続いた範囲ごとに 1 件。スロット数 |
| `TrailingBytes` | ファイルの末尾に 16 バイトに満たない端数があった | バイト数 |

- `cpu_id` は異常の起きた CPU(Interleaved はトレースバッファの CPU)。
- 時刻: Interleaved・MissingBufferEnd(列の終わり)はそのトレースバッファの最後のスロット、OutsideBuffer は続いた範囲の最初のスロット、TrailingBytes は 0。
- 時刻の範囲は持たない。欠けた範囲は、前後の値(SeqGap の前の通番と今の通番など)で分かる。
- payloads や words は持たない。必要なら TraceEvent の番号から引く。

---

## 6. parse() の処理

```
バイト列
  → kev_header::parse          ヘッダ → KevHeader、版の決定
  → kev_syspage::skip          syspage を読み飛ばす
  → event_parser::parse        (版ごと)
       1. スロットの読み取り          → Vec<Slot>
       2. CPU 別スロット列に分ける
       3. reconstruct_timestamp(CPU ごと)
       4. CONTROL の点検(ファイルの順に、CPU ごとの状態を持って見る)
       5. merge                       → 1 列のスロット
       6. combine                     → Vec<TraceEvent>、DiagInfo
       7. セマンティック ID と display_status を決める
  → KevFile
```

### 6.0 例: スロットから行まで

QNX 8.0 の kev で、CPU 0 の MSG_SENDV の呼び出し(S のスロット 1 つ)が次のように書かれていたとする。

| word | 値 | 意味 |
|---|---|---|
| 0(header) | `0x0000080b` | 構造 S(`_TRACE_STRUCT_S`)、CPU 0、フラグ 0、内部クラス `_TRACE_KER_CALL_C`、内部イベント番号 `__KER_MSG_SENDV`(enter なので種別のビットなし)。ビットの位置は §6.2.1 |
| 1 | `0x00000100` | timestamp_cycles_low32 |
| 2・3 | `0x00000003`・`0x41414141` | payloads |

1. **parse()**: header から内部 ID (`_TRACE_KER_CALL_C`, `__KER_MSG_SENDV`) を取り出し、`semantic_ids()` で (`KER_CALL_ENTER`, `kercall::MSG_SENDV`) にする(enter / exit / int のビットが無いので ENTER)。時刻は TIME で補った 64 ビット。
   → `TraceEvent { class_id: KER_CALL_ENTER, event_id: MSG_SENDV, int_class_id: 2, int_event_id: 11, payloads: [3, 0x41414141], combine_status: Complete, display_status: Display, … }`
2. **decode()**: args.rs の EVENTS から `ev(ClassId::KER_CALL_ENTER, kercall::MSG_SENDV)` の行を引く。payloads が 2 語なので fast の書式 `[h("coid", 0), s("msg[0]", 1, 1)]` を当てはめる。表示名は KER_CALL の規則で `MSG_SENDV/11`。
   → `DecodedEvent { class_name: "KER_CALL", event_name: "MSG_SENDV/11", args: " coid:0x00000003 msg[0]:\"AAAA\" (0x41414141)" }`
3. **traceprinter 互換出力**: 行の書式に当てはめる。

```
t:0x00000100 CPU:00 KER_CALL        :MSG_SENDV/11       coid:0x00000003 msg[0]:"AAAA" (0x41414141)
```

同じイベントが組(header が CB `0x4000080b`・CC `0x8000080b`・CE `0xc000080b` の 3 スロット、payloads 6 語)で書かれていれば、wide の書式で
` coid:0x00000003 sparts:1 rparts:2 msg[0]:"AAAABBBBCCCC" (0x41414141 0x42424242 0x43434343)` のようになる。

### 6.1 ヘッダ

- `TRACE_<英大文字・数字・_>::` をキーとし、次のキーの直前までを値とする。`TRACE_HEADER_END::` で終わる。
- キーの形になっていない `::` は値に含まれる。
- 誤りとする基準は traceprinter に合わせる(§10)。

### 6.2 スロットの読み取り

```rust
struct Slot {
    timestamp_cycles: u64,          // 3 で決まる
    words: [u32; SLOT_WORDS_NUM],   // SLOT_WORDS_NUM = 4
}
```

- syspage の後ろから 16 バイトずつ、リトルエンディアンで読む。
- 16 バイトに満たない末尾は読まず、TrailingBytes を記録する。

### 6.2.1 header の各部と内部 ID

header(スロットの word[0])の各部は、QNX のヘッダ(`sys/trace.h`)のマクロで取り出す。gen_qnx_def.py がマクロの本文からシフトとマスクを取り、def.rs に書く。ロジックは def.rs の名前だけを使う(`event_parser.rs` の `header_bits`)。

| 部分 | ヘッダの定義 | def.rs | QNX 8.0 のビット |
|---|---|---|---|
| 構造 | `_TRACE_GET_STRUCT`。値は `_TRACE_STRUCT_S`・`_CB`・`_CC`・`_CE` | `STRUCT_SHIFT`・`STRUCT_MASK`・`STRUCT_*` | 30〜31 |
| CPU | `_NTO_TRACE_GETCPU` | `CPU_SHIFT`・`CPU_MASK` | 24〜29 |
| フラグ | `_NTO_TRACE_PROCESS_INJECTED` のビットから CPU の手前まで | `FLAGS_SHIFT`・`FLAGS_MASK`・`FLAG_PROCESS_INJECTED` | 16〜23 |
| 内部クラス | `_NTO_TRACE_GETEVENT_C`。値は `_TRACE_*_C`(def.rs にはシフト後の値) | `CLASS_SHIFT`・`CLASS_MASK`・`CLASS_*`・`CLASS_DEFINE_NAMES` | 10〜14 |
| 内部イベント番号 | `_NTO_TRACE_GETEVENT` | `EVENT_SHIFT`・`EVENT_MASK` | 0〜9 |

内部イベント番号の意味は、内部クラスごとに違う。

| 内部クラス | 内部イベント番号 | def.rs |
|---|---|---|
| `_TRACE_CONTROL_C` | `_TRACE_CONTROL_TIME`・`_BUFFER`・`_BUFFER_END`・`_DROPPED` | `CONTROL_*`・`CONTROL_EVENTS` |
| `_TRACE_KER_CALL_C` | コール番号 `__KER_*` に、種別と 64 ビット版のビットを足したもの(下の表) | `KERCALLS`・`KERCALLS_64`・`KERCALL_*`・`BIT64` |
| `_TRACE_INT_C` | `_TRACE_INT_*` | `INT_EVENTS` |
| `_TRACE_PR_TH_C` | `_TRACE_PR_TH_CREATE_P` より小さい番号はスレッドの状態 `STATE_*`。ほかは `_TRACE_PR_TH_*`(仮想スレッドの `*_VT` と、プロセスの生成・破棄・名前) | `THREAD_STATES`・`PR_TH_VTHREAD`・`PR_TH_PROCESS`・`PR_TH_PROCESS_BASE` |
| `_TRACE_SYSTEM_C` | `_NTO_TRACE_SYS_*`。64 ビット版は `_NTO_TRACE_KERCALL64` のビットを足したもの | `SYSTEM_EVENTS`・`SYSTEM_EVENTS_64` |
| `_TRACE_USER_C`・`_TRACE_QUIP_C` | アプリが決める任意の番号 | (表は無い) |
| `_TRACE_COMM_C` | `_NTO_TRACE_COMM_*` | `COMM_EVENTS` |
| `_TRACE_SEC_C` | `_NTO_TRACE_SEC_*` | `SEC_EVENTS` |
| `_TRACE_QVM_C` | `_NTO_TRACE_QVM_*` | `QVM_EVENTS` |
| 上のどれでもない(0・未定義の番号) | — | (traceprinter は UNKNOW の診断の行を出す。§9.3) |

カーネルコールの内部イベント番号:

| ビット | ヘッダの定義 | def.rs | QNX 8.0 の値 |
|---|---|---|---|
| コール番号 | `__KER_*`(`_NTO_TRACE_KERCALLMASK` の範囲) | `KERCALL_NUM_MASK` | 0x7f のマスク |
| 戻り(exit) | `_TRACE_MAX_KER_CALL_NUM` | `KERCALL_EXIT` | 0x80 |
| 割り込まれ(int) | `_TRACE_MAX_KER_CALL_NUM << 1` | `KERCALL_INT` | 0x100 |
| 64 ビット版 | `_NTO_TRACE_KERCALL64` | `BIT64` | 0x200 |

例(QNX 8.0): `__KER_NOP`(0)の enter / exit / int は 0x000 / 0x080 / 0x100、`__KER_MSG_SENDV`(11)は 0x00b / 0x08b / 0x10b、その 64 ビット版は 0x20b / 0x28b / 0x30b。
exit と int の両方のビットが立ったものは、ヘッダに定義が無い(セマンティック ID のクラスが UNKNOWN になる)。

CONTROL の payloads の位置(TIME の上位・下位、BUFFER の通番・スロット数、DROPPED の数)はヘッダに定義が無く、TableOfEvents の引数名から取る(def.rs の `*_WORD`)。BUFFER_END の payloads は `_TRACECOOKIE`(def.rs の `BUFFER_COOKIE`)。

### 6.3 reconstruct_timestamp

CPU 別スロット列ごとに、ファイルの順に見る。

- CONTROL の TIME(S のスロット)が来たら、その payloads の上位の語(TableOfEvents の `msb`)を上位 32 ビットとする。
- 各スロットの `timestamp_cycles = (上位 32 ビット << 32) | timestamp_cycles_low32`。
- TIME が無いのに timestamp_cycles_low32 が半周(`0x80000000`)より大きく減ったときは、上位を進めずに WrapWithoutTime を記録する。traceprinter と同じ並びにするため。

### 6.4 CONTROL の点検

ファイルの順に、CPU ごとの状態(開いているトレースバッファ、前の通番、外のスロット)を持って見る。CONTROL のスロットは S のものだけを数える。

- BUFFER: 前のトレースバッファが閉じていなければ MissingBufferEnd。通番の連続を確かめる(SeqGap)。直前の同じ CPU のスロットが TIME なら、それをトレースバッファの先頭として数える。
- トレースバッファの中: スロットを数える。別の CPU のスロットが来たら Interleaved。
- BUFFER_END: cookie(CookieMismatch)と、スロット数(CountMismatch)を確かめて閉じる。
- DROPPED: Dropped。
- トレースバッファの外のスロット: 続いた範囲ごとに OutsideBuffer。
- 列の終わりでトレースバッファが閉じていなければ MissingBufferEnd。

### 6.5 merge

- 各 CPU 別スロット列の先頭のうち、`(timestamp_cycles, CPU 番号)` が最小のものを順に取り出す。
- CPU の中では並べ替えない(時刻が戻ってもファイルの順のまま)。
- 取り出した順の 1 から数えた番号を、スロットの通し番号とする(ORPHAN 行の番号)。

### 6.6 combine

merge した順にスロットを処理し、項目を待ち行列に入れて、先頭から順に TraceEvent にする。

| 規則 | 内容 |
|---|---|
| 組の鍵 | (CPU, 内部クラス, 内部イベント番号, timestamp_cycles_low32)。鍵の違う組は同時に組み立て中になれる |
| S | 項目として待ち行列に入れる。組には影響しない |
| CB | 同じ鍵の組が組み立て中なら、それを Truncated にする。新しい組を始める |
| CC / CE | 同じ鍵の組につなぐ。CE なら閉じる(Complete)。組が無ければ Orphan |
| 窓 | 先頭の項目が組み立て中のまま、後ろに 1022 項目たまったら、その組を Truncated にする。項目は全 CPU の merge の順で数える |
| 列の終わり | 組み立て中の組を Truncated にする |
| 出力の順 | 項目の先頭スロットの順(組の後ろに来た S は、組の後に出る) |
| header が 0 のスロット | 内部クラス 0・イベント 0 の S として数え、Silent にする |

窓の大きさ 1022 はヘッダに定義が無く、traceprinter の振る舞いから決めた値である。版の定義(args.rs の `COMBINE_WINDOW`)に置く。

### 6.7 セマンティック ID と display_status

内部 ID → セマンティック ID は、版の def.rs の表で引く(`event_parser.rs` の `semantic_ids`)。逆向きは `internal_ids`。

| 内部クラス | 手順 |
|---|---|
| `_TRACE_KER_CALL_C` | 1. 種別のビット(exit・int)でクラスを決める: どちらも無ければ `KER_CALL_ENTER`、exit なら `KER_CALL_EXIT`、int なら `KER_CALL_INT`、両方なら UNKNOWN。<br>2. 64 ビットのビットがあれば `KERCALLS_64`、無ければ `KERCALLS` で、コール番号(`KERCALL_NUM_MASK` の範囲)を引く。無ければ event_id は UNKNOWN |
| `_TRACE_PR_TH_C` | `PR_TH_PROCESS` にあれば `PROCESS`。無ければ `THREAD` で、`THREAD_STATES`・`PR_TH_VTHREAD` の順に引く。どちらにも無ければ event_id は UNKNOWN |
| `_TRACE_USER_C`・`_TRACE_QUIP_C` | `USER`・`QUIP`。event_id は内部イベント番号そのもの |
| CONTROL・INT・SYSTEM・COMM・SEC・QVM | クラスの表(SYSTEM は 64 ビット版の表も)で引く。無ければ event_id は UNKNOWN |
| それ以外 | class_id も event_id も UNKNOWN |

- 内部クラスとセマンティックなクラスは 1 対 1 ではない(KER_CALL は 3 つ、PR_TH は THREAD と PROCESS に分かれる)。
- display_status は、traceprinter がイベントの行または診断の行を出すかと、定義の有無で決める(§5.3。イベントの行を出すかは event_decoder の `format` と同じ規則。§7.1.1)。

---

## 7. decode()

### 7.1 書式

- 文字列は traceprinter `-n` と同じ書式にする。
  - `class_name`: 表示クラス名(`KER_CALL`、`KER_EXIT`、`THREAD` など)
  - `event_name`: 表示イベント名(`MSG_SENDV/11`、`THRUNNING` など)
  - `args`: 名前より後ろの部分(先頭の区切りの空白を含む)
- fast / wide の書式は、イベントの構造で選ぶ(traceprinter の振る舞い)。
  - payloads が 2 語以下(S と、CB だけで途切れた組)→ fast の書式
  - それ以外 → wide の書式
- 引数の名前は traceprinter の表示(QNX の名前)のまま。
- SYSTEM の SLOG の payloads が slog2 のレコードなら、表示イベント名は `SLOG2`、args はレコードの文字列(§7.4)。
- traceprinter が行を出さないもの(Silent、Unknown、Orphan)は、名前が決まればその名前、決まらなければ `UNKNOWN(内部クラス)` と内部イベント番号を返し、args は payloads を 16 進で並べる。

### 7.1.1 1 イベントの行を作る手順(event_decoder.rs の `format`)

1. 内部 ID をセマンティック ID にし、args.rs の EVENTS の行を引く。無ければそのクラスの `any` の行(`max_code` があればその番号まで)。どちらも無ければ行を出さない。
2. 表示クラス名と表示イベント名を決める(下の表)。表示クラス名が無ければ行を出さない。
3. SYSTEM の SLOG で payloads が slog2 のレコードなら、名前 `SLOG2` とレコードの文字列(§7.4)。
4. 行が `intcall` なら、表示クラス名 `INT_CALL`・引数なし。
5. payloads の語数が行の `min_words` より少なければ、行を出さない。
6. 書式を選ぶ: 2 語より多ければ wide、それ以外は fast。fast が無ければ `FAST_FALLBACK_WIDE`(KER_CALL は false、ほかは true)で wide を使うか決める。書式が無い・silent なら行を出さない。
7. KER_CALL_EXIT で payloads の最初の語が `0xffffffff` なら、失敗した exit の共通の書式(`KERCALL_FAILURE`)。それ以外は選んだ書式を当てはめる。

| 表示名の規則 | 表示クラス名 | 表示イベント名 |
|---|---|---|
| カーネルコール | `CLASS_DISPLAY`(KER_CALL・KER_EXIT・KER_INT) | `コール名/番号`(`kercall_name`)。コール名は def.rs の名前で、`KERCALL_NAMES`(exit だけは `KERCALL_NAMES_EXIT` も)で読み替える。def.rs に無い番号は `KER_番号` |
| INT_CALL の形(`.intcall()`) | `INT_CALL` | `KER_コール名/番号`(`intcall_name`。読み替えなし) |
| スレッドの状態 | `THREAD` | `TH` + 状態の名前(`thread_name`) |
| `.name("…")` のある行 | `CLASS_DISPLAY` | その名前 |
| `.name_by_code(f)` のある行(USER・QUIP・SEC の any) | `CLASS_DISPLAY` | `f(内部イベント番号)` |
| それ以外 | `CLASS_DISPLAY` | def.rs の名前(64 ビット版は 64 ビットでない版の名前) |

### 7.1.2 args.rs の構成

| 節 | 定義 | 内容 |
|---|---|---|
| 出力の枠 | `LINE_END`・`BANNER`・`HEADER_TITLE`・`EVENTS_TITLE`・`HEADER_SKIP`・`HEADER_KEY_WIDTH`・`CLASS_WIDTH`・`NAME_WIDTH` | 改行、先頭の行、ヘッダの行と行の幅 |
| 組み立て | `COMBINE_WINDOW` | 窓の大きさ(§6.6) |
| 診断の行 | `DIAG_TRUNC`・`DIAG_ORPHAN`・`DIAG_UNKNOWN`・`DIAG_*` | 診断の行の書式と、その中の名前の表 |
| 表示クラス名 | `CLASS_DISPLAY`・`DISPLAY_INT_CALL` | セマンティックなクラス ID → 表示クラス名 |
| fast の書式が無いとき | `FAST_FALLBACK_WIDE` | wide を使うか |
| 表示イベント名 | `kercall_name`・`intcall_name`・`thread_name`・`KERCALL_NAMES*`・`THREAD_MAX_PRINTED` | 上の表の規則 |
| 値の名前の表 | `TABLE_*` | フラグ・列挙の値の名前(`flags`・`enum_name` が使う) |
| SLOG2 | `SLOG2_*` | §7.4 の定数 |
| 後ろに空行 | `BLANK_AFTER` | 行の後ろに空行を入れるイベント |
| 癖のある書式 | `fn …(a: &Out)` | `custom` の関数(十数イベント) |
| イベントごとの書式 | `EVENTS` | セマンティック ID ごとに 1 行(全 ID) |

EVENTS の行の書き方:

```rust
ev(ClassId::KER_CALL_ENTER, kercall::MSG_SENDV)
    .fast(&[h("coid", 0), s("msg[0]", 1, 1)])                                   // S(と CB だけの組)
    .wide(&[h("coid", 0), d("sparts", 1), d("rparts", 2), s("msg[0]", 3, 3)]),  // 組
ev(ClassId::KER_CALL_INT, kercall::MSG_SENDV).intcall(),                        // INT_CALL の形
ev(ClassId::KER_CALL_ENTER, kercall::NOP_64).fast_silent().wide_silent(),       // 行を出さない
any(ClassId::USER).name_by_code(user_event_name).args(&[dump()]),               // 定義の無い番号すべて
```

| 行の部品 | 意味 |
|---|---|
| `ev(クラス, イベント)` / `any(クラス)` | セマンティック ID の行 / そのクラスで定義の無い番号の行 |
| `.args(&[…])` / `.fast(&[…])` / `.wide(&[…])` | fast と wide に同じ書式 / fast だけ / wide だけ |
| `.args_silent()` / `.fast_silent()` / `.wide_silent()` | traceprinter は出力しない(書式の無い行も出力しない) |
| `.name("…")` / `.name_by_code(f)` | 表示名を決める |
| `.intcall()` | 引数の無い INT_CALL の形 |
| `.min_words(n)` | payloads が n 語より少なければ出力しない |
| `.max_code(n)` | `any` の行を内部イベント番号 n までに限る |

| 書式の要素 | 出力 |
|---|---|
| `h(ラベル, 語)` | ` ラベル:0x%08x` |
| `d(ラベル, 語)` / `u(ラベル, 語)` | ` ラベル:` 符号付き / 符号なし 10 進 |
| `q(ラベル, 語)` / `e(ラベル, 語)` | 語と次の語の 64 ビット(16 進 / 符号付き 10 進)。次の語が無ければ `TRUNC:` の表記 |
| `s(ラベル, 語, 語数)` | ` ラベル:"印字可能な部分" (0x.. ...)`(メッセージの先頭) |
| `sz(ラベル, 語)` | ` ラベル:` 語からの文字列(途中に印字できない文字があれば空) |
| `flags(語, 表, 区切り)` | 表のビットが立っていれば、区切りと名前 |
| `dump()` | ` STR:"..."` と ` d0:0x.. d1:0x..`(USER など) |
| `custom(関数)` | 関数が書く。関数は `a.u(i)`・`a.d(i)`・`a.q(i)`・`a.words()`・`a.nul_str(i)`・`a.raw_str(i)`・`a.enum_name(表, i, 既定)` で読み、`out!(a, "…", …)` で書く |

- ラベル付きの要素は、開始の語がイベントに無ければ出力しない(traceprinter の振る舞い)。custom の関数の中では、無い語は 0 として読める。
- 語の番号は payloads の中の位置(0 から)。

### 7.2 欠けた payloads の表し方(`DecodeOptions::missing`)

| 値 | 内容 |
|---|---|
| `Traceprinter`(既定) | traceprinter と同じ(開始の語が無い引数は出さない) |
| `Marked` | 開始の語が無い引数を `label:(truncated)`(Truncated の組)/ `label:(missing)`(それ以外)と書く |

- CLI は `Traceprinter` だけを使う。`Marked` は API からだけ使う。

### 7.3 書式の部品(tp_format.rs)

args.rs はセマンティック ID ごとに、書式の要素の並びを fast と wide について書く。
- 決まった形の引数は要素(`h`・`d`・`u`・`q`・`e`・`s`・`sz`・`flags`・`dump`)で書く。大半のイベントはこれだけで書ける。
- 条件付きの部分や区切りの違う書式など、要素で書けないもの(十数イベント)は `custom(関数)` にし、args.rs の中に普通の Rust の関数で書く。関数は payloads を `a.u(0)`・`a.q(4)`・`a.raw_str(1)` などで読み、`out!(a, ...)` で書く。
- 文字列で書式を表す言語(テンプレート・条件)は持たない。
- 合成データの生成器は、custom の関数を語数を変えて動かし、読む語を調べる(語の足りない kev を作らないため)。

### 7.4 SLOG2

SYSTEM の SLOG の payloads に slog2 のレコードが入っていると、traceprinter は `SLOG2` という名前で、レコードの文字列を出す。規則はすべて traceprinter の振る舞いから決めた(合成データで確かめた)。定数は args.rs の SLOG2 の節、手順は event_decoder.rs の slog2 の節。

```
t:0x00000110 CPU:00 SYSTEM          :SLOG2             FA connect(62)[tid:5]: pid 1234 proc=server 1234 client 00000012345678d0
```

レコード(payloads をリトルエンディアンのバイト列にしたもの。オフセットはその先頭から):

| オフセット | 長さ | 内容 |
|---|---|---|
| 0 | 2 | `0xcfbd`(u16)。違えば SLOG の書式 |
| 2 | 2 | レコード長(u16、バイト) |
| 7 | 1 | 種別: 0 = 文字列、1 = 書式と引数 |
| 20 | | 本文 |

- payloads が 20 バイトより短ければ SLOG の書式(S のスロットの SLOG はいつも SLOG)。64 ビット版の番号は SLOG2 として読まない(行を出さない)。
- ほかのバイト(1〜6 の残り、8〜19)は表示に使わない。
- レコード長が 20 以下、536 より大きい、payloads より長い → 名前だけで、文字列は空。
- 種別が文字列 → 本文のレコード長 − 1 バイトまでを、NUL の手前まで出す。
- 種別が書式と引数 → `FA ` の後ろに、書式を当てはめた文字列を出す。
  - 書式は本文から NUL まで。NUL がレコードに無いか 511 バイトより長ければ `FA ` だけ。
  - 引数は書式の NUL の後ろから、2 バイト揃えで並ぶ。各引数は u16 のヘッダ(ビット 15 = 文字列、下位 15 ビット = データのバイト数)とデータ。
  - 当てはめられない(引数が足りない、型が合わない、扱わない変換)ときは、書式をそのまま出す。余った引数は使わない。
  - 当てはめた文字列は、最初の NUL の手前まで、1023 バイトまで。
- 種別がほか → 文字列は空。
- 文字列の中のバイトはそのまま出す(制御文字・非 ASCII も)。改行は traceprinter 互換出力では CRLF になる(traceprinter がテキストモードで書くため)。

書式の当てはめは C の printf と同じ。traceprinter は Windows の実行ファイルなので `long` は 32 ビット。C と違うところ・決めたところ:

| 項目 | 規則 |
|---|---|
| 変換 | `d i u x X o p c s f F e E g G a A` と `%%`。ほか(`n`・`q`・`Z`・`C`・`S`・`I64` など)は当てはめられない |
| フラグ・幅・精度 | フラグは `-+ #0`。幅・精度は数字だけ(`*`・`$` は当てはめられない)。`.` の後ろに数字が無い、`%` の前にフラグなどがある、末尾の `%` は当てはめられない |
| 数の引数 | データが 1・2・4・8 バイトのもの。d・i では 4 バイトを符号拡張、ほかは 0 で拡張。ほかの長さは当てはめられない(16 バイトは traceprinter が不定の値を出すので再現しない) |
| 長さ修飾子 | 整数は `hh` 8・`h` 16・`l` 32・`ll`・`j`・`z`・`t` 64 ビット、無しと `L` は 32 ビット。浮動小数点では無視 |
| %s | 引数は文字列でも数でもよい。データの最後のバイトが NUL でなければ当てはめられない。最初の NUL まで |
| %ls など | `l`・`ll`・`j`・`z`・`t` の %s・%c はワイド文字(UTF-16)。%ls はデータの先頭から(データの後ろも続けて)、0 か 0xff より大きい文字の手前まで。%lc は下位 16 ビットで、0xff より大きければ空 |
| %c | 数の下位 8 ビット。0 なら文字列はそこで終わる。文字列の引数は当てはめられない |
| %p | フラグ・幅・精度・長さ修飾子が無ければ 0 で埋めた 16 桁の 16 進(小文字)。あれば `%llx` と同じ(`#` で `0x`) |
| %u | 空白のフラグで前に空白を付ける(`+` は付けない) |
| %#o | 値が 0 でなく、桁が 0 で始まらないときだけ前に 0 を付ける(`%#.0o` の 0 は空) |
| 浮動小数点 | 0 で拡張した 64 ビットを double のビット列として読む。NaN には `-` を付けない。`%a` の非正規化数は `0x0.…p-1022` |

---

## 8. セマンティック ID

説明と全 ID の一覧は docs/semantic-id.md。ここには設計上の決まりだけを書く。

```rust
pub struct ClassId(pub u16);
pub struct EventId(pub u16);
// ClassId::UNKNOWN / EventId::UNKNOWN = 0xFFFF(解釈できなかったもの)
```

- semantic_id.rs に定数として定義する。版に依存しない。手で保守する。
- 一度決めた番号は変えない・消さない。追加だけする(版のサポートを切っても消さない)。
- イベントの種類は (class_id, event_id) の組で表す。event_id はクラスごとの番号。
- KER_CALL の enter / exit / int はクラスで区別し、イベント ID は共通。64 ビット版(KER_CALL・SYSTEM)は別のイベント ID。
- 番号が任意のイベント(USER・QUIP)は、内部イベント番号をそのまま event_id にする。
- 対応表に無いものは UNKNOWN。
- 内部 ID ⇔ セマンティック ID の変換は版ごと(`qnx<版>/event_parser.rs` の `semantic_ids`・`internal_ids`)。対応表は def.rs が持つ。
- 表示文字列(`class_name()`・`event_name()`)は traceprinter の表示で、版で変わりうるので版を引数にとる。セマンティック ID の定数名とは別のもの。
- docs/semantic-id.md の一覧は生成部分で、試験(`semantic_id_doc_is_up_to_date`)が古くなっていないかを確かめる。

---

## 9. traceprinter 互換出力

`traceprinter -n -f X.kev -o OUT` が OUT に書く内容と、バイト単位で同じにする。標準出力・標準エラーに出るもの(末尾の集計 `TRACEPARSER: … cnt` など)は再現しない。

### 9.1 全体

```
TRACEPRINTER version 1.02
TRACEPARSER LIBRARY version 1.02
 -- HEADER FILE INFORMATION -- 
<ヘッダの行>
 -- KERNEL EVENTS -- 
<イベントの行・診断の行>
```

- 改行は CRLF。
- ヘッダの行: keywords を kev の順に `{キー:>22}:: {値}` の形で書く。`TRACE_HEADER_BEGIN` と `TRACE_HEADER_END` は書かない。

### 9.2 イベントの行

```
t:0x{timestamp_cycles_low32:08x} CPU:{cpu:02} {class_name:<16}:{event_name:<18}{args}
```

- Silent のイベントは何も書かない。
- 後ろに空行を入れるイベント(BUFFER・BUFFER_END・DROPPED)は、args.rs の `BLANK_AFTER` が持つ。

### 9.3 診断の行

| 行 | 位置 | 内容 |
|---|---|---|
| `TRUNC` | 途切れた組のイベントの行の直前 | orphan を除くイベントの通し番号、先頭スロットの 4 words |
| `ORPHAN` | 1022 イベント前(下記) | merge した順のスロットの通し番号、構造(CC / CE)、スロットの 4 words |
| `UNKNOW` | 定義の無いクラスのイベントの行の直前(途切れた組なら TRUNC の後)。クラス 0・イベント 0 には出さない | header と timestamp_cycles_low32 |

- 定義のあるクラスの定義の無いイベントには、traceprinter は UNKNOW の行も出さない(§5.3 の Unknown)。

- 行の書式は args.rs が持つ。番号と位置は parse() が DiagInfo に入れ、traceprinter.rs は TraceEvent と DiagInfo から行を作る。
- ORPHAN 行の位置: traceprinter はイベントの行を 1022 件ためてから書くので、ORPHAN 行は「それまでに出た orphan を除くイベントの数 − 1022」件目の後(0 件未満なら先頭)に出る。DiagInfo の `display_before` はその次のイベントの events の番号。
- 診断の行の words は、TraceEvent から組み直す(§5.3)。

---

## 10. エラー

| 状況 | 扱い | traceprinter |
|---|---|---|
| `TRACE_HEADER_END::` が無い | エラー | 内部エラー 17 |
| `TRACE_VER_MAJOR` / `TRACE_VER_MINOR` が無い(値は問わない) | エラー | 内部エラー 13 |
| `TRACE_LITTLE_ENDIAN` が無い(`TRACE_BIG_ENDIAN` だけ、またはどちらも無い) | エラー | `TRACE_BIG_ENDIAN` だけなら読む。どちらも無ければ内部エラー 11 |
| ヘッダの値に NUL・印字可能な ASCII 以外のバイトがある | 取り除いて読む | NUL の手前まで、印字可能な ASCII だけを表示する |
| ヘッダの項目(キー::値)が 2048 バイト以上 | エラー | 内部エラー 17 |
| `TRACE_SYSPAGE_LEN` の分のバイトが無い | エラー | 内部エラー 12 |
| `TRACE_SYSPAGE_LEN` が無い・空 | 0 とする | 読む |
| `TRACE_SYS_RELEASE` が無い | QNX 8.0 として読み、警告(log)を出す | 読む |
| `TRACE_SYS_RELEASE` が対応していない版 | エラー | 読む(tracehub は版の定義を選べないため) |
| イベント部の異常(トレースバッファの不整合、組の異常など) | エラーにしない。DiagInfo に記録して続ける | 読む(場合によって内部エラー) |

---

## 11. 定義の作り方

| ファイル | 作り方 | 内容 |
|---|---|---|
| semantic_id.rs | 手で保守(番号は追加だけ) | セマンティック ID |
| qnxNNN/def.rs | tests/kev/tools/gen_qnx_def.py で生成(手で編集しない) | header のビット配置、内部クラス ID、CONTROL のイベント番号と payloads の位置、KER_CALL のビット、クラスごとの (内部イベント番号, 名前, セマンティック ID) |
| qnxNNN/args.rs | AI が作り、traceprinter の出力で確かめる | 出力の枠、診断の行、表示名、書式、出力しない条件。イベントの書式(EVENTS)はセマンティック ID ごとに 1 行で、全 ID を持つ |

- 入力は QNX の公開資料(SDP のヘッダ、TableOfEvents)と traceprinter の出力。
- QNX の配布物(ヘッダ、TableOfEvents)はリポジトリに入れない。生成した `.rs` はリポジトリに入れる。
- 手順は docs/qnx-version-guide.md に書く。

---

## 12. 依存とロギング

- 依存は `log` クレートだけ(ライブラリと CLI)。CLI の引数は自分で解析する。
- ライブラリは直接何も出力しない。イベント部の異常は DiagInfo で返す。
- エラーにも DiagInfo にもならない、知らせたいこと(`TRACE_SYS_RELEASE` が無いので 8.0 として読んだ、など)は `log::warn!` で出す。出力先は使う側が決める(CLI は標準エラーに `warning: …` と書く)。
- エラーと警告の文面は英語にする。

---

## 13. CLI

```
tracehub traceprinter <kev ファイル> <出力ファイル>
```

- traceprinter 互換出力(`-n` 相当)を出力ファイルに書く。
- エラーのときは、理由を標準エラーに書いて終了コード 1 で終わる。

---

## 14. 試験

詳細は docs/testing.md。

| 種類 | 内容 | 実行 |
|---|---|---|
| 単体試験 | ヘッダ・merge・combine・CONTROL の点検・書式・SLOG2・セマンティック ID・定義の網羅・資料の一覧 | `cargo test` |
| 合成データ | 生成器(全イベントの網羅、語が足りないイベント、窓、ORPHAN の位置、SLOG2、4 CPU の一周)と tests/kev/qnx<版>/data の小さな場面を、kevcsv.py で kev にして traceprinter の出力とバイト単位で比べる | `python tests/kev/tools/compare.py [--qnx 800]` |
| 実機の kev | 実機でカーネルコールなどを呼ぶプログラムを動かし、fast と wide の 2 種類を取って同じく比べる。仕上げとして最後に行う | `python tests/kev/tools/compare.py --kev-dir <DIR>` |
| 確認用のイベント一覧 | セマンティック ID ごとの定義・書式・TableOfEvents との違い・比較の結果 | `python tests/kev/tools/event_catalog.py --table <TableOfEvents>` |

- 正解は traceprinter の出力。期待値をリポジトリに持たず、比べるたびに traceprinter を動かす。traceprinter の無い環境では、kev の横に置いた正解(`X.tp.txt`)と比べる。
- 実機で取りにくいイベントは、合成データで確かめる。
- traceprinter の出力と実機の kev は、リポジトリに入れない。
