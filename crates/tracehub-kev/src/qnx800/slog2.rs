//! SLOG2(SYSTEM の SLOG の payloads に入った slog2 のレコード)の表示文字列。
//!
//! 規則は traceprinter の振る舞いから決めた(設計書 §7.3)。定数は args.rs の SLOG2 の節。
//! - payloads が本文の位置より短いか、先頭が SLOG2_MAGIC でなければ SLOG2 ではない(SLOG の書式)
//! - レコード長が本文の位置以下・上限より大きい・payloads より長い → 空
//! - 種別が文字列 → 本文を NUL の手前まで、種別が書式 → 前置き + 書式を当てはめた文字列、ほかの種別 → 空
//! - 書式を当てはめられない(引数が足りない・型が合わない・扱わない変換)ときは書式をそのまま出す
//!
//! 書式の当てはめは C の printf と同じ(traceprinter は Windows の実行ファイルなので long は 32 ビット)。違うところ:
//! - 扱う変換は d i u x X o p c s f F e E g G a A と %%。フラグは `-+ #0`、幅・精度は数字だけ(`*`・`$` は無し)。
//!   精度の `.` の後ろに数字が無いもの、`%` の前にフラグなどがあるもの、末尾の `%` は当てはめられない
//! - 数の引数は 1・2・4・8 バイト。符号付きの変換(d i)では 4 バイトを符号拡張し、ほかは 0 で拡張する
//! - %s の引数は文字列でも数でもよく、データの最後のバイトが NUL でなければ当てはめられない。
//!   文字列はデータの中の最初の NUL まで。ほかの変換に文字列の引数は当てはめられない
//! - %ls など(SLOG2_WIDE_LENGTHS)はワイド文字(UTF-16)。%ls はデータの先頭から、データの後ろも続けて、
//!   0 か 0xff より大きい文字の手前まで読む。%lc は下位 16 ビットで、0xff より大きければ空
//! - 浮動小数点の変換は、0 で拡張した 64 ビットを double のビット列として読む。NaN に符号を付けない
//! - %p: フラグ・幅・精度・長さ修飾子が無ければ 0 で埋めた 16 桁の 16 進、あれば %llx と同じ(# で 0x)
//! - %u: 空白のフラグで前に空白を付ける(+ は付けない)
//! - %#o: 値が 0 でなく、桁が 0 で始まらないときだけ 0 を前に付ける
//! - 結果は最初の NUL(%c の 0)の手前まで

use super::args;

/// payloads が slog2 のレコードなら、traceprinter がイベント名の後ろに出す文字列(1 文字 1 バイト)。
/// SLOG2 でなければ None
pub(crate) fn render(words: &[u32]) -> Option<String> {
    let b: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let body = args::SLOG2_BODY_OFFSET;
    if b.len() < body || u16le(&b, 0) != args::SLOG2_MAGIC {
        return None;
    }
    let len = u16le(&b, args::SLOG2_LEN_OFFSET) as usize;
    let mut out = Vec::new();
    if len > body && len <= b.len() && len <= args::SLOG2_MAX_RECORD {
        let rec = &b[..len];
        match rec[args::SLOG2_TYPE_OFFSET] {
            args::SLOG2_TYPE_STRING => out.extend_from_slice(until_nul(&rec[body..len - 1])),
            args::SLOG2_TYPE_FORMAT => {
                out.extend_from_slice(args::SLOG2_FORMAT_PREFIX.as_bytes());
                if let Some(n) = rec[body..].iter().position(|&c| c == 0)
                    && n <= args::SLOG2_MAX_FORMAT
                {
                    let fmt = &rec[body..body + n];
                    let mut reader = ArgReader {
                        payload: &b,
                        end: len,
                        pos: body + n + 1,
                    };
                    let s = printf(fmt, &mut reader).unwrap_or_else(|| fmt.to_vec());
                    let s = until_nul(&s);
                    out.extend_from_slice(&s[..s.len().min(args::SLOG2_MAX_OUTPUT)]);
                }
            }
            _ => {}
        }
    }
    Some(out.iter().map(|&c| c as char).collect())
}

