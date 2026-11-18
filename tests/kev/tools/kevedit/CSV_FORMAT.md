# kev の CSV 書式

QNX のカーネルトレース(kev ファイル)を、表計算ソフトやプログラムで作れるように CSV で表したもの。
`kevedit.html`(画面で編集)と `kevcsv.py`(コマンドで変換)は、どちらもこの書式で読み書きする。

この文書は、CSV を自動で作る人やプログラム(AI を含む)が、これだけを読めば正しい kev を作れることを目標にしている。
定義の値は QNX 8.0 のもの。

---

## 1. kev ファイルの構成

```
+---------------------------+
| ヘッダ(ASCII の文字列)     |  TRACE_HEADER_BEGIN:: … TRACE_HEADER_END::
+---------------------------+
| syspage(バイナリ)         |  TRACE_SYSPAGE_LEN バイト(0 でよい)
+---------------------------+
| スロット(16 バイト)× N   |  u32 × 4 = header, stamp, payload0, payload1
+---------------------------+
```

CSV では、この 3 つの部分を 3 つのセクションで表す。

- CSV の 1 行が、ヘッダの 1 項目、syspage の 16 バイト、またはスロット 1 つに当たる。
- 有効(`on`)な行を上から順に並べたものが、そのまま kev になる。

---

## 2. CSV 全体の決まり

| 項目 | 決まり |
|---|---|
| 文字コード | UTF-8。先頭の BOM はあってもなくてもよい(書き出しでは付ける) |
| 改行 | CRLF でも LF でもよい(書き出しは CRLF) |
| 区切り | コンマ。コンマ・`"`・改行を含むセル、および前後に空白のあるセルは `"` で囲み、中の `"` は `""` にする |
| 空の行 | すべてのセルが空の行は読み飛ばす |
| 1 列目 | 行の種類。下の表 |

1 列目の値:

| 値 | 意味 |
|---|---|
| `on` | 有効な行。kev に出力する |
| `off` | 無効な行。kev に出力しない(コメントアウトの代わり。大文字小文字は問わない) |
| `[header]` | ここからヘッダのセクション |
| `[syspage]` | ここから syspage のセクション |
| `[slots]` | ここからスロットのセクション |

セクションの扱い:

- セクションの行の 2 列目以降は、列の見出し。読み込みでは使わない。
- 最初のセクションの行より前にデータの行があると、誤りになる。
- 書き出しの順は `[header]` → `[syspage]` → `[slots]`。読み込みでは順を問わない。
- 同じセクションが 2 回以上現れたら、行は現れた順にそのセクションに足される。

---

## 3. `[header]` セクション

```
[header],key,value
on,TRACE_HEADER_BEGIN::,
on,TRACE_VER_MAJOR::,2
```

| 列 | 内容 |
|---|---|
| 2 列目 `key` | キー。**区切りの `::` まで含めて書く**(例: `TRACE_VER_MAJOR::`) |
| 3 列目 `value` | 値(空でもよい) |

### 出力のされ方

- kev のヘッダは、**有効な行の `key` と `value` を上から順にそのままつないだ文字列**になる。区切りや改行は自動では入らない。
- 前後の空白もそのまま文字として出力する。
- 使える文字は latin-1(0〜255)だけ。それ以外の文字があると kev を書けない。

例: 上の 2 行は `TRACE_HEADER_BEGIN::TRACE_VER_MAJOR::2` になる。

### 読む側(traceprinter など)の解釈

つないだ文字列を、kev を読む側は次の規則で項目に分ける。

- **キー**は `TRACE_` + 英大文字・数字・`_` が 1 文字以上 + `::` の並び。
- **値**は、キーの直後から次のキーの直前までの文字列。
- `TRACE_HEADER_END::` の直後から syspage が始まる。
- キーの形になっていない文字列(例: `TRACE_::`、`TRACE_ver::`、`TRACE_X:`)は、直前の項目の値の一部として読まれる。
- 同じキーが 2 つ以上あると、最後のものが使われる。

### 項目

