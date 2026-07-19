#[cfg(test)]
mod tests {
    use eztrans_rs::char_ranges::is_safe_chars;

    fn needs_special_encoding(c: char) -> bool {
        !is_safe_chars(c)
    }

    #[test]
    fn test_special_encoding_basic() {
        // Circled numbers (U+2460-2473) - now in safe range
        assert!(!needs_special_encoding('①'));
        assert!(!needs_special_encoding('②'));

        // Currency (U+20AC) - not in safe range
        assert!(needs_special_encoding('€'));

        // CJK Compatibility (U+3395, 3396) - not in safe range
        assert!(needs_special_encoding('㎕'));
        assert!(needs_special_encoding('㎖'));

        // Basic arrows
        // '→' (U+2192) is in safe range (U+2190-2193)
        assert!(!needs_special_encoding('→'));
        // '↔' (U+2194) is NOT in safe range
        assert!(needs_special_encoding('↔'));
    }

    #[test]
    fn test_safe_characters() {
        // ASCII letters and numbers
        assert!(!needs_special_encoding('A'));
        assert!(!needs_special_encoding('z'));
        assert!(!needs_special_encoding('0'));
        assert!(!needs_special_encoding('9'));

        // Hiragana
        assert!(!needs_special_encoding('あ'));
        assert!(!needs_special_encoding('ん'));

        // Katakana
        assert!(!needs_special_encoding('ア'));
        assert!(!needs_special_encoding('ン'));

        // Kanji
        assert!(!needs_special_encoding('日'));
        assert!(!needs_special_encoding('本'));
    }

    #[test]
    fn test_at_symbol() {
        // @ is now treated as safe character (ASCII range)
        assert!(!needs_special_encoding('@'));
    }

    #[test]
    fn test_control_characters() {
        assert!(needs_special_encoding('\0'));
        // 다른 제어 문자는 char_ranges에서 확인 필요
    }

    #[test]
    fn test_emoji_char_decomposition() {
        // 단일 코드포인트 이모지
        let single_emoji = "😀";
        let chars: Vec<char> = single_emoji.chars().collect();
        println!("단일 이모지 '{}': {:?}", single_emoji, chars);
        assert_eq!(chars.len(), 1);
        println!("  U+{:04X}", chars[0] as u32);

        // ZWJ 시퀀스 이모지 (가족)
        let family_emoji = "👨‍👩‍👧";
        let chars: Vec<char> = family_emoji.chars().collect();
        println!("가족 이모지 '{}': {} chars", family_emoji, chars.len());
        for (i, c) in chars.iter().enumerate() {
            println!("  [{}] U+{:04X} = '{}'", i, *c as u32, c);
        }
        // 👨 + ZWJ + 👩 + ZWJ + 👧 = 5개
        assert!(chars.len() > 1);

        // 국기 이모지 (Regional Indicator)
        let flag_emoji = "🇰🇷";
        let chars: Vec<char> = flag_emoji.chars().collect();
        println!("국기 이모지 '{}': {} chars", flag_emoji, chars.len());
        for (i, c) in chars.iter().enumerate() {
            println!("  [{}] U+{:04X}", i, *c as u32);
        }
        // 🇰 + 🇷 = 2개
        assert_eq!(chars.len(), 2);

        // 피부색 수정자 이모지
        let skin_emoji = "👋🏻";
        let chars: Vec<char> = skin_emoji.chars().collect();
        println!("피부색 이모지 '{}': {} chars", skin_emoji, chars.len());
        for (i, c) in chars.iter().enumerate() {
            println!("  [{}] U+{:04X}", i, *c as u32);
        }
        // 👋 + 🏻 = 2개
        assert_eq!(chars.len(), 2);
    }

    #[test]
    fn test_emoji_encode_decode_roundtrip() {
        // lib.rs의 인코딩/디코딩 방식 시뮬레이션 (6자리 고정 hex)
        fn encode(s: &str) -> String {
            use std::fmt::Write;
            let mut output = String::new();
            for c in s.chars() {
                let code = c as u32;
                // 이모지 범위 또는 특수 문자
                if code >= 0x1F000
                    || c == '\u{200D}'
                    || (code >= 0x1F1E0 && code <= 0x1F1FF)
                    || code >= 0x10000
                {
                    write!(&mut output, "+X{:06X}", code).unwrap();
                } else if code >= 0xAC00 && code <= 0xD7A3 {
                    // 한글
                    write!(&mut output, "+x{:06X}", code).unwrap();
                } else {
                    output.push(c);
                }
            }
            output
        }

        fn decode(s: &str) -> String {
            let mut output = String::new();
            let mut chars = s.chars().peekable();

            while let Some(c) = chars.next() {
                if c == '+' {
                    if let Some(&next) = chars.peek() {
                        if next == 'X' || next == 'x' {
                            chars.next();
                            // 6자리 고정 hex 읽기
                            let hex: String = chars.by_ref().take(6).collect();
                            if hex.len() == 6 && hex.chars().all(|h| h.is_ascii_hexdigit()) {
                                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                    if let Some(decoded) = char::from_u32(code) {
                                        output.push(decoded);
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                output.push(c);
            }
            output
        }

        let test_cases = [
            "😀",           // 단일 이모지
            "Hello 😀",     // 텍스트 + 이모지
            "👨‍👩‍👧",           // ZWJ 시퀀스
            "🇰🇷",           // 국기
            "👋🏻",           // 피부색
            "テスト😀です", // 일본어 + 이모지
            "한글테스트",   // 한글
            "테스트123",    // 한글 + 숫자
            "😀123ABC",     // 이모지 + 숫자 + 문자
        ];

        for original in test_cases {
            let encoded = encode(original);
            let decoded = decode(&encoded);
            println!(
                "원본: '{}' -> 인코딩: '{}' -> 디코딩: '{}'",
                original, encoded, decoded
            );
            assert_eq!(original, decoded, "라운드트립 실패: {}", original);
        }
    }
}
