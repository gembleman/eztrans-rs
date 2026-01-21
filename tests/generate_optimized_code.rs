// Generate optimized special_chars code
// Run with: cargo test --target i686-pc-windows-msvc --test generate_optimized_code -- --include-ignored --test-threads=1 --nocapture

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

#[test]
#[ignore]
#[serial]
fn generate_optimized_special_chars_code() {
    with_engine(|engine| {
        println!("\n=== Verifying is_safe_chars Function ===\n");

        let mut needs_encoding_chars = HashSet::new();
        let mut safe_chars = HashSet::new();

        // Test a subset of Unicode to verify is_safe_chars accuracy
        let test_ranges = vec![
            (0x0020u32, 0x007F), // Basic Latin
            (0x00A0, 0x00FF),    // Latin-1 Supplement
            (0x2000, 0x206F),    // General Punctuation
            (0x2100, 0x21FF),    // Letterlike & Arrows
            (0x3000, 0x303F),    // CJK Symbols
        ];

        for (start, end) in test_ranges {
            for code in start..=end {
                if let Some(c) = char::from_u32(code) {
                    if needs_encoding(engine, c) {
                        needs_encoding_chars.insert(c);
                    } else {
                        safe_chars.insert(c);
                    }
                }
            }
        }

        println!("Characters that need encoding: {}", needs_encoding_chars.len());
        println!("Characters that are safe: {}", safe_chars.len());

        // Check is_safe_chars accuracy
        let mut false_positives = Vec::new();
        let mut false_negatives = Vec::new();

        for &c in &safe_chars {
            if !is_safe_chars(c) {
                false_negatives.push(c);
            }
        }

        for &c in &needs_encoding_chars {
            if is_safe_chars(c) {
                false_positives.push(c);
            }
        }

        if !false_positives.is_empty() {
            println!("\nFalse positives (marked safe but needs encoding):");
            for c in false_positives.iter().take(20) {
                println!("  '{}' (U+{:04X})", c, *c as u32);
            }
        }

        if !false_negatives.is_empty() {
            println!("\nFalse negatives (marked unsafe but is safe):");
            for c in false_negatives.iter().take(20) {
                println!("  '{}' (U+{:04X})", c, *c as u32);
            }
        }

        println!("\n=== Statistics ===");
        println!("False positives: {}", false_positives.len());
        println!("False negatives: {}", false_negatives.len());
    });
}

#[test]
#[ignore]
#[serial]
fn analyze_character_categories() {
    with_engine(|_engine| {
        println!("\n=== Analyzing is_safe_chars Coverage ===\n");

        // Helper to categorize
        let categorize = |code: u32| -> &'static str {
            match code {
                0x0000..=0x007F => "Basic Latin",
                0x0080..=0x00FF => "Latin-1 Supplement",
                0x0100..=0x017F => "Latin Extended-A",
                0x0180..=0x024F => "Latin Extended-B",
                0x2000..=0x206F => "General Punctuation",
                0x2070..=0x209F => "Superscripts/Subscripts",
                0x20A0..=0x20CF => "Currency Symbols",
                0x2100..=0x214F => "Letterlike Symbols",
                0x2150..=0x218F => "Number Forms",
                0x2190..=0x21FF => "Arrows",
                0x2200..=0x22FF => "Math Operators",
                0x2300..=0x23FF => "Misc Technical",
                0x2400..=0x243F => "Control Pictures",
                0x2440..=0x245F => "OCR",
                0x2460..=0x24FF => "Enclosed Alphanumerics",
                0x2500..=0x257F => "Box Drawing",
                0x2580..=0x259F => "Block Elements",
                0x25A0..=0x25FF => "Geometric Shapes",
                0x2600..=0x26FF => "Misc Symbols",
                0x2700..=0x27BF => "Dingbats",
                0x3000..=0x303F => "CJK Symbols/Punctuation",
                0x3130..=0x318F => "Hangul Compatibility Jamo",
                0x3200..=0x32FF => "Enclosed CJK",
                0x3300..=0x33FF => "CJK Compatibility",
                _ => "Other",
            }
        };

        use std::collections::HashMap;
        let mut blocks: HashMap<String, Vec<char>> = HashMap::new();

        // Scan Unicode range to categorize safe characters
        for code in 0x0000u32..=0xFFFF {
            if let Some(c) = char::from_u32(code) {
                if is_safe_chars(c) {
                    let category = categorize(code);
                    blocks.entry(category.to_string())
                        .or_insert_with(Vec::new)
                        .push(c);
                }
            }
        }

        let mut sorted_blocks: Vec<_> = blocks.into_iter().collect();
        sorted_blocks.sort_by_key(|(name, _)| name.clone());

        println!("Safe characters by Unicode Block:\n");
        let mut total = 0;
        for (block_name, mut chars) in sorted_blocks {
            chars.sort();
            total += chars.len();
            println!("{}: {} characters", block_name, chars.len());
            if chars.len() <= 20 {
                print!("  ");
                for c in &chars {
                    print!("'{}' ", c);
                }
                println!();
            } else {
                print!("  Sample: ");
                for c in chars.iter().take(10) {
                    print!("'{}' ", c);
                }
                println!("... (+{} more)", chars.len() - 10);
            }
        }
        println!("\nTotal safe characters: {}", total);
    });
}