| キー | 値の例 | 必要か | 意味 |
|---|---|---|---|
| `TRACE_HEADER_BEGIN::` | (空) | 必須・先頭 | ヘッダの始まり |
| `TRACE_VER_MAJOR::` | `2` | 必須 | kev 形式の版。無いと traceprinter が読まない |
| `TRACE_VER_MINOR::` | `01` | 必須 | 同上 |
| `TRACE_LITTLE_ENDIAN::` | `TRUE` | 推奨 | スロットのバイト順。このキーがあればリトルエンディアン。無く `TRACE_BIG_ENDIAN::` があればビッグエンディアン。どちらも無ければリトルエンディアンとして扱う |
| `TRACE_ENCODING::` | `16 byte events` | 推奨 | これ以外の値は扱わない |
| `TRACE_CYCLES_PER_SEC::` | `19200000` | 推奨 | 時刻(クロック値)の周波数 |
| `TRACE_CPU_NUM::` | `1` | 推奨 | CPU の数。スロットの CPU 番号は 0 〜 CPU_NUM−1 にする |
| `TRACE_SYS_RELEASE::` | `8.0.0` | 推奨 | QNX の版 |
| `TRACE_SYSPAGE_LEN::` | `0` | 必須 | syspage のバイト数。空・無しは 0 と同じ。syspage を使わないなら `0` |
| `TRACE_TRACELOGGER_ARGS::` | `tracelogger -w` | 任意 | 値に `-w` があると wide モード(引数の多い記録)として解釈される。無ければ fast モード |
| `TRACE_HEADER_END::` | (空) | 必須・最後 | ヘッダの終わり |

ほかのキー(`TRACE_FILE_NAME::`、`TRACE_DATE::`、`TRACE_BOOT_DATE::`、`TRACE_SYSNAME::`、`TRACE_NODENAME::`、`TRACE_SYS_VERSION::`、`TRACE_MACHINE::` など)は、書いても書かなくてもよい。

---

## 4. `[syspage]` セクション

```
[syspage],offset,bytes
on,0,0x000102030405060708090a0b0c0d0e0f
```

| 列 | 内容 |
|---|---|
| 2 列目 `offset` | syspage の先頭からの位置(参考。読み込みでは使わない) |
| 3 列目 `bytes` | バイト列の 16 進。`0x` は付けても付けなくてもよい。空白は無視する。書き出しは 1 行 16 バイト |

- 有効な行のバイト列を順につないだものが syspage になる。
- そのバイト数は `TRACE_SYSPAGE_LEN::` の値と同じにする(違うと、それ以降のスロットの位置がずれる)。
- **通常は syspage を使わない**(行を書かず、`TRACE_SYSPAGE_LEN::` を `0` にする)。traceprinter は syspage が無くても読める。

---

## 5. `[slots]` セクション

```
[slots],header,stamp,payload0,payload1,(struct),(cpu),(flags),(class_name),(class),(event_name),(event),(payload0_str),(payload0_dec),(payload1_str),(payload1_dec)
on,0x00000401,0x00001000,0x00000000,0x00001000,S,0,0x00,CONTROL,1,TIME,1,,0,\x00\x10,4096
```

### 列

| 列 | 名前 | 内容 |
|---|---|---|
| 2 | `header` | ヘッダ語(u32)。§6 のビット配置 |
| 3 | `stamp` | 時刻(クロック値)の下位 32 ビット(u32)。§7 |
| 4 | `payload0` | データ語 1(u32)。§8 |
| 5 | `payload1` | データ語 2(u32)。§8 |
| 6 以降 | `(…)` | **参考の列。読み込みでは使わない**。書き出しでは画面と同じ内容を入れる。作るときは空でも、列ごと無くてもよい |

**kev に出力されるのは 2〜5 列の 4 語だけ。** 参考の列を書き換えても kev は変わらない。

参考の列の内容:

| 列 | 内容 |
|---|---|
| `(struct)` | 構造(`S` / `CB` / `CC` / `CE`) |
| `(cpu)` | CPU 番号 |
| `(flags)` | フラグ(16 進) |
| `(class_name)` / `(class)` | クラス名(定義に無ければ空)と番号 |
| `(event_name)` / `(event)` | イベント名(定義に無ければ空)と番号 |
| `(payload0_str)` / `(payload0_dec)` | payload0 のバイト列を文字列にしたもの(§8)と 10 進の値 |
| `(payload1_str)` / `(payload1_dec)` | payload1 について同じもの |