fn u16le(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn until_nul(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
}

// ---------------------------------------------------------------- 引数

struct Arg<'a> {
    string: bool,
    data: &'a [u8],
    /// データの先頭から payloads の終わりまで(ワイド文字列はデータの後ろまで読む)
    rest: &'a [u8],
}

impl Arg<'_> {
    /// 数として読む(0 で拡張)。数の引数でないか長さが合わなければ None
    fn int(&self) -> Option<(u64, usize)> {
        if self.string || !args::SLOG2_INT_SIZES.contains(&self.data.len()) {
            return None;
        }
        let mut v = [0u8; 8];
        v[..self.data.len()].copy_from_slice(self.data);
        Some((u64::from_le_bytes(v), self.data.len()))
    }

    /// %s の文字列。データの最後のバイトが NUL でなければ None
    fn text(&self, wide: bool) -> Option<Vec<u8>> {
        if self.data.last() != Some(&0) {
            return None;
        }
        if !wide {
            return Some(until_nul(self.data).to_vec());
        }
        // UTF-16 の文字を、0 か 1 バイトにならない文字の手前まで
        Some(
            self.rest
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&w| w != 0 && w <= 0xff)
                .map(|w| w as u8)
                .collect(),
        )
    }
}

struct ArgReader<'a> {
    payload: &'a [u8],
    /// レコード長
    end: usize,
    pos: usize,
}

impl<'a> ArgReader<'a> {
    fn next(&mut self) -> Option<Arg<'a>> {
        let p = self.pos.next_multiple_of(args::SLOG2_ARG_ALIGN);
        if p + 2 > self.end {
            return None;
        }
        let h = u16le(self.payload, p);
        let n = (h & args::SLOG2_ARG_LEN_MASK) as usize;
        let d = p + 2;
        if d + n > self.end {
            return None;
        }
        self.pos = d + n;
        Some(Arg {
            string: h & args::SLOG2_ARG_STRING != 0,
            data: &self.payload[d..d + n],
            rest: &self.payload[d..],
        })
    }
}

// ---------------------------------------------------------------- 書式

#[derive(Default)]
struct Conv {
    left: bool,
    plus: bool,
    space: bool,
    alt: bool,
    zero: bool,
    width: Option<usize>,
    prec: Option<usize>,
    /// 長さ修飾子の整数のビット数(修飾子が無ければ None)
    length: Option<u32>,
    /// %s・%c がワイド文字(長さ修飾子が SLOG2_WIDE_LENGTHS)
    wide: bool,
}

impl Conv {
    fn plain(&self) -> bool {
        !(self.left || self.plus || self.space || self.alt || self.zero)
            && self.width.is_none()
            && self.prec.is_none()
            && self.length.is_none()
    }
}

/// 数字の並び。i32 に収まらなければ無いものとする
fn digits(fmt: &[u8], i: &mut usize) -> Option<Option<usize>> {
    let start = *i;
    let mut v: u64 = 0;
    while let Some(&c) = fmt.get(*i)
        && c.is_ascii_digit()
    {
        v = (v * 10 + (c - b'0') as u64).min(u64::MAX / 100);
        *i += 1;
    }
    if *i == start {
        return None;
    }
    Some((v <= i32::MAX as u64).then_some(v as usize))
}

