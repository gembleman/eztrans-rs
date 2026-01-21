#[inline]
pub const fn is_safe_chars(c: char) -> bool {
    let code = c as u32;
    matches!(code,
        0x000020 |
        0x0000A0..=0x0000A3 |
        0x0000A5..=0x0000A6 |
        0x0000A9..=0x0000AC |
        0x0000AE..=0x0000B0 |
        0x0000B2..=0x0000B3 |
        0x0000B9 |
        0x0000C0..=0x0000D6 |
        0x0000D8..=0x0000F6 |
        0x0000F8..=0x0000FF |
        0x00029E |
        0x00033A |
        0x000492 |
        0x0004A1 |
        0x0004A4 |
        0x00210D |
        0x002202 |
        0x00222B..=0x00222C |
        0x0022E6 |
        0x00246F..=0x002473 |
        0x0024B6..=0x0024BE |
        0x0024C0..=0x0024C3 |
        0x0024C5..=0x0024C8 |
        0x0024CA..=0x0024CF |
        0x002582 |
        0x0025EF |
        0x003013 |
        0x003099..=0x00309A |
        0x003232 |
        0x003239 |
        0x0032A4..=0x0032A8 |
        0x00565B |
        0x005699 |
        0x005BE4 |
        0x005CFB |
        0x006766 |
        0x0067BB |
        0x0067C0 |
        0x006844 |
        0x0068CF |
        0x006998 |
        0x0069E2 |
        0x006A30 |
        0x006A46 |
        0x006A73 |
        0x006A7E |
        0x006AE2 |
        0x006AE4 |
        0x006BD6 |
        0x006C3F |
        0x006C5C |
        0x006C6F |
        0x006C86 |
        0x006CDA |
        0x006D04 |
        0x006D6F |
        0x006D87 |
        0x007195 |
        0x007F52 |
        0x008A51 |
        0x009357 |
        0x0093A4 |
        0x0093C6 |
        0x0093DE |
        0x0093F8 |
        0x009431 |
        0x009445 |
        0x009448 |
        0x00969D |
        0x0096AF |
        0x009733 |
        0x00973B |
        0x009743 |
        0x00974D |
        0x00974F |
        0x009755 |
        0x009857 |
        0x009865 |
        0x009927 |
        0x00999E |
        0x00F929 |
        0x00F9DC |
        0x00FA13..=0x00FA14 |
        0x00FA29..=0x00FA2C
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_chars() {
        // 안전한 문자 테스트 (범위에 포함된 문자들)
        assert!(is_safe_chars(' '));  // U+000020
        assert!(is_safe_chars('¡'));  // U+0000A1
        assert!(is_safe_chars('À'));  // U+0000C0
        assert!(is_safe_chars('Ø'));  // U+0000D8

        // 안전하지 않은 문자 테스트 (전각 문자 및 범위 밖 문자)
        assert!(!is_safe_chars('A'));  // U+000041 (범위 밖)
        assert!(!is_safe_chars('Ａ')); // U+FF21 (전각 A)
        assert!(!is_safe_chars('０')); // U+FF10 (전각 0)
        assert!(!is_safe_chars('　')); // U+003000 (전각 공백)
    }
}
