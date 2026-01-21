// Analyze problematic character ranges for range-based optimization
// Run with: cargo test --target i686-pc-windows-msvc --test analyze_ranges -- --include-ignored --test-threads=1 --nocapture

use eztrans_rs::EzTransEngine;
use serial_test::serial;
use std::sync::Mutex;

struct EngineWrapper(EzTransEngine);
unsafe impl Send for EngineWrapper {}
unsafe impl Sync for EngineWrapper {}

static ENGINE: Mutex<Option<EngineWrapper>> = Mutex::new(None);

fn get_engine_paths() -> (String, String) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dll_path = format!("{}/../eztrans_dll/J2KEngine.dll", manifest_dir);
    let dat_path = format!("{}/../eztrans_dll/Dat", manifest_dir);
    (dll_path, dat_path)
}

fn with_engine<F, R>(f: F) -> R
where
    F: FnOnce(&EzTransEngine) -> R,
{
    let mut guard = ENGINE.lock().unwrap();
    if guard.is_none() {
        let (dll_path, dat_path) = get_engine_paths();
        let engine = EzTransEngine::new(&dll_path).expect("Failed to load DLL");
        engine
            .initialize_ex("CSUSER123455", &dat_path)
            .expect("Failed to initialize engine");
        *guard = Some(EngineWrapper(engine));
    }
    f(&guard.as_ref().unwrap().0)
}

fn needs_encoding(engine: &EzTransEngine, c: char) -> bool {
    let test_str = format!("あ{}い", c);
    let result1 = engine.translate_mmntw(&test_str);
    let encoded = engine.hangul_encode(&test_str);
    let result2 = engine.translate_mmntw(&encoded);

    if result1.is_err() || result2.is_err() {
        return true;
    }

    let r1 = result1.unwrap();
    let r2_decoded = engine.hangul_decode(&result2.unwrap());

    r1 != r2_decoded
}

/// Find continuous ranges of problematic characters
fn find_continuous_ranges(chars: &[char]) -> Vec<(u32, u32)> {
    if chars.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<u32> = chars.iter().map(|&c| c as u32).collect();
    sorted.sort_unstable();

    let mut ranges = Vec::new();
    let mut range_start = sorted[0];
    let mut range_end = sorted[0];

    for &code in &sorted[1..] {
        if code == range_end + 1 {
            range_end = code;
        } else {
            ranges.push((range_start, range_end));
            range_start = code;
            range_end = code;
        }
    }
    ranges.push((range_start, range_end));

    ranges
}

