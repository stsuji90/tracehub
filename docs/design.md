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
  Cargo.toml                   ワークスペース
  crates/
    tracehub-kev/              kev に関するすべての処理(ライブラリ)
      src/
        lib.rs                 公開 API
        model.rs               KevFile・TraceEvent・DiagInfo などの型
        ids.rs                 セマンティック ID(版に依存しない。手で保守)
        kev_header.rs          kev のヘッダの読み取り(版に共通)
        kev_syspage.rs         syspage の読み飛ばし(版に共通)
        tp_format.rs           書式を表す小さな言語と、その解釈(版に共通)
        qnx800/                QNX 8.0 に依存する部分
          mod.rs               内部 ID ⇔ セマンティック ID、表示クラス名
          def.rs               内部 ID の定義(生成ファイル)
          args.rs              traceprinter の表示の定義(手で保守)
          event_parser.rs      スロットの読み取りからイベントの列まで(§6)
          event_decoder.rs     TraceEvent → 表示文字列(§7)
          traceprinter.rs      traceprinter 互換出力(§9)
          scenarios.rs         試験用の合成データの生成(§14)
      examples/scenarios.rs    合成データを CSV で書く
    tracehub-cli/              CLI(tracehub コマンド)
  tools/
    gen_qnx_def.py             SDP のヘッダと TableOfEvents から def.rs を生成する
    compare.py                 traceprinter と tracehub の出力を比べる
    kevedit/                   試験用の kev を CSV から作る道具(kevedit.html、kevcsv.py、CSV_FORMAT.md)
  tests/data/                  試験用の合成データ(小さな場面の CSV)
  docs/
    design.md                  この文書
    qnx-version-guide.md       QNX の版の定義をそろえる手順(別の作業者向け)
```

### 版に依存する部分と依存しない部分

| 版に共通(と仮定する) | 版に依存する |
|---|---|
| ヘッダの書式、syspage の位置、スロットの大きさ、トレースバッファの枠、merge と combine の規則、書式の言語(tp_format.rs) | header のビット配置、内部 ID の値、内部 ID → セマンティック ID の対応、書式、traceprinter の表示文字列 |

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

- 文字列は、kev のバイトを 1 バイト 1 文字(latin-1)にしたもの。
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
    pub little_endian: bool,              // スロットのバイト順
    pub keywords: Vec<(String, String)>,  // キー(TRACE_ を含み :: を含まない)と値。kev に書かれた順
}
```