### 数値の書き方(2〜5 列)

| 書き方 | 例 | 値 |
|---|---|---|
| `0x` 付きの 16 進(大文字小文字は問わない) | `0x0000140C` | 5132 |
| 10 進 | `5132` | 5132 |
| 負の 10 進(32 ビットの 2 の補数) | `-1` | 0xffffffff |
| `_` を含む(無視される) | `0x4000_140c` | 0x4000140c |

- 範囲は −2147483648 〜 4294967295。空のセルや範囲外の値は誤りになる。
- `0x` の無い 16 進(例: `ff`)は使えない。10 進として読もうとして、誤りになる。
- 書き出しは `0x` + 8 桁の 16 進(小文字)。

---

## 6. ヘッダ語(`header` 列)のビット配置

```
 31 30 29        24 23        16 15 14      10 9            0
[構造 ][  CPU 番号 ][  フラグ    ][ 0][ クラス  ][ イベント番号 ]
```

| ビット | 名前 | 範囲 | 内容 |
|---|---|---|---|
| 0〜9 | イベント番号 | 0〜1023 | クラスごとの番号(§12 の表) |
| 10〜14 | クラス | 0〜31 | 下の表 |
| 15 | (未使用) | 0 | 0 にする |
| 16〜23 | フラグ | 0〜255 | ビット 16(値 1)= `_NTO_TRACE_PROCESS_INJECTED`。ほかは未定義。通常は 0 |
| 24〜29 | CPU 番号 | 0〜63 | このスロットを記録した CPU |
| 30〜31 | 構造 | 0〜3 | 0 = `S`(1 スロットで 1 イベント)、1 = `CB`(組の始まり)、2 = `CC`(組の途中)、3 = `CE`(組の終わり) |

**計算式**: `header = (構造 << 30) | (CPU << 24) | (フラグ << 16) | (クラス << 10) | イベント番号`

例: `0x4300140c` = 構造 1(CB)、CPU 3、フラグ 0、クラス 5(SYSTEM)、イベント 12(SLOG)

### クラス

| 番号 | 名前 | イベント番号の意味 |
|---|---|---|
| 0 | (なし) | 使わない |
| 1 | CONTROL | 1 = TIME、2 = BUFFER、4 = BUFFER_END、8 = DROPPED(§9) |
| 2 | KER_CALL | カーネルコール。下の「KER_CALL のイベント番号」 |
| 3 | INT | 割り込み |
| 4 | PR_TH | 0〜63 はスレッドの状態の遷移、64 の倍数はプロセスのイベント(§12) |
| 5 | SYSTEM | システムのイベント。ビット 9(0x200)が立つと 64 ビット版 |
| 6 | USER | ユーザーのイベント(番号は任意) |
| 7 | COMM | メッセージ・パルス |
| 8 | QUIP | 番号は任意 |
| 9 | SEC | セキュリティ |
| 10 | QVM | 仮想マシン |
| 11〜31 | (未定義) | 試験で未定義のクラスを作るときに使う |

### KER_CALL のイベント番号

```
  9      8   7    6 ... 0
[64bit][ 種別  ][コール番号]
```

**イベント番号** = `コール番号 | (種別 << 7) | (64 ビット版なら 0x200)`

| 部分 | 値 |
|---|---|
| コール番号 | 0〜127(§12 の KER_CALL の表) |
| 種別 | 0 = enter(呼び出し)、1 = exit(戻り)、2 = int(割り込まれた) |
| 64 ビット版 | 引数が 64 ビットの変種のとき 0x200 |

例: `MSG_SENDV`(11)の exit の 64 ビット版 → `11 | (1 << 7) | 0x200` = `0x28b`

---

## 7. 時刻(`stamp` 列)

- **stamp の意味**: 時刻(クロック値。周波数は `TRACE_CYCLES_PER_SEC::`)の**下位 32 ビット**。
- **上位 32 ビット**: 同じ CPU の直前の CONTROL / TIME スロット(§9)が運ぶ。
- **時刻の並び**: 同じ CPU のスロットは、ファイルの中で時刻の順に並べる(読む側は CPU の中では並べ替えない)。
- **組のスロット**: 組(CB … CE)では、すべてのスロットに同じ stamp を書く。
- **CPU をまたぐ並び**: 読む側は、全 CPU のスロットを (時刻, CPU 番号) の順にマージして表示する。

