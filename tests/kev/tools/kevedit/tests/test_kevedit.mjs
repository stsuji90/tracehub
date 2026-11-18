// kevedit.html の画面に依存しない部分(CORE)と kevcsv.py の試験。
//
//   node tests/test_kevedit.mjs [kev のディレクトリ]
//
// 1. 試験の中で作った kev(と、ディレクトリを渡せばその中の *.kev)について、
//    kev → kev、kev → CSV → kev がバイト単位で一致するか(末尾の端数バイトは除く)
// 2. kevcsv.py(python があれば)が kevedit.html と同じ CSV / kev を作るか
// 3. トレースバッファの点検・追加・スロット数の修正、ヘッダ、文字列、数値の読み方
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const html = fs.readFileSync(path.join(root, "kevedit.html"), "utf8");
const between = (a, b) => {
  const i = html.indexOf(a), j = html.indexOf(b, i + a.length);
  if (i < 0 || j < 0) throw new Error(`${a} 〜 ${b} が見つかりません`);
  return html.slice(i + a.length, j);
};
const defs = JSON.parse(between('<script id="defs" type="application/json">', "</script>"));
const core = between("// ==== CORE BEGIN", "// ==== CORE END").replace(/^[^\n]*\n/, ""); // 印の行の残りを除く
const { CONF, makeCore } = new Function(core + "\nreturn { CONF, makeCore };")();
const K = makeCore(defs);

let fail = 0;
const ok = (cond, msg) => { if (!cond) { fail++; console.log("NG", msg); } };
const same = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);
const issues = (slots) => { const a = K.analyzeBuffers(slots); return a.issues.length + a.bufs.filter((b) => b.issues.length).length; };

// ---- 試験用の kev を作る
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kevedit-test-"));
const ev = (cpu, cls, num, stamp, d1, d2, st = 0, flags = 0) => {
  let h = 0;
  for (const [f, v] of [["struct", st], ["cpu", cpu], ["class", cls], ["event", num], ["flags", flags]]) h = K.setF(h, f, v);
  return { on: true, fresh: false, h, stamp, d1, d2 };
};
function sample({ le = true, junk = "", syspage = 0, tail = 0, off = false } = {}) {
  const doc = K.newDoc();
  if (!le) doc.header = doc.header.map((x) => (x.key.startsWith(defs.header.little_endian) ? { ...x, key: defs.header.big_endian + defs.header.postfix } : x));
  if (syspage) {
    doc.header = doc.header.map((x) => (x.key.startsWith(defs.header.syspage_len) ? { ...x, value: String(syspage) } : x));
    for (let o = 0; o < syspage; o += 16) doc.syspage.push({ on: true, bytes: Uint8Array.from({ length: Math.min(16, syspage - o) }, (_, i) => (o + i) & 0xff) });
  }
  doc.slots = doc.slots.filter((s) => !s.fresh);
  for (const cpu of [0, 1]) {
    const r = K.addBuffer(doc.slots); // TIME, BUFFER, BUFFER_END
    for (let i = r.at; i < r.at + 3; i++) doc.slots[i].h = K.setF(doc.slots[i].h, "cpu", cpu);
    const [w1, w2] = K.strToWords("abcdefg", le);
    doc.slots.splice(r.at + 2, 0,                           // BUFFER と BUFFER_END の間に入れる
      ev(cpu, 4, 1, 0x100 + cpu, 1, 2),                    // PR_TH THRUNNING(simple)
      ev(cpu, 5, 12, 0x200 + cpu, 3, 4, 1),                // SYSTEM SLOG(CB)
      ev(cpu, 5, 12, 0x200 + cpu, w1, w2, 3),              // SYSTEM SLOG(CE)
      ev(cpu, 2, 0x28b, 0x300 + cpu, 0xffffffff, 0, 0, 1), // KER_CALL(exit・64bit)、フラグ付き
      ev(cpu, 31, 1023, 0x400 + cpu, 0x5c, 0x7f));         // 未定義のクラス
  }
  if (off) doc.slots[3].on = false;
  K.fixBufferCounts(doc.slots);
  let b = K.buildKev(doc);
  const pre = new TextEncoder().encode(junk);
  const out = new Uint8Array(pre.length + b.length + tail);
  out.set(pre); out.set(b, pre.length);
  return out;
}
const samples = {
  "le.kev": sample(),
  "be.kev": sample({ le: false }),
  "junk.kev": sample({ junk: "xyz" }),
  "syspage.kev": sample({ syspage: 37 }),
  "tail.kev": sample({ tail: 5 }),
};
for (const [n, b] of Object.entries(samples)) fs.writeFileSync(path.join(tmp, n), b);
const dirs = [tmp, ...process.argv.slice(2)];
const files = dirs.flatMap((d) => fs.readdirSync(d).filter((f) => f.endsWith(".kev")).sort().map((f) => path.join(d, f)));

