//! traceprinter の引数の書式の部品と、その当てはめ(QNX の版に共通)。
//!
//! 版ごとの args.rs は、イベントごとに [`Item`] の並びを書く。ここはそれを payloads に当てはめて
//! 文字列にするだけで、QNX 固有の名前や番号は持たない。
//!
//! # 要素
//! | 関数 | 出力 |
//! |---|---|
//! | `h(label, w)` | ` label:0x%08x`(語 w) |
//! | `d(label, w)` / `u(label, w)` | ` label:` 符号付き / 符号なし 10 進 |
//! | `q(label, w)` / `e(label, w)` | 語 w・w+1 の 64 ビット(16 進 / 符号付き 10 進)。w+1 が無ければ `TRUNC:` 表記 |
//! | `s(label, w, n)` | ` label:"印字可能部分" (0x.. ...)`(語 w から n 語) |
//! | `sz(label, w)` | ` label:` 語 w からの印字可能な文字列(途中に非印字文字があれば空) |
//! | `flags(w, table, sep)` | 表 table のビットが語 w で立っていれば、sep と名前 |
//! | `dump()` | ` STR:"..."` と ` d0:0x.. d1:0x..`(USER など) |
//! | `custom(f)` | 関数 f が書く([`Out`] の読み出しと `out!`)。上の要素で書けない癖のある書式 |
//!
//! label 付きの要素は、開始の語がイベントに無ければ出力しない(traceprinter の振る舞い)。
//! custom の関数の中では、無い語は 0 として読める。

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

use crate::semantic_id::{ClassId, EventId};

// ---------------------------------------------------------------- 書式の定義(版の args.rs が使う)

/// 値の名前の表(ビット名の並びは出力の順)
pub type Table = &'static [(u64, &'static str)];

/// 引数の書式の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    H,
    D,
    U,
    Q,
    E,
    /// メッセージの先頭(語数つき)
    S(u8),
    SZ,
}