下位 32 ビットが一周する(大きく減る)ときは、その前に同じ CPU の TIME スロットを入れて、上位を 1 進める。

---

## 8. データ語(`payload0` / `payload1` 列)

### 中身の意味

- payload の意味はイベントごとに違う。QNX のドキュメントにある「イベントごとの引数」(fast モード / wide モード)に従う。
- 引数が 2 語以下なら S(1 スロット)にする。3 語以上なら組(CB, CC…, CE)にする。

### 組(combine)の作り方

- 組のスロットは、構造以外のヘッダ語(CPU・クラス・イベント番号・フラグ)と stamp をすべて同じにする。
- 1 つ目を `CB`、途中を `CC`、最後を `CE` にする。2 スロットなら `CB`, `CE`。
- 引数の語は、`CB` の payload0, payload1, `CC` の payload0, payload1, …, `CE` の payload0, payload1 の順に並べる。
- 余った語は 0 で埋める。
- 組の途中に、別のイベントのスロット(別の CPU・別のクラス/イベントのもの)が挟まっても、読む側は (CPU, クラス, イベント番号, stamp) が同じスロットを集めて組み立てる。

### 文字列の引数

- 文字列は、引数の位置の語から、NUL で終わるバイト列として語に詰める。
- 1 語 = 4 バイトで、バイトの並びはファイルのバイト順に従う。リトルエンディアンなら、1 文字目が語の最下位バイトになる。

例(リトルエンディアン): `"ab"` + NUL → `0x00006261`。`"abcdefg"` + NUL → `0x64636261`, `0x00676665`。

### 参考の列 `(payload0_str)` / `(payload1_str)` の表し方

