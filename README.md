# tracehub

いろいろな形式のトレース情報を読み、分析しやすい形式に変換して出力するプログラム。

第 1 段階では QNX のカーネルトレース(kev ファイル、QNX 8.0)を読み、traceprinter と同じテキストを出力する。

## 使い方

```
cargo build --release
target/release/tracehub traceprinter <kev ファイル> <出力ファイル>
```

出力は `traceprinter -n -f <kev> -o <出力ファイル>` と同じ内容になる。

## ライブラリ(tracehub-format-kev)

```rust
let kev = tracehub_format_kev::parse(&std::fs::read("trace.kev")?)?;
for ev in &kev.events {
    let d = tracehub_format_kev::decode(kev.header.version, ev, &Default::default());
    println!("{} {} {}{}", ev.timestamp_cycles, d.class_name, d.event_name, d.args);
}
for d in &kev.diag {
    println!("{:?}", d); // 組の異常、トレースバッファの不整合
}
```

- 依存クレートは `log` だけ(警告を `log::warn!` で出す)。
- イベントは QNX の版に依存しないセマンティック ID(`ClassId`・`EventId`)を持つ。

## 文書

| 文書 | 内容 |
|---|---|
| [docs/design.md](docs/design.md) | 設計書(用語、データモデル、処理、traceprinter 互換出力、試験) |
| [docs/qnx-version-guide.md](docs/qnx-version-guide.md) | QNX の版の定義をそろえる手順 |
| [tests/kev/tools/kevedit/CSV_FORMAT.md](tests/kev/tools/kevedit/CSV_FORMAT.md) | 試験用の kev を作る CSV の書式 |

## 試験

```
cargo test --workspace
python tests/kev/tools/compare.py --traceprinter <traceprinter の場所> [--kev-dir <実機の kev のディレクトリ>]
```

`compare.py` は合成データ(と実機の kev)について、traceprinter と tracehub の出力をバイト単位で比べる。

## QNX の資料

`crates/tracehub-format-kev/src/qnx800/def.rs` は QNX SDP のヘッダから `tests/kev/tools/gen_qnx_def.py` で生成したもの。
QNX の配布物(ヘッダ、TableOfEvents)はリポジトリに含めない。生成し直すときは `inputs/` に置いて使う(手順は docs/qnx-version-guide.md)。
