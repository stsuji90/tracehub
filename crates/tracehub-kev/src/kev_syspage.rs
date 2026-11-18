//! syspage(QNX の版に共通)。第 1 段階では中身を読まず、読み飛ばすだけ。

use crate::model::KevError;

/// syspage を読み飛ばす。戻り値: イベント部の始まりのバイト位置
pub fn skip(buf: &[u8], start: usize, syspage_len: u64) -> Result<usize, KevError> {
    let rest = buf.len().saturating_sub(start) as u64;
    if syspage_len > rest {
        return Err(KevError::SyspageTooShort {
            expected: syspage_len,
            actual: rest,
        });
    }
    Ok(start + syspage_len as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips() {
        assert_eq!(skip(&[0; 10], 4, 6), Ok(10));
        assert_eq!(skip(&[0; 10], 4, 0), Ok(4));
        assert!(skip(&[0; 10], 4, 7).is_err());
    }
}