#[test]
#[ignore]
#[serial]
fn analyze_problematic_ranges() {
    with_engine(|engine| {
        println!("\n=== Analyzing All Problematic Character Ranges ===\n");

        // Test comprehensive unicode ranges
        let test_blocks = vec![
            (0x0000, 0x007F, "Basic Latin"),
            (0x0080, 0x00FF, "Latin-1 Supplement"),
            (0x0100, 0x017F, "Latin Extended-A"),
            (0x0180, 0x024F, "Latin Extended-B"),
            (0x2000, 0x206F, "General Punctuation"),
            (0x2070, 0x209F, "Superscripts and Subscripts"),
            (0x20A0, 0x20CF, "Currency Symbols"),
            (0x2100, 0x214F, "Letterlike Symbols"),
            (0x2150, 0x218F, "Number Forms"),
            (0x2190, 0x21FF, "Arrows"),
            (0x2200, 0x22FF, "Mathematical Operators"),
            (0x2300, 0x23FF, "Miscellaneous Technical"),
            (0x2400, 0x243F, "Control Pictures"),
            (0x2440, 0x245F, "Optical Character Recognition"),
            (0x2460, 0x24FF, "Enclosed Alphanumerics"),
            (0x2500, 0x257F, "Box Drawing"),
            (0x2580, 0x259F, "Block Elements"),
            (0x25A0, 0x25FF, "Geometric Shapes"),
            (0x2600, 0x26FF, "Miscellaneous Symbols"),
            (0x2700, 0x27BF, "Dingbats"),
            (0x3000, 0x303F, "CJK Symbols and Punctuation"),
            (0x3130, 0x318F, "Hangul Compatibility Jamo"),
            (0x3200, 0x32FF, "Enclosed CJK Letters and Months"),
            (0x3300, 0x33FF, "CJK Compatibility"),
        ];

        let mut all_problematic = Vec::new();

        for (start, end, name) in &test_blocks {
            print!("Scanning {} (U+{:04X}-U+{:04X})... ", name, start, end);

            let mut block_problematic = Vec::new();
            for code in *start..=*end {
                if let Some(c) = char::from_u32(code) {
                    if needs_encoding(engine, c) {
                        block_problematic.push(c);
                    }
                }
            }

            println!("{} chars", block_problematic.len());
            all_problematic.extend(block_problematic);
        }

        println!("\n=== Total Problematic Characters: {} ===\n", all_problematic.len());

        // Find continuous ranges
        let ranges = find_continuous_ranges(&all_problematic);

        println!("=== Optimized Continuous Ranges ({}) ===\n", ranges.len());

        for (start, end) in &ranges {
            let start_char = char::from_u32(*start).unwrap_or('?');
            let end_char = char::from_u32(*end).unwrap_or('?');
            let count = end - start + 1;

            if count == 1 {
                println!("  Single: U+{:04X} ('{}')", start, start_char);
            } else if count <= 3 {
                print!("  Small:  U+{:04X}-U+{:04X} (", start, end);
                for code in *start..=*end {
                    if let Some(c) = char::from_u32(code) {
                        print!("'{}' ", c);
                    }
                }
                println!(")");
            } else {
                println!("  Range:  U+{:04X}-U+{:04X} ('{}' to '{}', {} chars)",
                    start, end, start_char, end_char, count);
            }
        }

        // Generate Rust code for range checking
        println!("\n=== Generated Rust Code ===\n");
        println!("/// Check if a character needs encoding based on unicode ranges");
        println!("#[inline]");
        println!("const fn needs_special_encoding(c: char) -> bool {{");
        println!("    let code = c as u32;");
        println!("    matches!(code,");

        for (start, end) in &ranges {
            if start == end {
                println!("        0x{:04X} |", start);
            } else {
                println!("        0x{:04X}..=0x{:04X} |", start, end);
            }
        }

        println!("        _ => false");
        println!("    )");
        println!("}}");

        // Calculate size comparison
        let hashset_size = all_problematic.len() * std::mem::size_of::<char>();
        let range_size = ranges.len() * std::mem::size_of::<(u32, u32)>();

        println!("\n=== Size Comparison ===");
        println!("HashSet approach: {} bytes ({} chars × {} bytes)",
            hashset_size, all_problematic.len(), std::mem::size_of::<char>());
        println!("Range approach:   {} bytes ({} ranges × {} bytes)",
            range_size, ranges.len(), std::mem::size_of::<(u32, u32)>());
        println!("Savings: {} bytes ({:.1}%)",
            hashset_size.saturating_sub(range_size),
            (1.0 - range_size as f64 / hashset_size as f64) * 100.0);
    });
}

#[test]
#[ignore]
#[serial]
fn test_practical_vs_comprehensive() {
    with_engine(|engine| {
        println!("\n=== Practical vs Comprehensive Comparison ===\n");

        // Test some realistic Japanese text with various special characters
        let test_cases = vec![
            "今日の天気は①晴れ、②曇り、③雨です。",
            "価格は¥1,000です。",
            "面積は100㎡です。",
            "温度は30℃です。",
            "割合は½です。",
            "記号→矢印←です。",
            "\x00制御文字テスト\x01",  // Control characters
            "数学記号∑∏∫",
            "OCR記号⑇⑈⑉",
        ];

        for (i, text) in test_cases.iter().enumerate() {
            println!("Test case {}: {:?}", i + 1, text);

            match engine.default_translate(text) {
                Ok(result) => println!("  ✓ Translated: {}", result),
                Err(e) => println!("  ✗ Error: {:?}", e),
            }
        }

        println!("\n=== Recommendation ===");
        println!("For practical use cases (actual translation text):");
        println!("  - Use current 341-char HashSet approach");
        println!("  - Covers all commonly used special characters");
        println!("  - Fast lookup, minimal memory");
        println!();
        println!("For 100% coverage (including control chars, rare symbols):");
        println!("  - Use range-based approach (generated above)");
        println!("  - Covers all 2,969 problematic characters");
        println!("  - More efficient than large HashSet");
    });
}
