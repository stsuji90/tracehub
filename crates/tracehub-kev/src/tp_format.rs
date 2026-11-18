//! traceprinter の引数の書式を表す小さな言語と、その解釈(QNX の版に共通)。
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
//! | `t(text)` | テンプレート(下記) |
//! | `flags(w, table, each)` | 表 table のビットが語 w で立っていれば each を出力 |
//! | `dump()` | ` STR:"..."` と ` d0:0x.. d1:0x..`(USER など) |
//!
//! label 付きの要素は、開始の語がイベントに無ければ出力しない(traceprinter の振る舞い)。
//! どの要素にも `.when("条件")` を付けられる(満たさなければ出力しない)。
//!
//! # テンプレート
//! `{u0}` `{d0}` `{x0}`(語 0 の符号なし / 符号付き 10 進 / 16 進)、`{q0}` `{e0}`(64 ビット)、
//! `{x0:08}` `{q0:#08}`(printf の幅指定)、`{u0&0xffff}`(マスク)、
//! `{s2}`(語 2 からの印字可能な範囲)、`{z2}`(NUL まで)、`{r2}` `{rnext}`(NUL までの生バイトと、その続き)、
//! `{z8:6}`(右寄せ幅)、`{names:TABLE:q0}`(ビット名を空白区切り)、`{enum:TABLE:u4:DEFAULT}`(値の名前)、
//! `{{` `}}`(括弧そのもの)。名前のテンプレートでは `{code}` `{num}` `{call}` `{name}` も使える。
//!
//! # 条件
//! `words >= 4`、`u3 != 0xffffffff`、`u4 & 0x10000000`(0 でない)、`u2 & 0x1 == 0`、`A && B`。

use std::collections::HashMap;
use std::fmt::Write as _;