/// 書式を当てはめる。当てはめられなければ None
fn printf(fmt: &[u8], a: &mut ArgReader) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < fmt.len() {
        let c = fmt[i];
        i += 1;
        if c != b'%' {
            out.push(c);
            continue;
        }
        if fmt.get(i) == Some(&b'%') {
            out.push(b'%');
            i += 1;
            continue;
        }
        let mut cv = Conv::default();
        while let Some(&f) = fmt.get(i) {
            match f {
                b'-' => cv.left = true,
                b'+' => cv.plus = true,
                b' ' => cv.space = true,
                b'#' => cv.alt = true,
                b'0' => cv.zero = true,
                _ => break,
            }
            i += 1;
        }
        if let Some(w) = digits(fmt, &mut i) {
            cv.width = w;
        }
        if fmt.get(i) == Some(&b'.') {
            i += 1;
            cv.prec = Some(digits(fmt, &mut i)?.unwrap_or(0));
        }
        if let Some(&(m, bits)) = args::SLOG2_LENGTHS
            .iter()
            .find(|(m, _)| fmt[i..].starts_with(m.as_bytes()))
        {
            cv.length = Some(bits);
            cv.wide = args::SLOG2_WIDE_LENGTHS.contains(&m);
            i += m.len();
        }
        let conv = *fmt.get(i)?;
        i += 1;
        let body = match conv {
            b'd' | b'i' => {
                let (raw, size) = a.next()?.int()?;
                let v = if size == 4 {
                    raw as u32 as i32 as i64
                } else {
                    raw as i64
                };
                let v = match cv.length.unwrap_or(args::SLOG2_INT_BITS) {
                    8 => v as i8 as i64,
                    16 => v as i16 as i64,
                    32 => v as i32 as i64,
                    _ => v,
                };
                let sign = if v < 0 {
                    "-"
                } else if cv.plus {
                    "+"
                } else if cv.space {
                    " "
                } else {
                    ""
                };
                integer(&cv, sign, "", v.unsigned_abs(), 10, false)
            }
            b'u' | b'x' | b'X' | b'o' => {
                let (raw, _) = a.next()?.int()?;
                let v = match cv.length.unwrap_or(args::SLOG2_INT_BITS) {
                    8 => raw as u8 as u64,
                    16 => raw as u16 as u64,
                    32 => raw as u32 as u64,
                    _ => raw,
                };
                match conv {
                    b'u' => integer(&cv, if cv.space { " " } else { "" }, "", v, 10, false),
                    b'o' => integer(&cv, "", "", v, 8, false),
                    _ => {
                        let upper = conv == b'X';
                        let pre = match (cv.alt && v != 0, upper) {
                            (false, _) => "",
                            (true, false) => "0x",
                            (true, true) => "0X",
                        };
                        integer(&cv, "", pre, v, 16, upper)
                    }
                }
            }
            b'p' => {
                let (raw, _) = a.next()?.int()?;
                if cv.plain() {
                    format!("{raw:0w$x}", w = args::SLOG2_POINTER_DIGITS).into_bytes()
                } else {
                    integer(&cv, "", if cv.alt && raw != 0 { "0x" } else { "" }, raw, 16, false)
                }
            }
            b'c' => {
                let (raw, _) = a.next()?.int()?;
                // ワイド文字は下位 16 ビット。1 バイトにならなければ空
                let ch = match (cv.wide, raw as u16) {
                    (false, _) => vec![raw as u8],
                    (true, w) if w <= 0xff => vec![w as u8],
                    (true, _) => Vec::new(),
                };
                pad(&cv, Vec::new(), ch, false)
            }
            b's' => {
                let mut s = a.next()?.text(cv.wide)?;
                if let Some(p) = cv.prec {
                    s.truncate(p);
                }
                pad(&cv, Vec::new(), s, false)
            }
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                let (raw, _) = a.next()?.int()?;
                float(&cv, conv, f64::from_bits(raw))
            }
            _ => return None,
        };
        out.extend_from_slice(&body);
    }
    Some(out)
}

/// 幅に合わせる。zero なら符号・前置きと本体の間を 0 で埋める
fn pad(cv: &Conv, head: Vec<u8>, body: Vec<u8>, zero: bool) -> Vec<u8> {
    let len = head.len() + body.len();
    let fill = cv.width.unwrap_or(0).saturating_sub(len);
    let mut out = Vec::with_capacity(len + fill);
    if cv.left {
        out.extend(head);
        out.extend(body);
        out.resize(len + fill, b' ');
    } else if zero {
        out.extend(head);
        out.resize(out.len() + fill, b'0');
        out.extend(body);
    } else {
        out.resize(fill, b' ');
        out.extend(head);
        out.extend(body);
    }
    out
}

