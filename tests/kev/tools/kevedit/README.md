# kevedit

QNX のカーネルトレース(kev ファイル)を、試験用に手で作る・直すための道具一式。ほかのファイルやネット接続には依存しない。

| ファイル | 内容 |
|---|---|
| `kevedit.html` | 閲覧・編集ツール。ブラウザで開くだけで動く(HTML 1 ファイル、版ごとの定義を埋め込み済み) |
| `kevcsv.py` | kev ⇔ CSV の変換と点検(Python 3.8 以降、標準ライブラリだけ) |
| `CSV_FORMAT.md` | CSV の書式の詳細(人やプログラムが CSV を作るための説明) |
| `tests/test_kevedit.mjs` | 試験(Node.js) |

## kevedit.html

- **開く**: 「開く」で kev か CSV を開く(ページにファイルを落としてもよい)。何も開かなければ、最新の版の既定のヘッダと空の 100 行から始まる。
- **版**: 名前(クラス・イベント・構造)とヘッダ語の欄は、ヘッダの `TRACE_SYS_RELEASE` の版の定義で付ける(定義の無い版は最新の版で、注意を出す)。「新規」は「版」で選んだ版の既定のヘッダで作る。
- **ヘッダ**: 1 行に「キー(`::` を含む)」と「値」。有効な行をそのままつないだ文字列が kev のヘッダになる。
- **スロット**: 1 行 = 1 スロット(16 バイト)。
  - header・stamp・payload の 4 語(16 進)と、構造・CPU・フラグ・クラス・イベント・payload の文字列と 10 進は連動する。
  - 行ごとに有効・無効を切り替えられる。挿入・複製・削除もできる。
- **保存**: 「kev を保存」「CSV を保存」。

## kevcsv.py

```
python kevcsv.py kev2csv IN.kev [-o OUT.csv]
python kevcsv.py csv2kev IN.csv [-o OUT.kev]
python kevcsv.py check   IN.kev | IN.csv
```

- 注意(ヘッダの点検)は標準エラーに出す。誤りがあれば終了コードは 1。
- トレースバッファの枠は点検しない(tracehub か traceprinter で確かめる)。
- 同じ入力から、kevedit.html と同じ CSV / kev を作る。

## 試験

```
node tests/test_kevedit.mjs [kev のディレクトリ]
```

次のことを確かめる。

- 試験の中で作った kev(と、指定したディレクトリの `*.kev`)が、kev → kev、kev → CSV → kev でバイト単位で元に戻るか
- kevcsv.py と kevedit.html の結果が同じか(python があるとき)
- 版の選び方、ヘッダ、文字列、数値の読み方

## 定義を変えるとき

版ごとの定義(ビット配置・構造・既定のヘッダ・クラスとイベントの名前)は `tests/kev/qnx<版>/kevedit_defs.json` にあり、
`kevedit.html` の `<script id="defs">` と `kevcsv.py` の `VERSIONS` に埋め込んである。どれも生成ファイルなので手で直さない。

- 版の定義を作り直す・版を足す: `python tests/kev/tools/gen_qnx_def.py --sdp <SDP のヘッダ> --table <TableOfEvents>`(def.rs と一緒に作り、埋め込む)
- 版のサポートを切る: `tests/kev/qnx<版>/` を消してから `python tests/kev/tools/gen_qnx_def.py --embed-only`
