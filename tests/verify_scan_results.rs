// Verify Unicode Scan Results
// This test validates the results from full_unicode_scan_v2_results.txt
// Run with: cargo test --target i686-pc-windows-msvc --test verify_scan_results -- --nocapture --test-threads=1

use eztrans_rs::EzTransEngine;
use serial_test::serial;
use std::collections::HashSet;
use std::fs;
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

/// Check if a character needs special encoding
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

/// Parse the scan results file and extract problematic character codes
fn parse_scan_results(content: &str) -> HashSet<u32> {
    let mut chars = HashSet::new();

    for line in content.lines() {
        // Format: "U+XXXX 'c'"
        if line.starts_with("U+") {
            if let Some(hex_part) = line.strip_prefix("U+") {
                if let Some(code_str) = hex_part.split_whitespace().next() {
                    if let Ok(code) = u32::from_str_radix(code_str, 16) {
                        chars.insert(code);
                    }
                }
            }
        }
    }

    chars
}

#[test]
#[serial]
fn verify_problematic_chars_sample() {
    println!("\n=== VERIFYING SCAN RESULTS (Sample) ===\n");

    // Read the scan results file
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let results_path = format!("{}/full_unicode_scan_v2_results.txt", manifest_dir);

    let content = match fs::read_to_string(&results_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to read results file: {}", e);
            println!("Path tried: {}", results_path);
            return;
        }
    };

    let problematic_chars = parse_scan_results(&content);
    println!("Parsed {} problematic characters from results file", problematic_chars.len());

    // Sample some characters to verify
    let sample_chars: Vec<u32> = vec![
        // From the results file (should be problematic)
        0x0000, // NULL
        0x0040, // @
        0x00A1, // ¡
        0x00BC, // ¼
        0x0111, // đ
        0x1100, // ᄀ (Hangul Jamo)
        0x3130, // ㄰ (Hangul Compatibility Jamo)
        0xAC00, // 가 (Hangul syllable - check if problematic)
        0xFF02, // ＂

        // Characters NOT in results (should work fine)
        0x0041, // A (Latin A)
        0x0042, // B
        0x3042, // あ (Hiragana A)
        0x30A2, // ア (Katakana A)
        0x4E00, // 一 (CJK)
    ];

    with_engine(|engine| {
        println!("\n--- Testing Sample Characters ---\n");

        let mut correct = 0;
        let mut incorrect = 0;
        let mut errors = Vec::new();

        for &code in &sample_chars {
            if let Some(c) = char::from_u32(code) {
                let actual_needs_encoding = needs_encoding(engine, c);
                let expected_needs_encoding = problematic_chars.contains(&code);

                let status = if actual_needs_encoding == expected_needs_encoding {
                    correct += 1;
                    "OK"
                } else {
                    incorrect += 1;
                    errors.push((code, c, expected_needs_encoding, actual_needs_encoding));
                    "MISMATCH"
                };

                println!("U+{:04X} '{}': expected={}, actual={} [{}]",
                    code, c, expected_needs_encoding, actual_needs_encoding, status);
            }
        }

        println!("\n--- Sample Test Summary ---");
        println!("Correct: {}", correct);
        println!("Incorrect: {}", incorrect);

        if !errors.is_empty() {
            println!("\nMismatches found:");
            for (code, c, expected, actual) in &errors {
                println!("  U+{:04X} '{}': file says {}, but test shows {}", code, c, expected, actual);
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn verify_all_problematic_chars() {
    println!("\n=== VERIFYING ALL PROBLEMATIC CHARS ===\n");

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let results_path = format!("{}/full_unicode_scan_v2_results.txt", manifest_dir);

    let content = match fs::read_to_string(&results_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to read results file: {}", e);
            return;
        }
    };

    let problematic_chars = parse_scan_results(&content);
    println!("Verifying {} problematic characters...", problematic_chars.len());

    with_engine(|engine| {
        let mut correct = 0;
        let mut false_positive = 0; // File says problematic but test says OK
        let mut total_tested = 0;
        let mut false_positive_list = Vec::new();

        for &code in &problematic_chars {
            if let Some(c) = char::from_u32(code) {
                total_tested += 1;
                let actual_needs_encoding = needs_encoding(engine, c);

                if actual_needs_encoding {
                    correct += 1;
                } else {
                    false_positive += 1;
                    if false_positive_list.len() < 50 {
                        false_positive_list.push((code, c));
                    }
                }

                if total_tested % 500 == 0 {
                    print!("\rProgress: {} / {} (false positives: {})   ",
                        total_tested, problematic_chars.len(), false_positive);
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
            }
        }

        println!("\n\n--- Verification Results ---");
        println!("Total tested: {}", total_tested);
        println!("Confirmed problematic: {}", correct);
        println!("False positives (file says problematic but test OK): {}", false_positive);
        println!("Accuracy: {:.2}%", (correct as f64 / total_tested as f64) * 100.0);

        if !false_positive_list.is_empty() {
            println!("\nSample false positives (first 50):");
            for (code, c) in &false_positive_list {
                println!("  U+{:04X} '{}'", code, c);
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn verify_non_problematic_sample() {
    println!("\n=== VERIFYING NON-PROBLEMATIC CHARS (Sample) ===\n");

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let results_path = format!("{}/full_unicode_scan_v2_results.txt", manifest_dir);

    let content = match fs::read_to_string(&results_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to read results file: {}", e);
            return;
        }
    };

    let problematic_chars = parse_scan_results(&content);

    // Sample non-problematic characters (not in the file)
    let test_ranges = vec![
        (0x0041, 0x005A), // A-Z
        (0x0061, 0x007A), // a-z
        (0x0030, 0x0039), // 0-9
        (0x3040, 0x309F), // Hiragana
        (0x30A0, 0x30FF), // Katakana
        (0x4E00, 0x4E50), // CJK (sample)
    ];

    with_engine(|engine| {
        let mut correct = 0;
        let mut false_negative = 0; // File says OK but test says problematic
        let mut total_tested = 0;
        let mut false_negative_list = Vec::new();

        for (start, end) in &test_ranges {
            for code in *start..=*end {
                if problematic_chars.contains(&code) {
                    continue; // Skip if already marked as problematic
                }

                if let Some(c) = char::from_u32(code) {
                    total_tested += 1;
                    let actual_needs_encoding = needs_encoding(engine, c);

                    if !actual_needs_encoding {
                        correct += 1;
                    } else {
                        false_negative += 1;
                        if false_negative_list.len() < 50 {
                            false_negative_list.push((code, c));
                        }
                    }
                }
            }
        }

        println!("--- Non-Problematic Verification Results ---");
        println!("Total tested: {}", total_tested);
        println!("Confirmed OK: {}", correct);
        println!("False negatives (file says OK but actually problematic): {}", false_negative);
        println!("Accuracy: {:.2}%", (correct as f64 / total_tested as f64) * 100.0);

        if !false_negative_list.is_empty() {
            println!("\nFalse negatives found:");
            for (code, c) in &false_negative_list {
                println!("  U+{:04X} '{}'", code, c);
            }
        }
    });
}

#[test]
#[serial]
fn verify_scan_statistics() {
    println!("\n=== VERIFYING SCAN STATISTICS ===\n");

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let results_path = format!("{}/full_unicode_scan_v2_results.txt", manifest_dir);

    let content = match fs::read_to_string(&results_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to read results file: {}", e);
            return;
        }
    };

    let problematic_chars = parse_scan_results(&content);

    // Extract statistics from file header
    let mut total_tested_from_file: Option<u32> = None;
    let mut problematic_from_file: Option<u32> = None;

    for line in content.lines() {
        if line.starts_with("Total tested:") {
            if let Some(num_str) = line.strip_prefix("Total tested:") {
                total_tested_from_file = num_str.trim().parse().ok();
            }
        }
        if line.starts_with("Problematic:") {
            if let Some(num_str) = line.strip_prefix("Problematic:") {
                problematic_from_file = num_str.trim().parse().ok();
            }
        }
    }

    println!("File header claims:");
    println!("  Total tested: {:?}", total_tested_from_file);
    println!("  Problematic: {:?}", problematic_from_file);

    println!("\nActual parsed:");
    println!("  Problematic chars parsed: {}", problematic_chars.len());

    // Verify counts match
    if let Some(expected) = problematic_from_file {
        if problematic_chars.len() as u32 == expected {
            println!("\n✓ Character count matches!");
        } else {
            println!("\n✗ Character count mismatch!");
            println!("  Expected: {}, Actual: {}", expected, problematic_chars.len());
        }
    }

    // Analyze character distribution
    println!("\n--- Character Distribution ---");

    let mut bmp_count = 0;
    let mut supplementary_count = 0;
    let mut hangul_jamo = 0;
    let mut hangul_compat = 0;
    let mut hangul_syllable = 0;

    for &code in &problematic_chars {
        if code <= 0xFFFF {
            bmp_count += 1;
        } else {
            supplementary_count += 1;
        }

        match code {
            0x1100..=0x11FF => hangul_jamo += 1,
            0x3130..=0x318F => hangul_compat += 1,
            0xAC00..=0xD7AF => hangul_syllable += 1,
            0xA960..=0xA97F => hangul_jamo += 1,
            0xD7B0..=0xD7FF => hangul_jamo += 1,
            _ => {}
        }
    }

    println!("BMP (U+0000..U+FFFF): {}", bmp_count);
    println!("Supplementary (U+10000+): {}", supplementary_count);
    println!("\nHangul breakdown:");
    println!("  Jamo (1100-11FF, A960-A97F, D7B0-D7FF): {}", hangul_jamo);
    println!("  Compatibility Jamo (3130-318F): {}", hangul_compat);
    println!("  Syllables (AC00-D7AF): {}", hangul_syllable);
}