/// 整数の変換。sign は符号、pre は 0x などの前置き
fn integer(cv: &Conv, sign: &str, pre: &str, v: u64, base: u32, upper: bool) -> Vec<u8> {
    let mut d = match base {
        8 => format!("{v:o}"),
        16 if upper => format!("{v:X}"),
        16 => format!("{v:x}"),
        _ => v.to_string(),
    };
    if let Some(p) = cv.prec {
        if p == 0 && v == 0 {
            d.clear();
        }
        if d.len() < p {
            d = "0".repeat(p - d.len()) + &d;
        }
    }
    if base == 8 && cv.alt && v != 0 && !d.starts_with('0') {
        d.insert(0, '0');
    }
    let head = format!("{sign}{pre}").into_bytes();
    pad(cv, head, d.into_bytes(), cv.zero && cv.prec.is_none() && !cv.left)
}

/// 浮動小数点の変換
fn float(cv: &Conv, conv: u8, x: f64) -> Vec<u8> {
    let upper = conv.is_ascii_uppercase();
    let sign = if x.is_sign_negative() && !x.is_nan() {
        "-"
    } else if cv.plus {
        "+"
    } else if cv.space {
        " "
    } else {
        ""
    };
    if !x.is_finite() {
        let s = if x.is_nan() { "nan" } else { "inf" };
        let s = if upper { s.to_ascii_uppercase() } else { s.into() };
        return pad(cv, sign.as_bytes().to_vec(), s.into_bytes(), false);
    }
    let a = x.abs();
    let prec = cv.prec.unwrap_or(6);
    let (pre, body) = match conv.to_ascii_lowercase() {
        b'f' => ("", fixed(a, prec, cv.alt)),
        b'e' => ("", exp(a, prec, cv.alt, upper)),
        b'g' => ("", general(a, prec, cv.alt, upper)),
        _ => (if upper { "0X" } else { "0x" }, hex(a, cv.prec, cv.alt, upper)),
    };
    let head = format!("{sign}{pre}").into_bytes();
    pad(cv, head, body.into_bytes(), cv.zero && !cv.left)
}

fn fixed(a: f64, prec: usize, alt: bool) -> String {
    let mut s = format!("{a:.prec$}");
    if alt && prec == 0 {
        s.push('.');
    }
    s
}

/// 仮数(小数点以下 prec 桁)と 10 進の指数
fn exp_parts(a: f64, prec: usize) -> (String, i32) {
    let s = format!("{a:.prec$e}");
    let (m, e) = s.split_once('e').unwrap();
    (m.to_string(), e.parse().unwrap())
}

fn exp(a: f64, prec: usize, alt: bool, upper: bool) -> String {
    let (mut m, e) = exp_parts(a, prec);
    if alt && prec == 0 {
        m.push('.');
    }
    let sign = if e < 0 { '-' } else { '+' };
    format!("{m}{}{sign}{:02}", if upper { 'E' } else { 'e' }, e.unsigned_abs())
}

fn general(a: f64, prec: usize, alt: bool, upper: bool) -> String {
    let p = prec.max(1);
    let (_, x) = exp_parts(a, p - 1);
    let mut s = if (x as i64) < p as i64 && x >= -4 {
        fixed(a, (p as i64 - 1 - x as i64) as usize, alt)
    } else {
        exp(a, p - 1, alt, upper)
    };
    if !alt {
        // 小数部の末尾の 0 と小数点を除く
        let (num, tail) = match s.find(['e', 'E']) {
            Some(k) => (s[..k].to_string(), s[k..].to_string()),
            None => (s.clone(), String::new()),
        };
        let num = if num.contains('.') {
            num.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            num
        };
        s = num + &tail;
    }
    s
}

