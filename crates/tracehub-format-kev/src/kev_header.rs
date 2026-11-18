//! kev のヘッダの読み取り(QNX の版に共通)。
//!
//! ヘッダは `TRACE_<キー>::<値>` が区切りなしに続く ASCII の文字列で、`TRACE_HEADER_END::` で終わる。
//! キーは `TRACE_` + 英大文字・数字・`_` + `::`。値は次のキーの直前まで。
//! 誤りの基準は traceprinter に合わせる。

use crate::model::{KevError, KevHeader, QnxVersion};

const PREFIX: &[u8] = b"TRACE_";
const POSTFIX: &[u8] = b"::";
const KEY_END: &str = "TRACE_HEADER_END";
const KEY_VER_MAJOR: &str = "TRACE_VER_MAJOR";
const KEY_VER_MINOR: &str = "TRACE_VER_MINOR";
const KEY_LITTLE_ENDIAN: &str = "TRACE_LITTLE_ENDIAN";
const KEY_BIG_ENDIAN: &str = "TRACE_BIG_ENDIAN";
const KEY_CYCLES_PER_SEC: &str = "TRACE_CYCLES_PER_SEC";
const KEY_CPU_NUM: &str = "TRACE_CPU_NUM";
const KEY_SYSPAGE_LEN: &str = "TRACE_SYSPAGE_LEN";
const KEY_SYS_RELEASE: &str = "TRACE_SYS_RELEASE";
/// TRACE_SYS_RELEASE が無いときの版
const DEFAULT_VERSION: QnxVersion = QnxVersion::V800;
/// traceprinter が読めない、ヘッダの項目(キー::値)の長さ
const ITEM_LEN_LIMIT: usize = 2048;

/// ヘッダを読む。戻り値: (ヘッダ, ヘッダの直後のバイト位置)
pub fn parse(buf: &[u8]) -> Result<(KevHeader, usize), KevError> {
    let mut keys: Vec<(String, usize, usize)> = Vec::new(); // (キー, キーの位置, 値の位置)
    let mut end = None;
    let mut pos = 0;
    while let Some(p) = find(buf, PREFIX, pos) {
        let mut q = p + PREFIX.len();
        while q < buf.len() && (buf[q].is_ascii_uppercase() || buf[q].is_ascii_digit() || buf[q] == b'_') {
            q += 1;
        }
        if q > p + PREFIX.len() && buf[q..].starts_with(POSTFIX) {
            let key = latin1(&buf[p..q]);
            let vstart = q + POSTFIX.len();
            let is_end = key == KEY_END;
            keys.push((key, p, vstart));
            pos = vstart;
            if is_end {
                end = Some(vstart);
                break;
            }
        } else {
            pos = p + PREFIX.len();
        }
    }
    let end = end.ok_or(KevError::NoHeaderEnd)?;

    let mut keywords = Vec::with_capacity(keys.len());
    for (i, (key, start, vstart)) in keys.iter().enumerate() {
        let vend = keys.get(i + 1).map_or(*vstart, |k| k.1);
        if vend - start >= ITEM_LEN_LIMIT {
            return Err(KevError::HeaderItemTooLong {
                key: key.clone(),
                len: vend - start,
            });
        }
        keywords.push((key.clone(), latin1(&buf[*vstart..vend])));
    }
    let get = |k: &str| keywords.iter().rev().find(|(key, _)| key == k).map(|(_, v)| v.as_str());

    // traceprinter は形式の版のキーが無いと読まない(値は問わない)
    for key in [KEY_VER_MAJOR, KEY_VER_MINOR] {
        if get(key).is_none() {
            return Err(KevError::MissingKey { key: key.into() });
        }
    }
    // TRACE_SYS_RELEASE が無ければ DEFAULT_VERSION として読む(traceprinter は版を見ずに読む)
    let version = match get(KEY_SYS_RELEASE) {
        None => {
            log::warn!(
                "ヘッダに TRACE_SYS_RELEASE がありません。QNX {} として読みます",
                DEFAULT_VERSION.name()
            );
            DEFAULT_VERSION
        }
        Some(r) => QnxVersion::from_release(r).ok_or_else(|| KevError::UnsupportedVersion { release: r.into() })?,
    };
    let num = |k: &str| get(k).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
    let little_endian = get(KEY_LITTLE_ENDIAN).is_some() || get(KEY_BIG_ENDIAN).is_none();
    let header = KevHeader {
        version,
        cycles_per_sec: num(KEY_CYCLES_PER_SEC),
        cpu_num: num(KEY_CPU_NUM) as u32,
        syspage_len: num(KEY_SYSPAGE_LEN),
        little_endian,
        keywords,
    };
    Ok((header, end))
}

/// latin-1 のバイト列を、1 バイト 1 文字の文字列にする
pub(crate) fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = "TRACE_HEADER_BEGIN::TRACE_VER_MAJOR::2TRACE_VER_MINOR::01TRACE_LITTLE_ENDIAN::TRUE\
                      TRACE_CYCLES_PER_SEC::19200000TRACE_CPU_NUM::4TRACE_SYS_RELEASE::8.0.0TRACE_SYSPAGE_LEN::0\
                      TRACE_X_Y::a::bTRACE_HEADER_END::";

    #[test]
    fn reads_keywords_in_order() {
        let mut b = OK.as_bytes().to_vec();
        b.extend_from_slice(&[1, 2, 3]);
        let (h, end) = parse(&b).unwrap();
        assert_eq!(end, OK.len());
        assert_eq!(h.version, QnxVersion::V800);
        assert_eq!((h.cycles_per_sec, h.cpu_num, h.syspage_len), (19_200_000, 4, 0));
        assert!(h.little_endian);
        let keys: Vec<&str> = h.keywords.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys[0], "TRACE_HEADER_BEGIN");
        assert_eq!(*keys.last().unwrap(), "TRACE_HEADER_END");
        assert_eq!(h.get("TRACE_X_Y"), Some("a::b")); // キーの形でない "::" は値に含まれる
    }

    #[test]
    fn errors() {
        assert_eq!(parse(b"TRACE_HEADER_BEGIN::").unwrap_err(), KevError::NoHeaderEnd);
        let no_ver = OK.replace("TRACE_VER_MAJOR::2", "");
        assert!(matches!(parse(no_ver.as_bytes()), Err(KevError::MissingKey { .. })));
        let v7 = OK.replace("8.0.0", "7.1.0");
        assert!(matches!(parse(v7.as_bytes()), Err(KevError::UnsupportedVersion { .. })));
        let no_rel = OK.replace("TRACE_SYS_RELEASE::8.0.0", "");
        assert_eq!(parse(no_rel.as_bytes()).unwrap().0.version, DEFAULT_VERSION);
        let long = OK.replace("a::b", &"x".repeat(3000));
        assert!(matches!(
            parse(long.as_bytes()),
            Err(KevError::HeaderItemTooLong { .. })
        ));
    }
}
