// Character Range Discovery Tests
// This test suite systematically tests which characters EzTrans can and cannot handle
// Run with: cargo test --target i686-pc-windows-msvc --test char_range_discovery -- --include-ignored --test-threads=1 --nocapture

use eztrans_rs::EzTransEngine;
use eztrans_rs::char_ranges::is_safe_chars;
use serial_test::serial;
use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;

/// Wrapper to make EzTransEngine usable in static context
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

/// Test if a character can be safely processed by EzTrans
/// Returns true if the character causes issues (needs encoding)
fn test_char_safety(engine: &EzTransEngine, c: char) -> bool {
    // Test 1: Try translating the character alone
    let test_alone = format!("{}", c);
    let alone_wide: Vec<u16> = test_alone
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let translate_mmntw = match engine.translate_mmntw {
        Some(f) => f,
        None => return true, // If function not available, assume unsafe
    };

    // Test the character alone first
    let ret1 = unsafe { translate_mmntw(0, alone_wide.as_ptr()) };

    if ret1.is_null() {
        return true; // Null pointer = character caused problem
    }

    let result_alone = unsafe {
        let len = (0..).find(|&i| *ret1.add(i) == 0).unwrap_or(0);
        let output = String::from_utf16_lossy(&std::slice::from_raw_parts(ret1, len));

        // Free memory
        if let Some(free_mem) = engine.free_mem {
            free_mem(ret1 as *mut std::ffi::c_void);
        }

        output
    };

    // Test 2: Try translating with Japanese text around it
    let test_embedded = format!("あ{}い", c);
    let embedded_wide: Vec<u16> = test_embedded
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let ret2 = unsafe { translate_mmntw(0, embedded_wide.as_ptr()) };

    if ret2.is_null() {
        return true;
    }

    let result_embedded = unsafe {
        let len = (0..).find(|&i| *ret2.add(i) == 0).unwrap_or(0);
        let output = String::from_utf16_lossy(&std::slice::from_raw_parts(ret2, len));

        if let Some(free_mem) = engine.free_mem {
            free_mem(ret2 as *mut std::ffi::c_void);
        }

        output
    };

    // Test 3: Compare with encoded version
    let encoded = engine.hangul_encode(&test_embedded);
    let encoded_wide: Vec<u16> = encoded.encode_utf16().chain(std::iter::once(0)).collect();

    let ret3 = unsafe { translate_mmntw(0, encoded_wide.as_ptr()) };

    if ret3.is_null() {
        return true;
    }

    let result_encoded = unsafe {
        let len = (0..).find(|&i| *ret3.add(i) == 0).unwrap_or(0);
        let output = String::from_utf16_lossy(&std::slice::from_raw_parts(ret3, len));

        if let Some(free_mem) = engine.free_mem {
            free_mem(ret3 as *mut std::ffi::c_void);
        }

        output
    };

    let decoded = engine.hangul_decode(&result_encoded);

    // If results are different, the character needs encoding
    // Also check if the character disappeared or got corrupted
    if result_embedded != decoded {
        return true;
    }

    // Check if the character is preserved in the output
    if !result_alone.contains(c) && !result_embedded.contains(c) {
        // Character disappeared - needs encoding
        return true;
    }

    false
}

