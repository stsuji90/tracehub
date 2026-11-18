# 試験用の合成データ

kevcsv の CSV(書式は tools/kevedit/CSV_FORMAT.md)。`tools/compare.py` が kev にして、traceprinter の出力と比べる。
大きな場面(全イベントの網羅など)はここに置かず、生成器(`cargo run -p tracehub-kev --example scenarios -- <DIR>`)で作る。

| ファイル | 内容 |
|---|---|
| minimal.csv | 2 CPU・各 1 トレースバッファの最小のトレース(wide モードのヘッダ) |
| combine_edges.csv | 不完全な組(途切れ・orphan)の 10 ケース |
| combine_eof.csv | 組み立て中のままファイルが終わる |
| combine_interleave.csv | 組がトレースバッファの境目をまたぎ、間に別の CPU のトレースバッファが入る |
| combine_keys.csv | 組の鍵(クラス・イベント番号・時刻)と、組を切らないスロット |
| injected.csv | header のフラグ(_NTO_TRACE_PROCESS_INJECTED)付きのイベント |
| reversal.csv | 同じ CPU の中で時刻が戻る(2 CPU) |
| seq_gap.csv | トレースバッファの通番の飛びと DROPPED(2 CPU) |
| unknown_class.csv | 定義の無いクラス(0、11〜31)のイベント |