/// 書式の要素
#[derive(Debug, Clone, Copy)]
pub enum Item {
    Arg { op: Op, label: &'static str, word: u8 },
    Flags { word: u8, table: Table, sep: &'static str },
    Dump,
    Custom(fn(&Out)),
}

const fn arg(op: Op, label: &'static str, word: u8) -> Item {
    Item::Arg { op, label, word }
}
pub const fn h(label: &'static str, word: u8) -> Item {
    arg(Op::H, label, word)
}
pub const fn d(label: &'static str, word: u8) -> Item {
    arg(Op::D, label, word)
}
pub const fn u(label: &'static str, word: u8) -> Item {
    arg(Op::U, label, word)
}
pub const fn q(label: &'static str, word: u8) -> Item {
    arg(Op::Q, label, word)
}
pub const fn e(label: &'static str, word: u8) -> Item {
    arg(Op::E, label, word)
}
pub const fn s(label: &'static str, word: u8, words: u8) -> Item {
    arg(Op::S(words), label, word)
}
pub const fn sz(label: &'static str, word: u8) -> Item {
    arg(Op::SZ, label, word)
}
pub const fn flags(word: u8, table: Table, sep: &'static str) -> Item {
    Item::Flags { word, table, sep }
}
pub const fn dump() -> Item {
    Item::Dump
}
pub const fn custom(f: fn(&Out)) -> Item {
    Item::Custom(f)
}

/// 書式(fast / wide のどちらか)
#[derive(Debug, Clone, Copy)]
pub enum Spec {
    /// 定義が無い
    None,
    /// traceprinter は出力しない
    Silent,
    Items(&'static [Item]),
}

/// 表示名
#[derive(Debug, Clone, Copy)]
pub enum Name {
    /// 名前の規則で決める
    Rule,
    Fixed(&'static str),
    /// 内部イベント番号から作る(any の行)
    Code(fn(u16) -> String),
}

/// イベント 1 種類の表示の定義。イベントはセマンティック ID で指す。
/// `event = None`(`any`)はそのクラスで定義の無い番号すべて(`max_code` があればその内部イベント番号まで)
#[derive(Debug, Clone, Copy)]
pub struct Ev {
    pub class: ClassId,
    pub event: Option<EventId>,
    /// any の内部イベント番号の上限
    pub max_code: Option<u16>,
    pub name: Name,
    /// S(と CB だけで途切れた組)の書式
    pub fast: Spec,
    /// 組の書式
    pub wide: Spec,
    /// これより payloads の語が少なければ出力しない
    pub min_words: u8,
    /// KER_CALL を引数の無い INT_CALL の形で表示する
    pub intcall: bool,
}

const fn ev_of(class: ClassId, event: Option<EventId>) -> Ev {
    Ev {
        class,
        event,
        max_code: None,
        name: Name::Rule,
        fast: Spec::None,
        wide: Spec::None,
        min_words: 0,
        intcall: false,
    }
}

pub const fn ev(class: ClassId, event: EventId) -> Ev {
    ev_of(class, Some(event))
}

/// そのクラスで定義の無い番号
pub const fn any(class: ClassId) -> Ev {
    ev_of(class, None)
}

impl Ev {
    pub const fn max_code(self, code: u16) -> Ev {
        Ev {
            max_code: Some(code),
            ..self
        }
    }
    pub const fn name(self, name: &'static str) -> Ev {
        Ev {
            name: Name::Fixed(name),
            ..self
        }
    }
    pub const fn name_by_code(self, f: fn(u16) -> String) -> Ev {
        Ev {
            name: Name::Code(f),
            ..self
        }
    }
    pub const fn min_words(self, n: u8) -> Ev {
        Ev { min_words: n, ..self }
    }
    pub const fn intcall(self) -> Ev {
        Ev { intcall: true, ..self }
    }
    /// fast と wide に同じ書式
    pub const fn args(self, items: &'static [Item]) -> Ev {
        Ev {
            fast: Spec::Items(items),
            wide: Spec::Items(items),
            ..self
        }
    }
    pub const fn args_silent(self) -> Ev {
        Ev {
            fast: Spec::Silent,
            wide: Spec::Silent,
            ..self
        }
    }
    pub const fn fast(self, items: &'static [Item]) -> Ev {
        Ev {
            fast: Spec::Items(items),
            ..self
        }
    }
    pub const fn fast_silent(self) -> Ev {
        Ev {
            fast: Spec::Silent,
            ..self
        }
    }
    pub const fn wide(self, items: &'static [Item]) -> Ev {
        Ev {
            wide: Spec::Items(items),
            ..self
        }
    }
    pub const fn wide_silent(self) -> Ev {
        Ev {
            wide: Spec::Silent,
            ..self
        }
    }
}

// ---------------------------------------------------------------- custom の関数が使う出力先

/// custom の関数に渡す、payloads の読み出しと出力先(`out!` で書く)。
/// 文字列は 1 文字 1 バイト(latin-1)
pub struct Out<'a> {
    w: &'a [u32],
    out: RefCell<&'a mut String>,
    /// 生バイトの文字列の続きの位置(`raw_next`)
    next_byte: Cell<usize>,
    /// 読んだ語の記録(試験用の kev を作るため)
    seen: Option<&'a RefCell<Seen>>,
}

#[derive(Debug, Default)]
struct Seen {
    max_read: Option<usize>,
    strings: BTreeSet<usize>,
    tables: BTreeSet<usize>,
    raw: bool,
}

impl Out<'_> {
    /// 書く(`out!` から使う)
    pub fn put(&self, args: fmt::Arguments) {
        let _ = self.out.borrow_mut().write_fmt(args);
    }
    fn note(&self, i: usize, f: impl FnOnce(&mut Seen)) {
        if let Some(s) = self.seen {
            let mut s = s.borrow_mut();
            s.max_read = Some(s.max_read.map_or(i, |m| m.max(i)));
            f(&mut s);
        }
    }
    fn g(&self, i: usize) -> u32 {
        self.note(i, |_| {});
        self.w.get(i).copied().unwrap_or(0)
    }
    /// payloads の語数
    pub fn words(&self) -> usize {
        self.w.len()
    }
    /// 語 i(符号なし)
    pub fn u(&self, i: usize) -> u32 {
        self.g(i)
    }
    /// 語 i(符号付き)
    pub fn d(&self, i: usize) -> i32 {
        self.g(i) as i32
    }
    /// 語 i・i+1 の 64 ビット(i が下位)
    pub fn q(&self, i: usize) -> u64 {
        ((self.g(i + 1) as u64) << 32) | self.g(i) as u64
    }
    /// 語 i からの NUL までの文字列
    pub fn nul_str(&self, i: usize) -> String {
        self.note(i, |s| {
            s.strings.insert(i);
        });
        word_bytes(self.w, i).take_while(|&b| b != 0).map(char::from).collect()
    }
    /// 語 i からの NUL までの生バイト(続きは raw_next)
    pub fn raw_str(&self, i: usize) -> String {
        self.note(i, |s| {
            s.strings.insert(i);
            s.raw = true;
        });
        self.raw_from(i * 4)
    }
    /// 前の raw_str・raw_next の NUL の次からの、NUL までの生バイト
    pub fn raw_next(&self) -> String {
        self.raw_from(self.next_byte.get())
    }
    fn raw_from(&self, start: usize) -> String {
        let mut s = String::new();
        let mut pos = start;
        for b in word_bytes(self.w, 0).skip(start) {
            pos += 1;
            if b == 0 {
                break;
            }
            s.push(b as char);
        }
        self.next_byte.set(pos);
        s
    }
    /// 語 i の値の名前(表に無ければ default)
    pub fn enum_name(&self, table: Table, i: usize, default: &'static str) -> &'static str {
        let v = self.g(i) as u64;
        self.note(i, |s| {
            s.tables.insert(i);
        });
        table.iter().find(|(k, _)| *k == v).map_or(default, |(_, n)| n)
    }
}

/// custom の関数で書く: `out!(a, " pid:{}", a.d(0))`
macro_rules! out {
    ($a:expr, $($t:tt)*) => {
        $a.put(format_args!($($t)*))
    };
}
pub(crate) use out;

// ---------------------------------------------------------------- 当てはめ

/// 欠けた引数の扱い
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// traceprinter と同じ(開始の語が無い引数は出さない)
    Omit,
    /// 出さない代わりに ` label:<mark>` と書く
    Mark(&'static str),
}

/// 要素の並びを payloads に当てはめて文字列にする
pub fn render(items: &[Item], w: &[u32], missing: Missing, out: &mut String) {
    render_seen(items, w, missing, out, None);
}

fn render_seen(items: &[Item], w: &[u32], missing: Missing, out: &mut String, seen: Option<&RefCell<Seen>>) {
    for it in items {
        match *it {
            Item::Custom(f) => {
                let o = Out {
                    w,
                    out: RefCell::new(out),
                    next_byte: Cell::new(0),
                    seen,
                };
                f(&o);
            }
            Item::Flags { word, table, sep } => {
                let v = w.get(word as usize).copied().unwrap_or(0) as u64;
                for (bit, name) in table {
                    if v & bit != 0 {
                        out.push_str(sep);
                        out.push_str(name);
                    }
                }
            }
            Item::Dump => dump_words(w, out),
            Item::Arg { op, label, word } => {
                let word = word as usize;
                if word >= w.len() {
                    if let Missing::Mark(m) = missing {
                        let _ = write!(out, " {label}:{m}");
                    }
                    continue;
                }
                out.push(' ');
                out.push_str(label);
                out.push(':');
                arg_value(op, word, w, out);
            }
        }
    }
}

fn arg_value(op: Op, i: usize, w: &[u32], out: &mut String) {
    let len = w.len();
    let g = |k: usize| w.get(k).copied().unwrap_or(0);
    let g64 = |k: usize| ((g(k + 1) as u64) << 32) | g(k) as u64;
    match op {
        Op::H => {
            let _ = write!(out, "0x{:08x}", g(i));
        }
        Op::D => {
            let _ = write!(out, "{}", g(i) as i32);
        }
        Op::U => {
            let _ = write!(out, "{}", g(i));
        }
        // 64 ビットの引数で上位の語が無いときは TRUNC: 表記(traceprinter の振る舞い)
        Op::Q => {
            if i + 1 >= len {
                let _ = write!(out, "TRUNC:{:x}", g(i));
            } else {
                let _ = write!(out, "0x{:016x}", g64(i));
            }
        }
        Op::E => {
            if i + 1 >= len {
                let _ = write!(out, "TRUNC:{}", g(i) as i32);
            } else {
                let _ = write!(out, "{}", g64(i) as i64);
            }
        }
        // メッセージの先頭: "印字可能部分" (0x.. ...)。括弧の中の語数は残りの語数に縮む
        Op::S(n) => {
            let n = (n as usize).min(len.saturating_sub(i));
            let mut bytes = Vec::with_capacity(4 * n);
            for k in 0..n {
                bytes.extend_from_slice(&g(i + k).to_le_bytes());
            }
            out.push('"');
            let mut txt = Vec::new();
            scan_text(&bytes, &mut txt);
            push_bytes(out, &txt);
            out.push_str("\" (");
            for k in 0..n {
                if k > 0 {
                    out.push(' ');
                }
                let _ = write!(out, "0x{:08x}", g(i + k));
            }
            out.push(')');
        }
        Op::SZ => {
            let mut buf = Vec::new();
            terminated_printable(w, i, &mut buf);
            push_bytes(out, &buf);
        }
    }
}

/// USER・QUIP の汎用の書式: [ STR:"..."], d0:0x.. d1:0x..
fn dump_words(w: &[u32], out: &mut String) {
    let bytes: Vec<u8> = w.iter().flat_map(|x| x.to_le_bytes()).collect();
    let mut txt = Vec::new();
    if scan_text(&bytes, &mut txt) {
        out.push_str(" STR:\"");
        push_bytes(out, &txt);
        out.push('"');
    }
    out.push(',');
    for (i, x) in w.iter().enumerate() {
        let _ = write!(out, " d{i}:0x{x:08x}");
    }
}

/// payloads をそのまま 16 進で並べる(解釈できないイベント)
pub fn hex_words(w: &[u32], out: &mut String) {
    for x in w {
        let _ = write!(out, " 0x{x:08x}");
    }
}

/// バイト列を 1 バイト 1 文字(latin-1)で足す
pub fn push_bytes(out: &mut String, b: &[u8]) {
    out.extend(b.iter().map(|&c| c as char));
}

fn word_bytes(words: &[u32], start: usize) -> impl Iterator<Item = u8> + '_ {
    words.iter().skip(start).flat_map(|w| w.to_le_bytes())
}

fn printable(b: u8) -> bool {
    (0x20..=0x7e).contains(&b)
}

/// 語 start からの印字可能な文字の並び。NUL か末尾で終わるときだけ出し、非印字文字で途切れるときは何も出さない
fn terminated_printable(words: &[u32], start: usize, out: &mut Vec<u8>) {
    let mut buf = Vec::new();
    for b in word_bytes(words, start) {
        if printable(b) {
            buf.push(b);
        } else if b == 0 {
            break;
        } else {
            return;
        }
    }
    out.extend_from_slice(&buf);
}

/// 語単位の文字列の取り出し(traceprinter の振る舞い)。
/// 4 バイトとも印字可能なら足して次の語へ、印字可能 + NUL ならそこまで足して終わり、それ以外はその語を足さずに終わる。
/// 戻り値: NUL で終わった(または末尾まで印字可能だった)か
fn scan_text(bytes: &[u8], out: &mut Vec<u8>) -> bool {
    for chunk in bytes.chunks(4) {
        let n = chunk.iter().take_while(|&&b| printable(b)).count();
        if n == chunk.len() {
            out.extend_from_slice(chunk);
            continue;
        }
        if chunk[n] == 0 {
            out.extend_from_slice(&chunk[..n]);
            return true;
        }
        return false;
    }
    true
}

// ---------------------------------------------------------------- 書式が使う語(試験用の kev を作るため)

/// 書式が payloads のどの語を使うか
#[derive(Debug, Clone, Default)]
pub struct Usage {
    /// label 付きの引数が読む最後の語
    pub max_arg: Option<usize>,
    /// 書式が無条件に読む語数(これより短いと traceprinter はスロットの外を読む)
    pub min_required: usize,
    /// 文字列が始まる語
    pub str_words: BTreeSet<usize>,
    /// 値の名前の表で解釈される語
    pub table_words: BTreeSet<usize>,
    /// NUL までの生バイトの文字列(raw_str)を含む
    pub raw_str: bool,
    /// dump() を含む
    pub dump: bool,
}

/// custom の関数を試しに動かすときの語数の上限
const PROBE_WORDS: usize = 16;

impl Usage {
    pub fn add(&mut self, items: &[Item]) {
        for it in items {
            match *it {
                Item::Arg { op, word, .. } => {
                    let word = word as usize;
                    let last = match op {
                        Op::Q | Op::E => word + 1,
                        Op::S(n) => word + (n as usize).saturating_sub(1),
                        _ => word,
                    };
                    self.max_arg = Some(self.max_arg.map_or(last, |m| m.max(last)));
                    if matches!(op, Op::S(_) | Op::SZ) {
                        self.str_words.insert(word);
                    }
                }
                Item::Flags { word, .. } => {
                    self.min_required = self.min_required.max(word as usize + 1);
                    self.table_words.insert(word as usize);
                }
                Item::Dump => self.dump = true,
                Item::Custom(_) => self.add_custom(it),
            }
        }
    }

    /// custom の関数を語数を変えて動かし、読んだ語を調べる
    fn add_custom(&mut self, it: &Item) {
        let pattern: Vec<u32> = (0..PROBE_WORDS as u32).map(|i| 0xF0F0_F080 + i).collect();
        let probe = |n: usize| {
            let seen = RefCell::new(Seen::default());
            render_seen(
                std::slice::from_ref(it),
                &pattern[..n],
                Missing::Omit,
                &mut String::new(),
                Some(&seen),
            );
            seen.into_inner()
        };
        // それ以上のどの語数 n で動かしても、n 以上の語を読まない最小の語数
        let ok: Vec<bool> = (0..=PROBE_WORDS)
            .map(|n| probe(n).max_read.is_none_or(|m| m < n))
            .collect();
        let need = (0..=PROBE_WORDS)
            .rev()
            .take_while(|&n| ok[n])
            .last()
            .unwrap_or(PROBE_WORDS);
        self.min_required = self.min_required.max(need);
        let full = probe(PROBE_WORDS);
        self.str_words.extend(full.strings);
        self.table_words.extend(full.tables);
        self.raw_str |= full.raw;
    }

    pub fn add_spec(&mut self, s: Spec) {
        if let Spec::Items(i) = s {
            self.add(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(items: &[Item], w: &[u32], missing: Missing) -> String {
        let mut out = String::new();
        render(items, w, missing, &mut out);
        out
    }

    #[test]
    fn args_and_missing() {
        let items = [h("a", 0), d("b", 1), q("c", 2), s("m", 0, 3), sz("n", 2)];
        assert_eq!(
            r(&items, &[0x41414141, 0xffff_ffff, 7], Missing::Omit),
            " a:0x41414141 b:-1 c:TRUNC:7 m:\"AAAA\" (0x41414141 0xffffffff 0x00000007) n:"
        );
        assert_eq!(
            r(&[h("a", 0), h("b", 1)], &[1], Missing::Mark("(missing)")),
            " a:0x00000001 b:(missing)"
        );
        const T: Table = &[(1, "A"), (4, "C")];
        assert_eq!(r(&[flags(0, T, " ")], &[5], Missing::Omit), " A C");
    }

    #[test]
    fn custom_items() {
        fn f(a: &Out) {
            out!(a, "[{}|{}|{:x}]", a.u(0), a.d(1), a.q(2));
            if a.words() >= 5 {
                out!(a, " {}|{}", a.raw_str(4), a.raw_next());
            }
        }
        let w = [
            7,
            0xffff_ffff,
            1,
            2,
            u32::from_le_bytes(*b"ab\0c"),
            u32::from_le_bytes(*b"xyz\0"),
        ];
        assert_eq!(r(&[custom(f)], &w, Missing::Omit), "[7|-1|200000001] ab|cxyz");
        assert_eq!(r(&[custom(f)], &w[..1], Missing::Omit), "[7|0|0]");
        let mut u = Usage::default();
        u.add(&[custom(f)]);
        assert_eq!(u.min_required, 4);
        assert!(u.raw_str);
        assert_eq!(u.str_words, [4].into());
    }
}