#[test]
#[ignore]
#[serial]
fn test_discover_problematic_unicode_ranges() {
    with_engine(|engine| {
        println!("\n=== Testing Unicode Character Ranges ===\n");

        let mut problematic_chars = HashSet::new();
        let mut problematic_ranges = BTreeMap::new();

        // Test ranges to check
        let test_ranges = vec![
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
            (0xAC00, 0xAC0F, "Hangul Syllables (sample)"),
        ];

        for (start, end, name) in test_ranges {
            println!("Testing range: {} (U+{:04X} - U+{:04X})", name, start, end);
            let mut range_problems = Vec::new();

            for code in start..=end {
                if let Some(c) = char::from_u32(code) {
                    if test_char_safety(engine, c) {
                        problematic_chars.insert(c);
                        range_problems.push(c);
                    }
                }
            }

            if !range_problems.is_empty() {
                println!("  ⚠ Found {} problematic characters", range_problems.len());
                problematic_ranges.insert(name, range_problems);
            } else {
                println!("  ✓ All characters safe");
            }
        }

        println!("\n=== Summary ===");
        println!(
            "Total problematic characters found: {}",
            problematic_chars.len()
        );
        println!("\n=== Problematic Ranges ===");

        for (range_name, chars) in &problematic_ranges {
            println!("\n{}:", range_name);
            println!("  Count: {}", chars.len());
            println!(
                "  Characters: {}",
                chars
                    .iter()
                    .take(20)
                    .map(|c| format!("{} (U+{:04X})", c, *c as u32))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if chars.len() > 20 {
                println!("  ... and {} more", chars.len() - 20);
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn test_current_special_chars_necessity() {
    with_engine(|engine| {
        println!("\n=== Testing is_safe_chars Coverage ===\n");

        let mut actually_needed = 0;
        let mut not_needed = 0;
        let mut not_needed_chars = Vec::new();
        let mut tested = 0;

        // Test characters marked as unsafe
        for code in 0x0000u32..=0xFFFF {
            if let Some(c) = char::from_u32(code) {
                if !is_safe_chars(c) {
                    tested += 1;
                    if test_char_safety(engine, c) {
                        actually_needed += 1;
                    } else {
                        not_needed += 1;
                        not_needed_chars.push(c);
                    }

                    // Limit testing to avoid long runtime
                    if tested >= 1000 {
                        break;
                    }
                }
            }
        }

        println!("Total unsafe chars tested: {}", tested);
        println!("Actually needed: {}", actually_needed);
        println!("Not needed: {}", not_needed);

        if !not_needed_chars.is_empty() {
            println!("\n=== Characters marked unsafe but don't need encoding ===");
            for c in not_needed_chars.iter().take(50) {
                println!("  {} (U+{:04X})", c, *c as u32);
            }
            if not_needed_chars.len() > 50 {
                println!("  ... and {} more", not_needed_chars.len() - 50);
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn test_specific_character_sets() {
    with_engine(|engine| {
        println!("\n=== Testing Specific Character Categories ===\n");

        let test_sets = vec![
            (
                "Circled Numbers",
                vec!['①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧', '⑨', '⑩'],
            ),
            ("Circled Letters", vec!['ⓐ', 'ⓑ', 'ⓒ', 'ⓓ', 'ⓔ', 'ⓕ']),
            ("Card Suits", vec!['♠', '♥', '♣', '♦']),
            ("Arrows", vec!['←', '→', '↑', '↓', '↔', '↕']),
            ("Box Drawing", vec!['─', '│', '┌', '┐', '└', '┘']),
            ("Math Symbols", vec!['±', '×', '÷', '≤', '≥', '≠']),
            ("Currency", vec!['$', '€', '¥', '£', '₩']),
            ("Korean Jamo", vec!['ㄱ', 'ㄴ', 'ㄷ', 'ㄹ', 'ㅁ']),
            ("Korean Syllables", vec!['가', '나', '다', '라', '마']),
        ];

        for (name, chars) in test_sets {
            println!("Testing {}", name);
            let problematic: Vec<_> = chars
                .iter()
                .filter(|&&c| test_char_safety(engine, c))
                .collect();

            if problematic.is_empty() {
                println!("  ✓ All characters safe");
            } else {
                println!(
                    "  ⚠ Problematic: {}",
                    problematic
                        .iter()
                        .map(|c| format!("{} (U+{:04X})", c, **c as u32))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn test_generate_optimized_special_chars() {
    with_engine(|engine| {
        println!("\n=== Verifying is_safe_chars Against Additional Ranges ===\n");

        let mut problematic_chars = HashSet::new();

        // Test additional ranges that might be problematic
        let additional_ranges = vec![
            (0x2460, 0x24FF), // Enclosed Alphanumerics
            (0x2500, 0x257F), // Box Drawing
            (0x25A0, 0x25FF), // Geometric Shapes
            (0x2600, 0x26FF), // Miscellaneous Symbols
            (0x3200, 0x32FF), // Enclosed CJK Letters and Months
            (0x3300, 0x33FF), // CJK Compatibility
        ];

        for (start, end) in additional_ranges {
            for code in start..=end {
                if let Some(c) = char::from_u32(code) {
                    // Check if marked safe but actually needs encoding
                    if is_safe_chars(c) && test_char_safety(engine, c) {
                        problematic_chars.insert(c);
                    }
                }
            }
        }

        if problematic_chars.is_empty() {
            println!("\n✓ No problematic characters found in additional ranges!");
        } else {
            println!(
                "\n⚠ Characters marked safe but need encoding: {}",
                problematic_chars.len()
            );

            let mut sorted: Vec<_> = problematic_chars.iter().collect();
            sorted.sort();

            for c in sorted.iter().take(50) {
                println!("  '{}' (U+{:04X})", c, **c as u32);
            }

            if problematic_chars.len() > 50 {
                println!("  ... and {} more", problematic_chars.len() - 50);
            }
        }
    });
}
