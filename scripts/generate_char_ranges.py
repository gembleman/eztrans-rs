#!/usr/bin/env python3
"""Generate char_ranges.rs from scan results"""

with open('full_unicode_scan_v2_results.txt', 'r', encoding='utf-8') as f:
    content = f.read()

start = content.find('Generated Rust ranges:')
ranges_section = content[start + len('Generated Rust ranges:'):].strip()
lines = ranges_section.split('\n')
ranges = [line.strip() for line in lines if line.strip()]

print(f'Found {len(ranges)} ranges')

# Generate Rust code
rust_code = '''//! Character range checking for EzTrans special character encoding
//!
//! This module provides efficient range-based checking for characters that
//! need special encoding before being processed by EzTrans.
//!
//! Generated from full Unicode scan (full_unicode_scan_v2_results.txt)

/// Check if a character needs special encoding based on unicode ranges
///
/// This function checks if a character falls into any of the unicode ranges
/// that EzTrans cannot handle correctly. Characters in these ranges must be
/// encoded using the `+x####` or `+X####` format before translation.
///
/// # Coverage
///
/// This function covers **9,607 problematic characters** across **1,713 ranges**,
/// providing 100% coverage of all characters that EzTrans cannot handle.
///
/// # Examples
///
/// ```
/// use eztrans_rs::char_ranges::needs_special_encoding;
///
/// assert!(needs_special_encoding('@'));  // At symbol
/// assert!(needs_special_encoding('\u{2460}')); // Circled number 1
/// assert!(needs_special_encoding('\u{20AC}')); // Euro sign
/// assert!(!needs_special_encoding('\u{3042}')); // Hiragana (safe)
/// assert!(!needs_special_encoding('A')); // ASCII letter (safe)
/// ```
#[inline]
pub const fn needs_special_encoding(c: char) -> bool {
    let code = c as u32;

    // Auto-generated from full Unicode scan (V2)
    // Total: 9,607 problematic characters in 1,713 ranges
    matches!(code,
'''

for r in ranges:
    rust_code += f'        {r} |\n'

rust_code = rust_code.rstrip(' |\n')
rust_code += '''
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_encoding_basic() {
        // Characters that need encoding
        assert!(needs_special_encoding('\0')); // NULL
        assert!(needs_special_encoding('@'));   // At symbol
        assert!(needs_special_encoding('\u{00A1}'));   // Inverted exclamation
        assert!(needs_special_encoding('\u{00BC}'));   // Fraction
        assert!(needs_special_encoding('\u{2460}'));  // Circled number
        assert!(needs_special_encoding('\u{20AC}'));   // Euro sign
        assert!(needs_special_encoding('\u{1100}'));  // Hangul Jamo
        assert!(needs_special_encoding('\u{3130}'));  // Hangul Compatibility Jamo
    }

    #[test]
    fn test_safe_characters() {
        // ASCII letters and numbers
        assert!(!needs_special_encoding('A'));
        assert!(!needs_special_encoding('z'));
        assert!(!needs_special_encoding('0'));
        assert!(!needs_special_encoding('9'));
        assert!(!needs_special_encoding('!'));

        // Hiragana
        assert!(!needs_special_encoding('\u{3042}')); // あ
        assert!(!needs_special_encoding('\u{3093}')); // ん

        // Katakana
        assert!(!needs_special_encoding('\u{30A2}')); // ア
        assert!(!needs_special_encoding('\u{30F3}')); // ン

        // Kanji
        assert!(!needs_special_encoding('\u{65E5}')); // 日
        assert!(!needs_special_encoding('\u{672C}')); // 本

        // Hangul Syllables (most are safe, but some need encoding)
        assert!(!needs_special_encoding('\u{AC00}')); // 가
    }

    #[test]
    fn test_hangul_jamo() {
        // Hangul Jamo (U+1100-U+11FF) needs encoding
        assert!(needs_special_encoding('\u{1100}'));
        assert!(needs_special_encoding('\u{11FF}'));

        // Hangul Jamo Extended-A (U+A960-U+A97F)
        assert!(needs_special_encoding('\u{A960}'));

        // Hangul Jamo Extended-B (U+D7B0-U+D7FF)
        assert!(needs_special_encoding('\u{D7B0}'));
    }
}
'''

with open('src/char_ranges.rs', 'w', encoding='utf-8') as f:
    f.write(rust_code)

print(f'Generated src/char_ranges.rs with {len(ranges)} ranges')
