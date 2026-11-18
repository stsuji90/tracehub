# kevedit

QNX のカーネルトレース(kev ファイル)を、試験用に手で作る・直すための道具一式。ほかのファイルやネット接続には依存しない。

| ファイル | 内容 |
|---|---|
| `kevedit.html` | 編集ツール。ブラウザで開くだけで動く(HTML 1 ファイル、定義を埋め込み済み) |
| `kevcsv.py` | kev ⇔ CSV の変換と点検(Python 3.8 以降、標準ライブラリだけ) |
| `CSV_FORMAT.md` | CSV の書式の詳細(人やプログラムが CSV を作るための説明) |
| `tests/test_kevedit.mjs` | 試験(Node.js) |

## kevedit.html

- **開く**: 「開く」で kev か CSV を開く(ページにファイルを落としてもよい)。何も開かなければ、既定のヘッダと空の 100 行から始まる。
- **ヘッダ**: 1 行に「キー(`::` を含む)」と「値」。有効な行をそのままつないだ文字列が kev のヘッダになる。
- **スロット**: 1 行 = 1 スロット(16 バイト)。
  - header・stamp・payload の 4 語(16 進)と、構造・CPU・フラグ・クラス・イベント・payload の文字列と 10 進は連動する。
  - 行ごとに有効・無効を切り替えられる。挿入・複製・削除もできる。
- **トレースバッファ**:
  - 「トレースバッファ追加」で、末尾に TIME・BUFFER・BUFFER_END を足す。
  - 「バッファ内スロット数修正」で、BUFFER のスロット数を実際の数にする。
  - 点検の一覧(食い違い・通番の飛びなど)も表示する。
- **保存**: 「kev を保存」「CSV を保存」。

## kevcsv.py

```
python kevcsv.py kev2csv IN.kev [-o OUT.csv]
python kevcsv.py csv2kev IN.csv [-o OUT.kev]
python kevcsv.py check   IN.kev | IN.csv
```

- 注意は標準エラーに出す。誤りがあれば終了コードは 1。
- 同じ入力から、kevedit.html と同じ CSV / kev を作る。

## 試験

```
node tests/test_kevedit.mjs [kev のディレクトリ]
```

次のことを確かめる。

- 試験の中で作った kev(と、指定したディレクトリの `*.kev`)が、kev → kev、kev → CSV → kev でバイト単位で元に戻るか
- kevcsv.py と kevedit.html の結果が同じか(python があるとき)
- トレースバッファの補助、ヘッダ、文字列、数値の読み方

## 定義を変えるとき

クラス・イベント番号の名前やビット配置は、QNX 8.0 の値を次の 2 か所に同じ内容で埋め込んである。変えるときは両方を同じに直す。

- `kevedit.html` の `<script id="defs">`
- `kevcsv.py` の `DEFS`
