# 試験用の合成データ

kevcsv の CSV(書式は tests/kev/tools/kevedit/CSV_FORMAT.md)。`tests/kev/tools/compare.py` が kev にして、traceprinter の出力と比べる。
大きな場面(全イベントの網羅など)はここに置かず、生成器(`cargo run -p tracehub-kev --example test_support -- scenarios <DIR>`)で作る。

| ファイル | 内容 |
|---|---|
| minimal.csv | 2 CPU・各 1 トレースバッファの最小のトレース(wide モードのヘッダ) |
| combine_edges.csv | 不完全な組(途切れ・orphan)の 10 ケース |
| combine_eof.csv | 組み立て中のままファイルが終わる |
| combine_interleave.csv | 組がトレースバッファの境目をまたぎ、間に別の CPU のトレースバッファが入る |
| combine_keys.csv | 組の鍵(クラス・イベント番号・時刻)と、組を切らないスロット |
| flags_diag.csv | header のフラグ付きの診断の行(定義の無いクラスの S と組、TRUNC、ORPHAN) |
| injected.csv | header のフラグ(_NTO_TRACE_PROCESS_INJECTED)付きのイベント |
| reversal.csv | 同じ CPU の中で時刻が戻る(2 CPU) |
| seq_gap.csv | トレースバッファの通番の飛びと DROPPED(2 CPU) |
| unknown_class.csv | 定義の無いクラス(0、11〜31)のイベント |
| unknown_event.csv | 定義のあるクラスの定義の無いイベント(SYSTEM 0x20・0x220 など。traceprinter は何も出さない) |