// ---- 1. 往復
for (const f of files) {
  const b = new Uint8Array(fs.readFileSync(f));
  const doc = K.parseKev(b);
  const tailBytes = doc.notes.some(([, m]) => /端数/.test(m)) ? (b.length - K.buildKev(doc).length) : 0;
  const body = b.subarray(0, b.length - tailBytes);
  ok(same(K.buildKev(doc), body), `${f}: kev → kev が一致しない`);
  const csv = K.toCsv(doc);
  const doc2 = K.fromCsv(csv);
  ok(same(K.buildKev(doc2), body), `${f}: kev → CSV → kev が一致しない`);
  ok(K.toCsv(doc2) === csv, `${f}: CSV → CSV が一致しない`);
  console.log(`ok ${path.basename(f)} (${doc.slots.length} スロット)`);
}
ok(issues(K.parseKev(samples["le.kev"]).slots) === 0, "試験用の kev にトレースバッファの注意がある");

// ---- 2. kevcsv.py と同じ結果か
const py = ["python", "python3", "py"].find((c) => spawnSync(c, ["--version"]).status === 0);
if (!py) console.log("python が無いので kevcsv.py の試験は省略");
else {
  const kevcsv = path.join(root, "kevcsv.py");
  for (const f of files) {
    const b = new Uint8Array(fs.readFileSync(f));
    const doc = K.parseKev(b);
    const csvPath = path.join(tmp, "out.csv"), kevPath = path.join(tmp, "out.kev");
    let r = spawnSync(py, [kevcsv, "kev2csv", f, "-o", csvPath]);
    ok(r.status === 0 || r.status === 1, `${f}: kevcsv.py kev2csv が失敗 ${r.stderr}`);
    ok(fs.readFileSync(csvPath, "utf8") === "﻿" + K.toCsv(doc), `${f}: kevcsv.py の CSV が kevedit.html と違う`);
    r = spawnSync(py, [kevcsv, "csv2kev", csvPath, "-o", kevPath]);
    ok(same(new Uint8Array(fs.readFileSync(kevPath)), K.buildKev(doc)), `${f}: kevcsv.py の kev が kevedit.html と違う`);
  }
  console.log("ok kevcsv.py");
}