// ---------------------------------------------------------------- 書式の定義(版の args.rs が使う)

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
pub enum ItemKind {
    Arg {
        op: Op,
        label: &'static str,
        word: u8,
    },
    Text(&'static str),
    Flags {
        word: u8,
        table: &'static str,
        each: &'static str,
    },
    Dump,
}

#[derive(Debug, Clone, Copy)]
pub struct Item {
    pub kind: ItemKind,
    /// 出力する条件(空なら常に)
    pub when: &'static str,
}

impl Item {
    pub const fn when(self, cond: &'static str) -> Item {
        Item { when: cond, ..self }
    }
}

const fn arg(op: Op, label: &'static str, word: u8) -> Item {
    Item {
        kind: ItemKind::Arg { op, label, word },
        when: "",
    }
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
pub const fn t(text: &'static str) -> Item {
    Item {
        kind: ItemKind::Text(text),
        when: "",
    }
}
pub const fn flags(word: u8, table: &'static str, each: &'static str) -> Item {
    Item {
        kind: ItemKind::Flags { word, table, each },
        when: "",
    }
}
pub const fn dump() -> Item {
    Item {
        kind: ItemKind::Dump,
        when: "",
    }
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

/// 値の名前の表(ビット名の並びは出力の順)
pub type Table = &'static [(u64, &'static str)];

/// イベント 1 種類の表示の定義。イベントは内部クラス名・内部イベント名(def.rs の名前)で指す。
/// `event = "*"` はそのクラスで定義の無い番号すべて
#[derive(Debug, Clone, Copy)]
pub struct Ev {
    pub class: &'static str,
    pub event: &'static str,
    /// KER_CALL の種別("ENTER" / "EXIT" / "INT")
    pub kind: &'static str,
    /// 64 ビット版
    pub is64: bool,
    /// 表示名のテンプレート(None なら名前の規則で決める)
    pub name: Option<&'static str>,
    /// S(と CB だけで途切れた組)の書式
    pub fast: Spec,
    /// 組の書式
    pub wide: Spec,
    /// これより payloads の語が少なければ出力しない
    pub min_words: u8,
    /// KER_CALL を引数の無い INT_CALL の形で表示する
    pub intcall: bool,
}

pub const fn ev(class: &'static str, event: &'static str) -> Ev {
    Ev {
        class,
        event,
        kind: "",
        is64: false,
        name: None,
        fast: Spec::None,
        wide: Spec::None,
        min_words: 0,
        intcall: false,
    }
}

impl Ev {
    pub const fn kind(self, kind: &'static str) -> Ev {
        Ev { kind, ..self }
    }
    pub const fn is64(self) -> Ev {
        Ev { is64: true, ..self }
    }
    pub const fn name(self, name: &'static str) -> Ev {
        Ev {
            name: Some(name),
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

// ---------------------------------------------------------------- コンパイル済みの書式

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    U,
    D,
    X,
    Q,
    E,
}

#[derive(Debug, Clone, Copy, Default)]
struct Fmt {
    alt: bool,
    zero: bool,
    width: usize,
}

#[derive(Debug, Clone, Copy)]
enum StrKind {
    /// 印字可能な文字が続く範囲
    Printable,
    /// NUL まで
    Nul,
    /// NUL までの生バイト
    Raw,
}

#[derive(Debug, Clone)]
enum Seg {
    Lit(String),
    Val {
        kind: Kind,
        word: usize,
        mask: Option<u64>,
        fmt: Fmt,
    },
    Str {
        kind: StrKind,
        word: Option<usize>,
        width: usize,
    },
    Names {
        table: &'static str,
        kind: Kind,
        word: usize,
    },
    Enum {
        table: &'static str,
        kind: Kind,
        word: usize,
        default: String,
    },
    Var {
        name: String,
        fmt: Fmt,
    },
}

/// コンパイル済みのテンプレート
#[derive(Debug, Clone, Default)]
pub struct Template {
    segs: Vec<Seg>,
}

/// 条件(論理積)
#[derive(Debug, Clone, Default)]
pub struct Cond {
    atoms: Vec<Atom>,
}

#[derive(Debug, Clone, Copy)]
struct Atom {
    words: bool,
    kind: Kind,
    word: usize,
    mask: Option<u64>,
    cmp: Cmp,
    rhs: u64,
}

#[derive(Debug, Clone, Copy)]
enum Cmp {
    Ge,
    Gt,
    Le,
    Lt,
    Eq,
    Ne,
    NonZero,
}

/// コンパイル済みの要素
#[derive(Debug, Clone)]
pub struct CItem {
    kind: CKind,
    when: Option<Cond>,
}

#[derive(Debug, Clone)]
enum CKind {
    Arg {
        op: Op,
        label: &'static str,
        word: usize,
    },
    Text(Template),
    Flags {
        word: usize,
        table: &'static str,
        each: Template,
    },
    Dump,
}

/// 書式をコンパイルする(定義の誤りは panic。定義の試験で見つける)
pub fn compile(items: &[Item]) -> Vec<CItem> {
    items
        .iter()
        .map(|it| CItem {
            when: (!it.when.is_empty()).then(|| Cond::parse(it.when)),
            kind: match it.kind {
                ItemKind::Arg { op, label, word } => CKind::Arg {
                    op,
                    label,
                    word: word as usize,
                },
                ItemKind::Text(t) => CKind::Text(Template::parse(t)),
                ItemKind::Flags { word, table, each } => CKind::Flags {
                    word: word as usize,
                    table,
                    each: Template::parse(each),
                },
                ItemKind::Dump => CKind::Dump,
            },
        })
        .collect()
}

fn parse_kind(c: char) -> Option<Kind> {
    Some(match c {
        'u' => Kind::U,
        'd' => Kind::D,
        'x' => Kind::X,
        'q' => Kind::Q,
        'e' => Kind::E,
        _ => return None,
    })
}

fn parse_ref(s: &str) -> (Kind, usize) {
    let mut ch = s.chars();
    let k = ch
        .next()
        .and_then(parse_kind)
        .unwrap_or_else(|| panic!("invalid value reference: {s}"));
    let w = ch
        .as_str()
        .parse()
        .unwrap_or_else(|_| panic!("invalid word number: {s}"));
    (k, w)
}

fn parse_num(s: &str) -> u64 {
    let s = s.trim();
    match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => s.parse(),
    }
    .unwrap_or_else(|_| panic!("invalid number: {s}"))
}

fn parse_fmt(s: &str) -> Fmt {
    let mut f = Fmt::default();
    let mut rest = s;
    if let Some(r) = rest.strip_prefix('#') {
        f.alt = true;
        rest = r;
    }
    if rest.starts_with('0') && rest.len() > 1 {
        f.zero = true;
        rest = &rest[1..];
    }
    if !rest.is_empty() {
        f.width = rest.parse().unwrap_or_else(|_| panic!("invalid width: {s}"));
    }
    f
}

impl Template {
    pub fn parse(src: &'static str) -> Template {
        let mut segs = Vec::new();
        let mut lit = String::new();
        let mut rest = src;
        while let Some(c) = rest.chars().next() {
            if rest.starts_with("{{") || rest.starts_with("}}") {
                lit.push(c);
                rest = &rest[2..];
            } else if c == '{' {
                let end = rest.find('}').unwrap_or_else(|| panic!("missing closing brace: {src}"));
                if !lit.is_empty() {
                    segs.push(Seg::Lit(std::mem::take(&mut lit)));
                }
                segs.push(Self::parse_seg(&rest[1..end]));
                rest = &rest[end + 1..];
            } else {
                lit.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        if !lit.is_empty() {
            segs.push(Seg::Lit(lit));
        }
        Template { segs }
    }

    fn parse_seg(s: &'static str) -> Seg {
        if let Some(r) = s.strip_prefix("names:") {
            let (t, v) = r.split_once(':').expect("names:TABLE:q0");
            let (kind, word) = parse_ref(v);
            return Seg::Names { table: t, kind, word };
        }
        if let Some(r) = s.strip_prefix("enum:") {
            let p: Vec<&'static str> = r.splitn(3, ':').collect();
            assert_eq!(p.len(), 3, "enum:TABLE:u4:DEFAULT");
            let (kind, word) = parse_ref(p[1]);
            return Seg::Enum {
                table: p[0],
                kind,
                word,
                default: p[2].into(),
            };
        }
        let (body, fmt) = match s.split_once(':') {
            Some((a, f)) => (a, parse_fmt(f)),
            None => (s, Fmt::default()),
        };
        if matches!(body, "code" | "num" | "call" | "name") {
            return Seg::Var { name: body.into(), fmt };
        }
        let first = body.chars().next().expect("empty {}");
        match first {
            's' | 'z' | 'r' => {
                let kind = match first {
                    's' => StrKind::Printable,
                    'z' => StrKind::Nul,
                    _ => StrKind::Raw,
                };
                let rest = &body[1..];
                let word =
                    (rest != "next").then(|| rest.parse().unwrap_or_else(|_| panic!("invalid word number: {body}")));
                Seg::Str {
                    kind,
                    word,
                    width: fmt.width,
                }
            }
            _ => {
                let (r, mask) = match body.split_once('&') {
                    Some((a, m)) => (a.trim(), Some(parse_num(m))),
                    None => (body, None),
                };
                let (kind, word) = parse_ref(r);
                Seg::Val { kind, word, mask, fmt }
            }
        }
    }

    /// 名前のテンプレートを描画する({code} {num} {call} {name})
    pub fn render_vars(&self, vars: &[(&str, Var)], out: &mut String) {
        for s in &self.segs {
            match s {
                Seg::Lit(l) => out.push_str(l),
                Seg::Var { name, fmt } => match vars.iter().find(|(n, _)| n == name).map(|(_, v)| v) {
                    Some(Var::Num(n)) => fmt_num(out, Kind::U, *n, *fmt),
                    Some(Var::Str(s)) => out.push_str(s),
                    None => {}
                },
                _ => {}
            }
        }
    }

    fn render(&self, cx: &mut Ctx, out: &mut String) {
        for s in &self.segs {
            match s {
                Seg::Lit(l) => out.push_str(l),
                Seg::Val { kind, word, mask, fmt } => {
                    let mut v = cx.value(*kind, *word);
                    if let Some(m) = mask {
                        v &= m;
                    }
                    fmt_num(out, *kind, v, *fmt);
                }
                Seg::Str { kind, word, width } => {
                    let start = word.map_or(cx.next_byte, |w| w * 4);
                    let mut buf = Vec::new();
                    let next = match kind {
                        StrKind::Printable => {
                            printable_run(cx.words, start / 4, &mut buf);
                            start
                        }
                        StrKind::Nul => {
                            nul_string(cx.words, start / 4, &mut buf);
                            start
                        }
                        StrKind::Raw => raw_string(cx.words, start, &mut buf),
                    };
                    for _ in buf.len()..*width {
                        out.push(' ');
                    }
                    push_bytes(out, &buf);
                    cx.next_byte = next;
                }
                Seg::Names { table, kind, word } => {
                    let v = cx.value(*kind, *word);
                    let mut first = true;
                    for (bit, name) in cx.table(table) {
                        if v & bit != 0 {
                            if !first {
                                out.push(' ');
                            }
                            first = false;
                            out.push_str(name);
                        }
                    }
                }
                Seg::Enum {
                    table,
                    kind,
                    word,
                    default,
                } => {
                    let v = cx.value(*kind, *word);
                    let name = cx
                        .table(table)
                        .iter()
                        .find(|(k, _)| *k == v)
                        .map_or(default.as_str(), |(_, n)| n);
                    out.push_str(name);
                }
                Seg::Var { .. } => {}
            }
        }
    }
}

/// 名前のテンプレートの変数の値
pub enum Var<'a> {
    Num(u64),
    Str(&'a str),
}

impl Cond {
    pub fn parse(src: &str) -> Cond {
        Cond {
            atoms: src.split("&&").map(|p| Self::parse_atom(p.trim())).collect(),
        }
    }

    fn parse_atom(s: &str) -> Atom {
        let ops = [
            (">=", Cmp::Ge),
            ("<=", Cmp::Le),
            ("==", Cmp::Eq),
            ("!=", Cmp::Ne),
            (">", Cmp::Gt),
            ("<", Cmp::Lt),
        ];
        let (lhs, cmp, rhs) = match ops.iter().find_map(|(t, c)| s.find(t).map(|p| (p, *t, *c))) {
            Some((p, t, c)) => (&s[..p], c, parse_num(&s[p + t.len()..])),
            None => (s, Cmp::NonZero, 0),
        };
        let (l, mask) = match lhs.split_once('&') {
            Some((a, m)) => (a.trim(), Some(parse_num(m))),
            None => (lhs.trim(), None),
        };
        if l == "words" {
            Atom {
                words: true,
                kind: Kind::U,
                word: 0,
                mask,
                cmp,
                rhs,
            }
        } else {
            let (kind, word) = parse_ref(l);
            Atom {
                words: false,
                kind,
                word,
                mask,
                cmp,
                rhs,
            }
        }
    }

    pub fn eval(&self, w: &[u32]) -> bool {
        self.atoms.iter().all(|a| {
            let g = |i: usize| w.get(i).copied().unwrap_or(0) as u64;
            let mut v = if a.words {
                w.len() as u64
            } else if matches!(a.kind, Kind::Q | Kind::E) {
                (g(a.word + 1) << 32) | g(a.word)
            } else {
                g(a.word)
            };
            if let Some(m) = a.mask {
                v &= m;
            }
            match a.cmp {
                Cmp::Ge => v >= a.rhs,
                Cmp::Gt => v > a.rhs,
                Cmp::Le => v <= a.rhs,
                Cmp::Lt => v < a.rhs,
                Cmp::Eq => v == a.rhs,
                Cmp::Ne => v != a.rhs,
                Cmp::NonZero => v != 0,
            }
        })
    }
}

// ---------------------------------------------------------------- 描画

/// 欠けた引数の扱い
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// traceprinter と同じ(開始の語が無い引数は出さない)
    Omit,
    /// 出さない代わりに ` label:<mark>` と書く
    Mark(&'static str),
}

struct Ctx<'a> {
    words: &'a [u32],
    tables: &'a HashMap<&'static str, Table>,
    next_byte: usize,
}

impl Ctx<'_> {
    fn g(&self, i: usize) -> u32 {
        self.words.get(i).copied().unwrap_or(0)
    }
    fn value(&self, kind: Kind, word: usize) -> u64 {
        match kind {
            Kind::Q | Kind::E => ((self.g(word + 1) as u64) << 32) | self.g(word) as u64,
            _ => self.g(word) as u64,
        }
    }
    fn table(&self, name: &str) -> Table {
        self.tables
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("no table named {name}"))
    }
}

/// 要素の並びを payloads に当てはめて文字列にする
pub fn render(items: &[CItem], w: &[u32], tables: &HashMap<&'static str, Table>, missing: Missing, out: &mut String) {
    let mut cx = Ctx {
        words: w,
        tables,
        next_byte: 0,
    };
    for it in items {
        if let Some(c) = &it.when
            && !c.eval(w)
        {
            continue;
        }
        match &it.kind {
            CKind::Text(t) => t.render(&mut cx, out),
            CKind::Flags { word, table, each } => {
                let v = cx.g(*word) as u64;
                for (bit, name) in cx.table(table) {
                    if v & bit != 0 {
                        each.render_vars(&[("name", Var::Str(name))], out);
                    }
                }
            }
            CKind::Dump => dump_words(w, out),
            CKind::Arg { op, label, word } => {
                if *word >= w.len() {
                    if let Missing::Mark(m) = missing {
                        let _ = write!(out, " {label}:{m}");
                    }
                    continue;
                }
                out.push(' ');
                out.push_str(label);
                out.push(':');
                arg_value(*op, *word, w, out);
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

fn pad(out: &mut String, body: &str, fmt: Fmt, neg: bool) {
    let len = body.len() + neg as usize;
    if fmt.width > len {
        let n = fmt.width - len;
        if fmt.zero {
            if neg {
                out.push('-');
            }
            out.extend(std::iter::repeat_n('0', n));
            out.push_str(body);
            return;
        }
        out.extend(std::iter::repeat_n(' ', n));
    }
    if neg {
        out.push('-');
    }
    out.push_str(body);
}

fn fmt_num(out: &mut String, kind: Kind, v: u64, fmt: Fmt) {
    match kind {
        Kind::U => pad(out, &v.to_string(), fmt, false),
        Kind::D => {
            let x = v as u32 as i32 as i64;
            pad(out, &x.unsigned_abs().to_string(), fmt, x < 0)
        }
        Kind::E => {
            let x = v as i64;
            pad(out, &x.unsigned_abs().to_string(), fmt, x < 0)
        }
        Kind::X | Kind::Q => {
            let h = format!("{v:x}");
            if fmt.alt && v != 0 {
                // C の %#0Nx: "0x" を含めて幅 N
                out.push_str("0x");
                pad(
                    out,
                    &h,
                    Fmt {
                        alt: false,
                        zero: fmt.zero,
                        width: fmt.width.saturating_sub(2),
                    },
                    false,
                );
            } else {
                pad(out, &h, fmt, false);
            }
        }
    }
}

fn word_bytes(words: &[u32], start: usize) -> impl Iterator<Item = u8> + '_ {
    words.iter().skip(start).flat_map(|w| w.to_le_bytes())
}

fn printable(b: u8) -> bool {
    (0x20..=0x7e).contains(&b)
}

/// 語 start から印字可能な文字が続く範囲
fn printable_run(words: &[u32], start: usize, out: &mut Vec<u8>) {
    out.extend(word_bytes(words, start).take_while(|&b| printable(b)));
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

/// 語 start から NUL まで
fn nul_string(words: &[u32], start: usize, out: &mut Vec<u8>) {
    out.extend(word_bytes(words, start).take_while(|&b| b != 0));
}

/// バイト位置 start から NUL までの生バイト。戻り値: NUL の次のバイト位置
fn raw_string(words: &[u32], start: usize, out: &mut Vec<u8>) -> usize {
    let mut pos = start;
    for b in word_bytes(words, 0).skip(start) {
        pos += 1;
        if b == 0 {
            return pos;
        }
        out.push(b);
    }
    pos
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
    /// 条件・テンプレートが無条件に読む語数(これより短いと traceprinter はスロットの外を読む)
    pub min_required: usize,
    /// 文字列が始まる語
    pub str_words: std::collections::BTreeSet<usize>,
    /// 値の名前の表で解釈される語
    pub table_words: std::collections::BTreeSet<usize>,
    /// NUL までの生バイトの文字列({rN})を含む
    pub raw_str: bool,
    /// dump() を含む
    pub dump: bool,
}

impl Usage {
    pub fn add(&mut self, items: &[Item]) {
        let upd = |m: &mut Option<usize>, w: usize| *m = Some(m.map_or(w, |x| x.max(w)));
        for it in items {
            if !it.when.is_empty() {
                for a in &Cond::parse(it.when).atoms {
                    if !a.words {
                        let w = if matches!(a.kind, Kind::Q | Kind::E) {
                            a.word + 1
                        } else {
                            a.word
                        };
                        self.min_required = self.min_required.max(w + 1);
                    }
                }
            }
            match it.kind {
                ItemKind::Arg { op, word, .. } => {
                    let word = word as usize;
                    let last = match op {
                        Op::Q | Op::E => word + 1,
                        Op::S(n) => word + (n as usize).saturating_sub(1),
                        _ => word,
                    };
                    upd(&mut self.max_arg, last);
                    if matches!(op, Op::S(_) | Op::SZ) {
                        self.str_words.insert(word);
                    }
                }
                ItemKind::Text(t) => {
                    let mut max: Option<usize> = None;
                    let mut has_str = false;
                    for s in &Template::parse(t).segs {
                        match s {
                            Seg::Val { kind, word, .. }
                            | Seg::Names { kind, word, .. }
                            | Seg::Enum { kind, word, .. } => {
                                if matches!(s, Seg::Names { .. } | Seg::Enum { .. }) {
                                    self.table_words.insert(*word);
                                }
                                upd(
                                    &mut max,
                                    if matches!(kind, Kind::Q | Kind::E) {
                                        word + 1
                                    } else {
                                        *word
                                    },
                                );
                            }
                            Seg::Str { kind, word, .. } => {
                                has_str = true;
                                self.raw_str |= matches!(kind, StrKind::Raw);
                                if let Some(w) = word {
                                    upd(&mut max, *w);
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(m) = max {
                        self.min_required = self.min_required.max(m + 1);
                        if has_str {
                            self.str_words.insert(m);
                        }
                    }
                }
                ItemKind::Flags { word, .. } => {
                    self.min_required = self.min_required.max(word as usize + 1);
                    self.table_words.insert(word as usize);
                }
                ItemKind::Dump => self.dump = true,
            }
        }
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

    fn r(tpl: &'static str, w: &[u32]) -> String {
        let tables: HashMap<&'static str, Table> = [("f", &[(1u64, "A"), (4, "C")][..])].into();
        let mut out = String::new();
        render(&compile(&[t(tpl)]), w, &tables, Missing::Omit, &mut out);
        out
    }

    #[test]
    fn templates() {
        assert_eq!(r("x:0x{x0:08} d:{d1}", &[0xab, 0xffff_ffff]), "x:0x000000ab d:-1");
        assert_eq!(r("{q0:016}", &[1, 2]), "0000000200000001");
        assert_eq!(r("{q0:#08}", &[0x9, 0]), "0x000009");
        assert_eq!(r("{q0:#08}", &[0, 0]), "00000000");
        assert_eq!(r("{u0&0xffff}", &[0x1234_5678]), "22136");
        assert_eq!(r("[{names:f:u0}]", &[5]), "[A C]");
        assert_eq!(r("{enum:f:u0:NONE}", &[2]), "NONE");
        assert_eq!(r("{{x}}", &[]), "{x}");
        let w = [u32::from_le_bytes(*b"ab\0c"), u32::from_le_bytes(*b"xyz\0")];
        assert_eq!(r("{r0}|{rnext}", &w), "ab|cxyz");
        assert_eq!(r("{s0}", &w), "ab");
    }

    #[test]
    fn args_and_missing() {
        let tables = HashMap::new();
        let items = compile(&[
            h("a", 0),
            d("b", 1),
            q("c", 2),
            s("m", 0, 3),
            sz("n", 2).when("words >= 3"),
        ]);
        let mut out = String::new();
        render(&items, &[0x41414141, 0xffff_ffff, 7], &tables, Missing::Omit, &mut out);
        assert_eq!(
            out,
            " a:0x41414141 b:-1 c:TRUNC:7 m:\"AAAA\" (0x41414141 0xffffffff 0x00000007) n:"
        );
        let mut out = String::new();
        render(
            &compile(&[h("a", 0), h("b", 1)]),
            &[1],
            &tables,
            Missing::Mark("(missing)"),
            &mut out,
        );
        assert_eq!(out, " a:0x00000001 b:(missing)");
    }

    #[test]
    fn conds() {
        let c = Cond::parse("words >= 2 && u1 & 0x10 == 0");
        assert!(c.eval(&[0, 0x20]));
        assert!(!c.eval(&[0, 0x10]));
        assert!(!c.eval(&[0]));
        assert!(Cond::parse("u0 & 0x1").eval(&[3]));
    }
}