- keywords は kev に書かれた順で持つ。traceprinter 互換出力のヘッダの行は、この順に書く。
- 同じキーが 2 回以上あれば、すべて持つ。`get()` は最後のものを返す。
- バイト順: `TRACE_LITTLE_ENDIAN` があればリトル、無く `TRACE_BIG_ENDIAN` があればビッグ、どちらも無ければリトル。
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
pub enum DisplayStatus { Display, Silent }
```

| combine_status | 意味 | payloads |
|---|---|---|
| `Complete` | S のスロット 1 つのイベント、または CB から CE までそろった組 | S なら 2 語、組なら 2 × スロット数 |
| `Truncated` | CE が来ないまま途切れた組(§6.6) | 集まったスロットの分 |
| `Orphan` | 組み立て中の組が無いのに来た CC / CE のスロット 1 つ | 2 語 |

| display_status | 意味 |
|---|---|
| `Display` | traceprinter が何かを出力するもの(イベントの行、または TRUNC・ORPHAN・UNKNOW の行) |
| `Silent` | traceprinter が何も出力しないもの(header が 0 のスロット、書式で出力しないと決まっているイベント、書式の無いイベントなど) |

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

- syspage の後ろから 16 バイトずつ読む。バイト順はヘッダに従う。
- 16 バイトに満たない末尾は読まず、TrailingBytes を記録する。

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

- 版の定義(def.rs の表)で、(内部クラス, 内部イベント番号) → (class_id, event_id) を引く。無ければ UNKNOWN(§8)。
- display_status は、traceprinter がイベントの行または診断の行を出すかで決める(event_decoder の書式の選択と同じ規則)。

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
- traceprinter が行を出さないもの(Silent、Orphan、定義の無いもの)は、名前が決まればその名前、決まらなければ `UNKNOWN(内部クラス)` と内部イベント番号を返し、args は payloads を 16 進で並べる。

### 7.2 欠けた payloads の表し方(`DecodeOptions::missing`)

| 値 | 内容 |
|---|---|
| `Traceprinter`(既定) | traceprinter と同じ(開始の語が無い引数は出さない) |
| `Marked` | 開始の語が無い引数を `label:(truncated)`(Truncated の組)/ `label:(missing)`(それ以外)と書く |

- CLI は `Traceprinter` だけを使う。`Marked` は API からだけ使う。

### 7.3 書式の言語(tp_format.rs)

args.rs はイベントごとに、書式の要素(`h`・`d`・`u`・`q`・`e`・`s`・`sz`・`t`・`flags`・`dump`)の並びを fast と wide について書く。
テンプレート(`t`)は `{u0}`・`{x0:08}`・`{q0}` などで payloads を参照する。詳しくは tp_format.rs の先頭のコメント。

---

## 8. セマンティック ID

```rust
pub struct ClassId(pub u16);
pub struct EventId(pub u16);
// UNKNOWN = 0xFFFF、EventId::BIT64 = 0x8000(64 ビット版)
```

- ids.rs に定数として定義する。版に依存しない。
- 一度決めた番号は変えない。追加だけする。手で保守する。

### クラス

| class_id | 名前 | 内容 |
|---|---|---|
| 1 | CONTROL | TIME・BUFFER・BUFFER_END・DROPPED |
| 2 | KER_CALL_ENTER | カーネルコールの呼び出し |
| 3 | KER_CALL_EXIT | カーネルコールの戻り |
| 4 | KER_CALL_INT | カーネルコールの割り込まれ |
| 5 | INT | 割り込み |
| 6 | THREAD | スレッドの状態の遷移・生成・破棄、仮想スレッド |
| 7 | PROCESS | プロセスの生成・破棄・名前 |
| 8 | SYSTEM | システムのイベント |
| 9 | USER | ユーザーのイベント |
| 10 | COMM | メッセージ・パルス |
| 11 | QUIP | QUIP |
| 12 | SEC | セキュリティ |
| 13 | QVM | 仮想マシン |
| 0xFFFF | UNKNOWN | 対応表に無い内部クラス(クラス 0 と未定義の番号)、種別の無い KER_CALL |

### イベント

- クラスごとのモジュール(`ids::kercall`・`ids::thread`・`ids::system` など)に、意味(QNX の名前)ごとの番号を定数で置く。カーネルコールの番号などとは別の番号。
- KER_CALL の enter / exit / int はクラスで区別し、イベント ID は共通(`ids::kercall`)。
- 64 ビット版(KER_CALL・SYSTEM)は `EventId::BIT64` を立てた番号(`ids::kercall::MSG_SENDV.bit64()`)。
- 番号が任意のイベント(USER・QUIP)は、内部イベント番号をそのまま event_id にする。
- 対応表に無いものは UNKNOWN(`0xFFFF`)。

### 表示文字列

- `class_name()` / `event_name()` は、traceprinter が表示するクラス名・イベント名を返す。
- 表示名は版で変わりうる(カーネルコールの番号など)ので、版を引数にとる。

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

- 行の書式は args.rs が持つ。番号と位置は parse() が DiagInfo に入れ、traceprinter.rs は TraceEvent と DiagInfo から行を作る。
- ORPHAN 行の位置: traceprinter はイベントの行を 1022 件ためてから書くので、ORPHAN 行は「それまでに出た orphan を除くイベントの数 − 1022」件目の後(0 件未満なら先頭)に出る。DiagInfo の `display_before` はその次のイベントの events の番号。
- 診断の行の words は、TraceEvent から組み直す(§5.3)。

---

## 10. エラー

| 状況 | 扱い | traceprinter |
|---|---|---|
| `TRACE_HEADER_END::` が無い | エラー | 内部エラー 17 |
| `TRACE_VER_MAJOR` / `TRACE_VER_MINOR` が無い(値は問わない) | エラー | 内部エラー 13 |
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
| ids.rs | 手で保守(番号は追加だけ) | セマンティック ID |
| qnxNNN/def.rs | tools/gen_qnx_def.py で生成(手で編集しない) | header のビット配置、内部クラス ID、CONTROL のイベント番号と payloads の位置、KER_CALL のビット、クラスごとの (内部イベント番号, 名前, セマンティック ID) |
| qnxNNN/args.rs | AI が作り、traceprinter の出力で確かめる | 出力の枠、診断の行、表示名、書式、出力しない条件 |

- 入力は QNX の公開資料(SDP のヘッダ、TableOfEvents)と traceprinter の出力。
- QNX の配布物(ヘッダ、TableOfEvents)はリポジトリに入れない。生成した `.rs` はリポジトリに入れる。
- 手順は docs/qnx-version-guide.md に書く。

---

## 12. 依存とロギング

- 依存は `log` クレートだけ(ライブラリと CLI)。CLI の引数は自分で解析する。
- ライブラリは直接何も出力しない。イベント部の異常は DiagInfo で返す。
- エラーにも DiagInfo にもならない、知らせたいこと(`TRACE_SYS_RELEASE` が無いので 8.0 として読んだ、など)は `log::warn!` で出す。出力先は使う側が決める(CLI は標準エラーに `警告: …` と書く)。

---

## 13. CLI

```
tracehub traceprinter <kev ファイル> <出力ファイル>
```

- traceprinter 互換出力(`-n` 相当)を出力ファイルに書く。
- エラーのときは、理由を標準エラーに書いて終了コード 1 で終わる。

---

## 14. 試験

| 種類 | 内容 | 実行 |
|---|---|---|
| 単体試験 | ヘッダ・merge・combine・CONTROL の点検・書式・セマンティック ID・合成データの枠 | `cargo test` |
| 合成データ | 生成器(全イベントの網羅、語が足りないイベント、窓、ORPHAN の位置、4 CPU の一周)と tests/data の小さな場面を、kevcsv.py で kev にして traceprinter の出力とバイト単位で比べる | `python tools/compare.py` |
| 実機の kev | 実機でカーネルコールなどを呼ぶプログラムを動かし、fast と wide の 2 種類を取って同じく比べる。仕上げとして最後に行う | `python tools/compare.py --kev-dir <DIR>` |

- traceprinter の場所は `--traceprinter` か環境変数 `TRACEHUB_TRACEPRINTER`、実機の kev の置き場所は `--kev-dir` か `TRACEHUB_KEV_DIR`。
- 実機で取りにくいイベントは、合成データで確かめる。
- traceprinter の出力と実機の kev は、リポジトリに入れない。
