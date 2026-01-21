// Simplified Character Range Tests for Optimization
// Run with: cargo test --target i686-pc-windows-msvc --test char_optimization -- --include-ignored --test-threads=1 --nocapture

use eztrans_rs::EzTransEngine;
use eztrans_rs::char_ranges::is_safe_chars;
use serial_test::serial;
use std::collections::HashSet;
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

/// Quick check if character needs encoding by comparing encoded vs non-encoded results
fn needs_encoding(engine: &EzTransEngine, c: char) -> bool {
    let test_str = format!("あ{}い", c);

    // Try normal translation
    let result1 = engine.translate_mmntw(&test_str);

    // Try with encoding
    let encoded = engine.hangul_encode(&test_str);
    let result2 = engine.translate_mmntw(&encoded);

    if result1.is_err() || result2.is_err() {
        return true;
    }

    let r1 = result1.unwrap();
    let r2_decoded = engine.hangul_decode(&result2.unwrap());

    // If results differ, encoding is needed
    r1 != r2_decoded
}

#[test]
#[ignore]
#[serial]
fn test_verify_current_special_chars() {
    with_engine(|engine| {
        println!("\n=== Verifying is_safe_chars Coverage ===\n");

        let mut needed = 0;
        let mut not_needed = Vec::new();
        let mut tested = 0;

        // Test unsafe characters (those not in is_safe_chars)
        for code in 0x0000u32..=0xFFFF {
            if let Some(c) = char::from_u32(code) {
                if !is_safe_chars(c) {
                    tested += 1;
                    if needs_encoding(engine, c) {
                        needed += 1;
                    } else {
                        not_needed.push(c);
                    }

                    // Limit testing to avoid long runtime
                    if tested >= 1000 {
                        break;
                    }
                }
            }
        }

        println!("Total unsafe chars tested: {}", tested);
        println!("Actually needed encoding: {}", needed);
        println!("Not needed encoding: {}", not_needed.len());

        if !not_needed.is_empty() {
            println!("\nCharacters marked unsafe but don't need encoding:");
            for c in not_needed.iter().take(20) {
                println!("  '{}' (U+{:04X})", c, *c as u32);
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn test_sample_unicode_blocks() {
    with_engine(|engine| {
        println!("\n=== Sampling Unicode Blocks ===\n");

        // Sample a few characters from each block instead of testing all
        let sample_chars = vec![
            // Basic Latin
            ('A', "Basic Latin Letter"),
            ('@', "At Sign"),
            ('$', "Dollar Sign"),

            // Latin-1 Supplement
            ('©', "Copyright"),
            ('®', "Registered"),
            ('°', "Degree"),
            ('±', "Plus-Minus"),
            ('×', "Multiplication"),
            ('÷', "Division"),

            // Number Forms
            ('½', "One Half"),
            ('¼', "One Quarter"),
            ('¾', "Three Quarters"),

            // Currency
            ('€', "Euro"),
            ('£', "Pound"),
            ('¥', "Yen"),
            ('₩', "Won"),

            // Arrows
            ('←', "Left Arrow"),
            ('→', "Right Arrow"),
            ('↑', "Up Arrow"),
            ('↓', "Down Arrow"),
            ('↔', "Left-Right Arrow"),

            // Enclosed Numbers
            ('①', "Circled 1"),
            ('②', "Circled 2"),
            ('⑩', "Circled 10"),

            // Enclosed Letters
            ('ⓐ', "Circled a"),
            ('Ⓐ', "Circled A"),

            // Box Drawing
            ('─', "Box Horizontal"),
            ('│', "Box Vertical"),
            ('┌', "Box Down-Right"),
            ('└', "Box Up-Right"),

            // Geometric Shapes
            ('■', "Black Square"),
            ('●', "Black Circle"),
            ('▲', "Black Triangle Up"),

            // Symbols
            ('☆', "White Star"),
            ('★', "Black Star"),
            ('♠', "Spade"),
            ('♥', "Heart"),
            ('♣', "Club"),
            ('♦', "Diamond"),

            // Hangul Jamo
            ('ㄱ', "Hangul Jamo G"),
            ('ㄴ', "Hangul Jamo N"),
            ('ㅏ', "Hangul Jamo A"),

            // CJK Compatibility
            ('㎕', "Micro-liter"),
            ('㎖', "Milli-liter"),
            ('㎞', "Kilo-meter"),
            ('㎡', "Square meter"),

            // Enclosed CJK
            ('㈀', "Parenthesized Hangul Kiyeok"),
            ('㉠', "Circled Hangul Kiyeok"),
        ];

        let mut problematic = Vec::new();
        let mut safe = Vec::new();

        for (c, desc) in sample_chars {
            if needs_encoding(engine, c) {
                problematic.push((c, desc));
            } else {
                safe.push((c, desc));
            }
        }

        println!("=== Safe Characters ({}) ===", safe.len());
        for (c, desc) in safe {
            println!("  '{}' (U+{:04X}) - {}", c, c as u32, desc);
        }

        println!("\n=== Problematic Characters ({}) ===", problematic.len());
        for (c, desc) in problematic {
            println!("  '{}' (U+{:04X}) - {}", c, c as u32, desc);
        }
    });
}

#[test]
#[ignore]
#[serial]
fn test_find_missing_chars() {
    with_engine(|engine| {
        println!("\n=== Finding Characters Needing Encoding ===\n");

        // Test ranges that should be problematic
        let test_ranges = vec![
            (0x2460, 0x24FF, "Enclosed Alphanumerics"),
            (0x3200, 0x32FF, "Enclosed CJK"),
            (0x3300, 0x33FF, "CJK Compatibility"),
        ];

        let mut needs_encoding_list = Vec::new();

        for (start, end, name) in test_ranges {
            println!("Checking {} (U+{:04X}-U+{:04X})...", name, start, end);

            // Sample every 4th character to speed up
            for code in (start..=end).step_by(4) {
                if let Some(c) = char::from_u32(code) {
                    if needs_encoding(engine, c) && is_safe_chars(c) {
                        needs_encoding_list.push((c, name));
                    }
                }
            }
        }

        if needs_encoding_list.is_empty() {
            println!("\n✓ No characters marked safe but need encoding!");
        } else {
            println!("\n⚠ Characters marked safe but need encoding: {}", needs_encoding_list.len());
            for (c, range) in &needs_encoding_list {
                println!("  '{}' (U+{:04X}) from {}", c, *c as u32, range);
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn test_optimize_special_chars() {
    with_engine(|engine| {
        println!("\n=== Verifying is_safe_chars Optimization ===\n");

        let mut safe_but_needs_encoding = HashSet::new();
        let mut unsafe_and_needs_encoding = HashSet::new();
        let mut tested = 0;

        // Step 1: Test characters to verify is_safe_chars accuracy
        println!("Step 1: Testing character ranges...");
        for code in 0x0000u32..=0xFFFF {
            if let Some(c) = char::from_u32(code) {
                if needs_encoding(engine, c) {
                    if is_safe_chars(c) {
                        safe_but_needs_encoding.insert(c);
                    } else {
                        unsafe_and_needs_encoding.insert(c);
                    }
                }
                tested += 1;

                // Sample testing to avoid long runtime
                if tested % 100 == 0 && tested >= 5000 {
                    break;
                }
            }
        }

        let filtered_count = unsafe_and_needs_encoding.len();
        println!("  Unsafe chars needing encoding: {}", filtered_count);
        println!("  Safe chars needing encoding: {}", safe_but_needs_encoding.len());

        // Step 2: Report findings
        println!("\nStep 2: Analysis results");

        if !safe_but_needs_encoding.is_empty() {
            println!("\n⚠ Characters marked safe but need encoding:");
            for c in safe_but_needs_encoding.iter().take(20) {
                println!("  '{}' (U+{:04X})", c, *c as u32);
            }
            if safe_but_needs_encoding.len() > 20 {
                println!("  ... and {} more", safe_but_needs_encoding.len() - 20);
            }
        } else {
            println!("\n✓ No false positives found!");
        }

        println!("\nTotal characters tested: {}", tested);
        println!("Accuracy: {:.2}%",
            100.0 * (1.0 - (safe_but_needs_encoding.len() as f64 / tested as f64)))
    });
}