// ---- 3a. トレースバッファ: 崩したスロット数を見つけて直す
{
  const slots = K.parseKev(samples["le.kev"]).slots;
  const before = K.buildKev({ header: K.newDoc().header, syspage: [], slots });
  const b0 = K.analyzeBuffers(slots).bufs[0];
  slots[b0.row].d2 += 1;
  ok(issues(slots) === 1, "崩したスロット数を見つけない");
  ok(K.fixBufferCounts(slots) === 1 && issues(slots) === 0, "スロット数を直せない");
  ok(same(K.buildKev({ header: K.newDoc().header, syspage: [], slots }), before), "直した結果が元と違う");
}
// ---- 3b. トレースバッファ追加: 末尾(新規の空行より前)に 3 行
{
  const doc = K.newDoc();
  let r = K.addBuffer(doc.slots);
  ok(r.at === 0 && r.cpu === 0 && r.seq === CONF.bufferSeqStart, `追加の位置・CPU・通番 ${JSON.stringify(r)}`);
  ok(doc.slots.length === CONF.newRows + 3 && issues(doc.slots) === 0, "追加した直後に注意がある");
  doc.slots.splice(2, 0, ev(0, 4, 1, 0x200, 1, 2), ev(0, 4, 1, 0x210, 1, 2));
  ok(issues(doc.slots) === 1 && K.fixBufferCounts(doc.slots) === 1 && issues(doc.slots) === 0, "イベントを入れたあとのスロット数の修正");
  r = K.addBuffer(doc.slots);
  ok(r.at === 5 && r.seq === CONF.bufferSeqStart + 1, `2 つ目の追加 ${JSON.stringify(r)}`);
  ok(issues(doc.slots) === 0 && K.analyzeBuffers(doc.slots).bufs.length === 2, "2 つ目の追加の後");
  ok(K.parseKev(K.buildKev(doc)).slots.length === 8, "追加したトレースバッファの kev");
}
// ---- 3c. ヘッダ: 行のキー(区切りを含む)と値をそのままつなぐ
{
  const doc = K.newDoc();
  ok(K.check(doc).length === 0, `新規作成で注意が出る: ${JSON.stringify(K.check(doc))}`);
  ok(K.parseKev(K.buildKev(doc)).slots.length === 0, "新規作成で空行が出力された");
  const text = K.headerText(doc.header);
  ok(same(K.buildKev(doc).subarray(0, text.length), Uint8Array.from(text, (c) => c.charCodeAt(0))), "ヘッダがつないだ文字列と違う");
  doc.header[1].key = doc.header[1].key.replace(/::$/, ":"); // 区切りを崩すと、その項目はキーとして読まれない
  ok(K.check(doc).some(([, m]) => /VER_MAJOR/.test(m)), "区切りを崩した VER_MAJOR に気づかない");
  const d2 = K.parseKev(samples["junk.kev"]);
  ok(d2.header[0].key === "" && d2.header[0].value === "xyz", "キーより前のバイトの行");
}
// ---- 3d. 文字列 ⇔ payload(両方のバイト順)
for (let n = 0; n < 20000; n++) {
  const d1 = (Math.random() * 2 ** 32) >>> 0, d2 = (Math.random() * 2 ** 32) >>> 0, le = n % 2 === 0;
  const [e1, e2] = K.strToWords(K.wordsToStr(d1, d2, le), le);
  if (e1 !== d1 || e2 !== d2) { ok(false, `文字列の往復 ${d1} ${d2} ${le}`); break; }
  const e = K.bytesToWords(K.strToBytes(K.wordToStr(d1, le), CONF.wordBytes), le)[0];
  if (e !== d1) { ok(false, `1 語の文字列の往復 ${d1} ${le}`); break; }
}
ok(K.wordToStr(K.strToWords("abcdefg", true)[0], true) === "abcd", "1 語目の文字列");
ok(K.wordToStr(K.strToWords("abcdefg", false)[1], false) === "efg", "2 語目の文字列(ビッグエンディアン)");
let threw = false;
try { K.strToWords("123456789", true); } catch { threw = true; }
ok(threw, "9 バイトの文字列が通った");
// ---- 3e. ヘッダ語・数値・CSV の誤り
for (let n = 0; n < 20000; n++) {
  const h = (Math.random() * 2 ** 32) >>> 0;
  let g = K.rest(h);
  for (const f of ["struct", "cpu", "class", "event", "flags"]) g = K.setF(g, f, K.getF(h, f));
  if (g !== h) { ok(false, `ヘッダ語の組み直し ${h.toString(16)}`); break; }
}
ok(K.parseU32("-1") === 0xffffffff, "負の 10 進");
ok(K.parseU32("0xF0F0F080") === 0xf0f0f080, "0x 付き 16 進");
ok(K.parseU32("ff", true) === 255, "16 進の欄で 0x 無し");
threw = false;
try { K.parseU32("4294967296"); } catch { threw = true; }
ok(threw, "32 ビットを超える値が通った");
threw = false;
try { K.fromCsv("[slots]\r\non,0x1,abc,0,0\r\n"); } catch (e) { threw = /2 行目/.test(e.message); }
ok(threw, "CSV の誤りに行番号が無い");

fs.rmSync(tmp, { recursive: true, force: true });
console.log(fail ? `NG ${fail} 件` : "すべて ok");
process.exit(fail ? 1 : 0);