- その語の 4 バイトを、末尾の NUL を省いて書く。
- 表示できないバイトは `\xHH`、`\` は `\\` で表す。

---

## 9. トレースバッファ(CONTROL の枠)

カーネルは CPU ごとのトレースバッファにスロットを書き、満杯になるとファイルへ出す。
kev では、**1 つのトレースバッファ = 同じ CPU のスロットがファイル上で連続した範囲**で、次の形をとる。

| 順 | スロット | header(CPU 0 の例) | stamp | payload0 | payload1 |
|---|---|---|---|---|---|
| 1 | CONTROL / TIME | `0x00000401` | 時刻の下位 32 ビット | 時刻の上位 32 ビット | 時刻の下位 32 ビット |
| 2 | CONTROL / BUFFER | `0x00000402` | TIME と同じ | 通番(CPU ごとに 1, 2, 3, …) | このトレースバッファのスロット数 |
| … | イベント | | | | |
| (途中) | CONTROL / TIME | `0x00000401` | | 上位 32 ビット | 下位 32 ビット(下位が一周したとき) |
| (途中) | CONTROL / DROPPED | `0x00000408` | | 失われたイベントの数 | 0 |
| 最後 | CONTROL / BUFFER_END | `0x00000404` | 最後のイベントの stamp | `0xe6e6e6e6` | `0xe6e6e6e6` |

CPU n のときは、header の CPU 欄(ビット 24〜29)を n にする(CPU 1 なら `0x01000401` など)。

### 決まり

- **BUFFER のスロット数**は、先頭の TIME から最後の BUFFER_END までのスロット数で、TIME・BUFFER・BUFFER_END 自身も含む。
  - 例: イベントが 3 スロットなら 6。
  - 0 にすると traceprinter が読まない。
  - 実際の数と違うと、traceprinter の表示順が変わることがある。
- **上限**: 1 つのトレースバッファのスロット数は、最大 1011。
- **通番**: CPU ごとに 1 から 1 ずつ増やす。飛ばすと「欠けたトレースバッファがある」と読まれる。
- **連続させる**: トレースバッファの途中(BUFFER から BUFFER_END まで)に、別の CPU のスロットを入れない。traceprinter が読めなくなることがある。
- **出力の順**: CPU の異なるトレースバッファは、どの順に並べてもよい(実際のファイルでは、満杯になった順)。
- **枠の外**: トレースバッファの外(BUFFER より前、BUFFER_END の後)にイベントを置くこともできるが、通常のファイルには無い形。

### 点検

`kevcsv.py check` と `kevedit.html` の「トレースバッファ」欄は、次の点を注意として示す(kev はそのまま書ける)。

- スロット数の食い違い
- 通番の飛び
- BUFFER_END が無い、または BUFFER の無い BUFFER_END
- cookie が `0xe6e6e6e6` ではない
- トレースバッファの途中に別の CPU のスロットがある
- トレースバッファの外のスロット
- 上限(1011)を超えている

---

## 10. 例(そのまま kev にでき、traceprinter で読める)

CPU 1 つ、トレースバッファ 1 つ、イベント 2 つ(スレッドの RUNNING、SLOG の組)。

```
[header],key,value
on,TRACE_HEADER_BEGIN::,
on,TRACE_VER_MAJOR::,2
on,TRACE_VER_MINOR::,01
on,TRACE_LITTLE_ENDIAN::,TRUE
on,TRACE_ENCODING::,16 byte events
on,TRACE_CYCLES_PER_SEC::,19200000
on,TRACE_CPU_NUM::,1
on,TRACE_SYS_RELEASE::,8.0.0
on,TRACE_SYSPAGE_LEN::,0
on,TRACE_HEADER_END::,
[syspage],offset,bytes
[slots],header,stamp,payload0,payload1
on,0x00000401,0x00001000,0x00000000,0x00001000
on,0x00000402,0x00001000,1,6
on,0x00001001,0x00001010,1,6
on,0x4000140c,0x00001020,3,5
on,0xc000140c,0x00001020,0x00006261,0
on,0x00000404,0x00001030,0xe6e6e6e6,0xe6e6e6e6
```

| 行 | 意味 |
|---|---|
| `0x00000401` | TIME: 時刻 = 0x00000000_00001000 |
| `0x00000402` | BUFFER: 通番 1、スロット数 6 |
| `0x00001001` | PR_TH / THRUNNING(S): pid 1、tid 6 |
| `0x4000140c` | SYSTEM / SLOG(CB): opcode 3、severity 5 |
| `0xc000140c` | SYSTEM / SLOG(CE): message `"ab"`(同じ stamp) |
| `0x00000404` | BUFFER_END |

traceprinter の表示(`-n`):
```
t:0x00001010 CPU:00 THREAD          :THRUNNING          pid:1 tid:6
t:0x00001020 CPU:00 SYSTEM          :SLOG               opcode:3 severity:5 message:ab
```

---

## 11. 作り方の手順(チェックリスト)

1. `[header]` を §3 の必須項目で書く。`TRACE_CPU_NUM::` は使う CPU の数にし、`TRACE_SYSPAGE_LEN::` は `0` にする。
2. `[syspage]` は見出しの行だけにする。
3. `[slots]` で、CPU ごとにトレースバッファを作る(TIME → BUFFER → イベント … → BUFFER_END)。
4. イベントの header を §6 の式で作る。クラスとイベント番号は §12 の表から選ぶ。
5. stamp は、同じ CPU の中で増えていくように書く。組のスロットは同じ stamp にする。
6. 各トレースバッファの BUFFER の payload1 に、スロット数(TIME から BUFFER_END まで)を書く。
7. `python kevcsv.py csv2kev X.csv` で kev にし、`python kevcsv.py check X.kev` で注意が出ないことを確かめる。わざと崩す試験データなら、出る注意が狙いどおりかを確かめる。

---

## 12. クラスとイベント番号の一覧(QNX 8.0)

名前は QNX のヘッダの定義名から取った短い名前(`_NTO_TRACE_SYS_SLOG` → `SLOG` など)。

- KER_CALL 以外の表の番号は、そのままイベント番号になる。
- SYSTEM の 64 ビット版は、番号に 0x200 を足す(画面の表示では `名前:64`)。
- PR_TH の 0〜63 はスレッドの状態(`TH` + 状態名)。64 以上はプロセス・仮想スレッドのイベント。

#### CONTROL(クラス 1)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 1 | TIME | 2 | BUFFER | 4 | BUFFER_END | 8 | DROPPED |

#### KER_CALL(クラス 2)

コール番号の一覧(イベント番号の作り方は上の「KER_CALL のイベント番号」)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 0 | NOP | 1 | TRACE_EVENT | 2 | RING0 | 3 | CACHE_CONTROL |
| 4 | SYS_SRANDOM | 5 | MSG_REGISTER_EVENT | 6 | MSG_SEND_PULSEPTR | 7 | SYS_CPUPAGE_GET |
| 8 | SYS_CPUPAGE_SET | 9 | MSG_PAUSE | 10 | MSG_CURRENT | 11 | MSG_SENDV |
| 12 | MSG_SENDVNC | 13 | MSG_ERROR | 14 | MSG_RECEIVEV | 15 | MSG_REPLYV |
| 16 | MSG_READV | 17 | MSG_WRITEV | 19 | MSG_INFO | 20 | MSG_SEND_PULSE |
| 21 | MSG_DELIVER_EVENT | 24 | MSG_RECEIVEPULSEV | 25 | MSG_VERIFY_EVENT | 26 | SIGNAL_KILL |
| 27 | SIGNAL_RETURN | 29 | SIGNAL_ACTION | 30 | SIGNAL_PROCMASK | 31 | SIGNAL_SUSPEND |
| 32 | SIGNAL_WAITINFO | 33 | SIGNAL_KILL_SIGVAL | 35 | CHANNEL_CREATE | 36 | CHANNEL_DESTROY |
| 37 | CHANNEL_CTL | 39 | CONNECT_ATTACH | 40 | CONNECT_DETACH | 41 | CONNECT_SERVER_INFO |
| 42 | CONNECT_CLIENT_INFO | 43 | CONNECT_FLAGS | 44 | CONNECT_DUPFD | 46 | THREAD_CREATE |
| 47 | THREAD_DESTROY | 49 | THREAD_DETACH | 50 | THREAD_JOIN | 51 | THREAD_CANCEL |
| 52 | THREAD_CTL | 53 | THREAD_CTLEXT | 55 | INTERRUPT_ATTACH | 56 | INTERRUPT_QUERY |
| 57 | INTERRUPT_DETACH | 58 | INTERRUPT_WAIT | 59 | INTERRUPT_MASK | 60 | INTERRUPT_UNMASK |
| 61 | INTERRUPT_UNBLOCK | 62 | INTERRUPT_CTL | 65 | CLOCK_TIME | 66 | CLOCK_ADJUST |
| 67 | CLOCK_PERIOD | 68 | CLOCK_ID | 70 | TIMER_CREATE | 71 | TIMER_DESTROY |
| 72 | TIMER_SETTIME | 73 | TIMER_INFO | 74 | TIMER_ALARM | 75 | TIMER_TIMEOUT |
| 76 | TIMER_DELEGATE | 77 | SYNC_RWLOCK | 78 | SYNC_CREATE | 79 | SYNC_DESTROY |
| 80 | SYNC_MUTEX_LOCK | 81 | SYNC_MUTEX_UNLOCK | 82 | SYNC_CONDVAR_WAIT | 83 | SYNC_CONDVAR_SIGNAL |
| 84 | SYNC_SEM_POST | 85 | SYNC_SEM_WAIT | 86 | SYNC_CTL | 87 | SYNC_BARRIER_WAIT |
| 88 | SCHED_GET | 89 | SCHED_SET | 90 | SCHED_YIELD | 91 | SCHED_INFO |
| 92 | SCHED_CTL | 93 | MSG_QUEUE_OPEN | 94 | MSG_QUEUE_SEND | 95 | MSG_QUEUE_RECEIVE |
| 96 | MSG_QUEUE_CLOSE | 97 | MSG_QUEUE_CTL | 98 | PIPE_OPEN | 99 | PIPE_CTL |
| 100 | PIPE_CLOSE | 104 | SYNC_TYPE_DESTROY | 105 | TEST | 106 | SYS_CUSTOM |
| 107 | BAD |  |  |  |  |  |  |

#### INT(クラス 3)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 7 | DELIVER |  |  |  |  |  |  |

#### PR_TH(クラス 4)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 0 | THDEAD | 1 | THRUNNING | 2 | THREADY | 3 | THSTOPPED |
| 4 | THSEND | 5 | THRECEIVE | 6 | THREPLY | 7 | THMQ_SEND |
| 8 | THMQ_RECEIVE | 9 | THWAITPAGE | 10 | THSIGSUSPEND | 11 | THSIGWAITINFO |
| 12 | THNANOSLEEP | 13 | THMUTEX | 14 | THCONDVAR | 15 | THJOIN |
| 16 | THINTR | 17 | THSEM | 18 | THWAITCTX | 19 | THRWLOCK_READ |
| 20 | THRWLOCK_WRITE | 21 | THBARRIER | 22 | THPIPE | 24 | THCREATE |
| 25 | THDESTROY | 26 | THMUON_MUTEX | 27 | THTRACEBUFFER | 28 | THINTR_ATTACH_EV |
| 29 | THTIMER_DELEGATE | 64 | CREATE_P | 87 | CREATE_VT | 88 | DESTROY_VT |
| 128 | DESTROY_P | 192 | CREATE_P_NAME | 256 | DESTROY_P_NAME | 320 | NAME_T |

#### SYSTEM(クラス 5)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 1 | RESERVED | 2 | PATHMGR | 3 | APS_NAME | 4 | APS_BUDGETS |
| 5 | APS_BNKR | 6 | MMAP | 7 | MUNMAP | 8 | MAPNAME |
| 9 | ADDRESS | 10 | FUNC_ENTER | 11 | FUNC_EXIT | 12 | SLOG |
| 13 | DEFRAG_START | 14 | RUNSTATE | 15 | POWER | 16 | IPI |
| 17 | PAGEWAIT | 18 | TIMER | 19 | DEFRAG_END | 20 | PROFILE |
| 21 | MAPNAME_64 | 22 | APS_PSTATS | 23 | APS_OSTATS | 24 | APS_INFO |
| 25 | APS_JOIN | 26 | APS_THREAD | 27 | APS_PROCESS | 28 | SCHED_CONF |
| 29 | IST_ATTACH | 30 | IST_DETACH | 31 | PCTRACE |  |  |

#### USER(クラス 6)

イベント番号の名前の定義は無い(0〜1023 の任意の番号)

#### COMM(クラス 7)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 0 | SMSG | 1 | SPULSE | 2 | RMSG | 3 | RPULSE |
| 4 | SPULSE_EXE | 5 | SPULSE_DIS | 6 | SPULSE_DEA | 7 | SPULSE_UN |
| 8 | SPULSE_QUN | 9 | SIGNAL | 10 | REPLY | 11 | ERROR |

#### QUIP(クラス 8)

イベント番号の名前の定義は無い(0〜1023 の任意の番号)

#### SEC(クラス 9)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 0 | ABLE | 1 | ABLE_LOOKUP | 2 | PATH_ATTACH | 3 | QNET_CONNECT |
| 4 | PERM_LOOKUP | 5 | PERM_TEST | 6 | UNREG_EVENT | 7 | OPEN_EVENT |

#### QVM(クラス 10)

| 番号 | 名前 | 番号 | 名前 | 番号 | 名前 | 番号 | 名前 |
|---|---|---|---|---|---|---|---|
| 0 | GUEST_ENTER | 1 | GUEST_EXIT | 2 | CREATE_VCPU_THREAD | 3 | RAISE_INTR |
| 4 | LOWER_INTR | 5 | TIMER_CREATE | 6 | TIMER_FIRE | 7 | CYCLES |

---

## 13. 道具

| ファイル | 使い方 |
|---|---|
| `kevedit.html` | ブラウザで開く(ネット接続は不要)。kev / CSV を開いて 1 スロットずつ編集し、kev / CSV で保存する。「トレースバッファ追加」(末尾に TIME・BUFFER・BUFFER_END を足す)と「バッファ内スロット数修正」(BUFFER のスロット数を実際の数にする)がある |
| `kevcsv.py` | `python kevcsv.py kev2csv X.kev` / `csv2kev X.csv` / `check X.kev`(または `X.csv`)。標準ライブラリだけで動く |
| `tests/test_kevedit.mjs` | `node tests/test_kevedit.mjs [kev のディレクトリ]`。kevedit.html と kevcsv.py の往復・一致の試験 |

- `kevcsv.py kev2csv` と `kevedit.html` の「CSV を保存」は、同じ kev から同じ CSV を書く。
- CSV からも、同じ kev を書く。