/// %a(0x の後ろ)
fn hex(a: f64, prec: Option<usize>, alt: bool, upper: bool) -> String {
    const FRAC_DIGITS: usize = 13;
    let bits = a.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (mut lead, e) = match (biased, frac) {
        (0, 0) => (0u64, 0),
        (0, _) => (0, -1022),
        _ => (1, biased - 1023),
    };
    let mut d = match prec {
        None => format!("{frac:013x}").trim_end_matches('0').to_string(),
        Some(p) if p >= FRAC_DIGITS => format!("{frac:013x}") + &"0".repeat(p - FRAC_DIGITS),
        Some(p) => {
            // 偶数への丸め(p = 0 なら先頭の桁の偶奇)
            let shift = 4 * (FRAC_DIGITS - p) as u32;
            let mut q = frac >> shift;
            let rem = frac & ((1u64 << shift) - 1);
            let half = 1u64 << (shift - 1);
            let odd = if p == 0 { lead & 1 == 1 } else { q & 1 == 1 };
            if rem > half || (rem == half && odd) {
                q += 1;
                if q >> (4 * p) != 0 {
                    q = 0;
                    lead += 1;
                }
            }
            if p == 0 { String::new() } else { format!("{q:0p$x}") }
        }
    };
    if upper {
        d = d.to_ascii_uppercase();
    }
    let dot = if !d.is_empty() || alt { "." } else { "" };
    let p = if upper { 'P' } else { 'p' };
    let sign = if e < 0 { '-' } else { '+' };
    format!("{lead}{dot}{d}{p}{sign}{}", e.unsigned_abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 書式と引数からレコード(種別は書式)を作り、payloads の語にする
    fn record(fmt: &[u8], params: &[(bool, &[u8])]) -> Vec<u32> {
        let mut r = vec![0u8; args::SLOG2_BODY_OFFSET];
        r[..2].copy_from_slice(&args::SLOG2_MAGIC.to_le_bytes());
        r[args::SLOG2_TYPE_OFFSET] = args::SLOG2_TYPE_FORMAT;
        r.extend_from_slice(fmt);
        r.push(0);
        for (string, d) in params {
            r.resize(r.len().next_multiple_of(args::SLOG2_ARG_ALIGN), 0);
            let h = d.len() as u16 | if *string { args::SLOG2_ARG_STRING } else { 0 };
            r.extend_from_slice(&h.to_le_bytes());
            r.extend_from_slice(d);
        }
        let len = r.len() as u16;
        r[2..4].copy_from_slice(&len.to_le_bytes());
        r.resize(r.len().next_multiple_of(8), 0);
        r.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
    }

    fn i4(v: i32) -> (bool, Vec<u8>) {
        (false, v.to_le_bytes().to_vec())
    }

    fn i8b(v: i64) -> (bool, Vec<u8>) {
        (false, v.to_le_bytes().to_vec())
    }

    fn s(v: &str) -> (bool, Vec<u8>) {
        let mut b = v.as_bytes().to_vec();
        b.push(0);
        (true, b)
    }

    fn run(fmt: &str, params: &[(bool, Vec<u8>)]) -> String {
        let p: Vec<(bool, &[u8])> = params.iter().map(|(a, b)| (*a, b.as_slice())).collect();
        render(&record(fmt.as_bytes(), &p)).unwrap()
    }

    // 期待値はすべて traceprinter の出力
    #[test]
    fn formats_like_traceprinter() {
        assert_eq!(
            run(
                "%s(%d)[tid:%d]: pid %d proc=%s client %p",
                &[
                    s("connect"),
                    i4(62),
                    i4(5),
                    i4(1234),
                    s("server 1234"),
                    i8b(0x12_3456_78d0)
                ]
            ),
            "FA connect(62)[tid:5]: pid 1234 proc=server 1234 client 00000012345678d0"
        );
        assert_eq!(run("%d %i %u", &[i4(-5), i4(-6), i4(-7)]), "FA -5 -6 4294967289");
        assert_eq!(run("%d %u", &[i8b(-5), i8b(-7)]), "FA -5 4294967289");
        assert_eq!(
            run("[%5d][%-5d][%05d]", &[i4(42), i4(42), i4(42)]),
            "FA [   42][42   ][00042]"
        );
        assert_eq!(
            run("[%.3s][%10s][%-10s]", &[s("abcdef"), s("ab"), s("ab")]),
            "FA [abc][        ab][ab        ]"
        );
        assert_eq!(run("%d %d", &[i4(1)]), "FA %d %d");
        assert_eq!(run("%d", &[s("str")]), "FA %d");
        assert_eq!(
            run("[%lld %llu %llx]", &[i4(-1), i4(-1), i4(i32::MIN)]),
            "FA [-1 4294967295 80000000]"
        );
        assert_eq!(
            run("[%lld %hd]", &[(false, vec![0xff, 0xff]), (false, vec![0xff, 0xff])]),
            "FA [65535 -1]"
        );
        assert_eq!(run("[%ld|%lu]", &[i8b(0x1_0000_0001), i8b(0x1_0000_0001)]), "FA [1|1]");
        assert_eq!(
            run("[%p %#p %10p]", &[i4(-1), i4(1), i4(1)]),
            "FA [00000000ffffffff 0x1          1]"
        );
        assert_eq!(
            run("[% u|%+u|% x|%+ u]", &[i4(7), i4(7), i4(7), i4(7)]),
            "FA [ 7|7|7| 7]"
        );
        assert_eq!(run("[%#o|%#.0o|%#5o]", &[i4(0), i4(0), i4(8)]), "FA [0||  010]");
        assert_eq!(
            run("[%.0d|%+.0d|% .0d|%5.0d]", &[i4(0), i4(0), i4(0), i4(0)]),
            "FA [|+| |     ]"
        );
        assert_eq!(run("[%c]", &[i4(0)]), "FA [");
        assert_eq!(run("[%.d]", &[i4(1)]), "FA [%.d]");
        assert_eq!(run("[% %d]", &[i4(1)]), "FA [% %d]");
        assert_eq!(run("[%5%]", &[i4(1)]), "FA [%5%]");
        assert_eq!(run("%% %d %%", &[i4(3)]), "FA % 3 %");
        let f = |v: f64| i8b(v.to_bits() as i64);
        assert_eq!(
            run("[%f %e %g %a]", &[f(1.5), f(1.5), f(1.5), f(1.5)]),
            "FA [1.500000 1.500000e+00 1.5 0x1.8p+0]"
        );
        assert_eq!(
            run(
                "[%.1a|%.0a|%.3a|%10a|%#a|%.0A]",
                &[f(1.5), f(1.5), f(0.1), f(1.5), f(1.0), f(1.5)]
            ),
            "FA [0x1.8p+0|0x2p+0|0x1.99ap-4|  0x1.8p+0|0x1.p+0|0X2P+0]"
        );
        assert_eq!(
            run(
                "[%g|%g|%.0g|%#.3g|%.2g|%G]",
                &[f(1e8), f(0.001), f(0.5), f(100.0), f(99.5), f(1e-7)]
            ),
            "FA [1e+08|0.001|0.5|100.|1e+02|1E-07]"
        );
        assert_eq!(
            run(
                "[%F %+f % f %010f]",
                &[f(f64::INFINITY), f(f64::NAN), f(-f64::NAN), f(f64::NEG_INFINITY)]
            ),
            "FA [INF +nan  nan       -inf]"
        );
        assert_eq!(run("[%a]", &[i4(1)]), "FA [0x0.0000000000001p-1022]");
        assert_eq!(run("[%.2e|%.0f|%.0f]", &[f(99.5), f(0.5), f(2.5)]), "FA [9.95e+01|0|2]");
    }

    #[test]
    fn record_rules() {
        // SLOG2 でない
        assert_eq!(render(&[0x0004_cfbd, 0x0100_0000]), None);
        assert_eq!(render(&[0x0010_cfbc, 0, 0, 0, 0, 0]), None);
        // 種別が文字列: 本文のレコード長 - 1 バイトまで
        let mut w = record(b"plain%d", &[]);
        w[1] &= !0xff00_0000;
        assert_eq!(render(&w).unwrap(), "plain%d");
        // 種別がほか → 空
        let mut w = record(b"x", &[]);
        w[1] |= 0x0200_0000;
        assert_eq!(render(&w).unwrap(), "");
        // レコード長が payloads より長い → 空
        let mut w = record(b"x", &[]);
        w[0] = (w[0] & 0xffff) | (0x100 << 16);
        assert_eq!(render(&w).unwrap(), "");
    }
}
