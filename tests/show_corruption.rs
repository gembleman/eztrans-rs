// Show how characters get corrupted
// Run with: cargo test --target i686-pc-windows-msvc --test show_corruption -- --nocapture --test-threads=1

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

fn show_hex(s: &str) -> String {
    s.chars()
        .map(|c| format!("U+{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
#[serial]
fn show_corruption_examples() {
    println!("\n=== HOW CHARACTERS GET CORRUPTED ===\n");

    with_engine(|engine| {
        // Test characters that cause problems
        let problem_chars = vec![
            ('@', "At sign"),
            ('¡', "Inverted exclamation"),
            ('¼', "Fraction 1/4"),
            ('ᄀ', "Hangul Jamo (ㄱ sound)"),
            ('㄰', "Hangul Compatibility Jamo"),
            ('＂', "Fullwidth quotation"),
            ('Ω', "Greek Omega"),
        ];

        println!("=== PROBLEMATIC CHARACTERS ===\n");

        for (c, desc) in &problem_chars {
            let test_str = format!("あ{}い", c);

            println!("--- {} (U+{:04X}) '{}' ---", desc, *c as u32, c);
            println!("Input:        \"{}\"", test_str);
            println!("Input hex:    {}", show_hex(&test_str));

            // Direct translation (without encoding)
            let result1 = engine.translate_mmntw(&test_str);
            match &result1 {
                Ok(r) => {
                    println!("Direct:       \"{}\"", r);
                    println!("Direct hex:   {}", show_hex(r));
                }
                Err(e) => println!("Direct:       ERROR {:?}", e),
            }

            // With hangul encoding
            let encoded = engine.hangul_encode(&test_str);
            println!("Encoded:      \"{}\"", encoded);
            println!("Encoded hex:  {}", show_hex(&encoded));

            let result2 = engine.translate_mmntw(&encoded);
            match &result2 {
                Ok(r) => {
                    println!("Trans(enc):   \"{}\"", r);
                    let decoded = engine.hangul_decode(r);
                    println!("Decoded:      \"{}\"", decoded);
                    println!("Decoded hex:  {}", show_hex(&decoded));
                }
                Err(e) => println!("Trans(enc):   ERROR {:?}", e),
            }

            // Compare
            if let (Ok(r1), Ok(r2)) = (&result1, &result2) {
                let decoded = engine.hangul_decode(r2);
                if r1 == &decoded {
                    println!("Result:       SAME (no corruption)");
                } else {
                    println!("Result:       DIFFERENT! Character was corrupted!");
                }
            }
            println!();
        }

        // Now show normal characters that work fine
        let normal_chars = vec![
            ('A', "Latin A"),
            ('あ', "Hiragana A"),
            ('ア', "Katakana A"),
            ('가', "Hangul syllable GA"),
            ('一', "CJK character"),
            ('!', "Exclamation"),
        ];

        println!("\n=== NORMAL CHARACTERS (no corruption) ===\n");

        for (c, desc) in &normal_chars {
            let test_str = format!("あ{}い", c);

            println!("--- {} (U+{:04X}) '{}' ---", desc, *c as u32, c);
            println!("Input:        \"{}\"", test_str);

            let result1 = engine.translate_mmntw(&test_str);
            let encoded = engine.hangul_encode(&test_str);
            let result2 = engine.translate_mmntw(&encoded);

            if let (Ok(r1), Ok(r2)) = (&result1, &result2) {
                let decoded = engine.hangul_decode(r2);
                println!("Direct:       \"{}\"", r1);
                println!("Via encode:   \"{}\"", decoded);
                if r1 == &decoded {
                    println!("Result:       SAME (OK)");
                } else {
                    println!("Result:       DIFFERENT!");
                }
            }
            println!();
        }
    });
}

#[test]
#[serial]
fn show_hangul_jamo_issue() {
    println!("\n=== HANGUL JAMO CORRUPTION DETAIL ===\n");

    with_engine(|engine| {
        // Hangul Jamo vs Hangul Syllable
        println!("Hangul Jamo (ᄀ U+1100) vs Hangul Syllable (가 U+AC00)\n");

        // Jamo - problematic
        let jamo = 'ᄀ'; // U+1100
        let test_jamo = format!("テスト{}です", jamo);
        println!("=== Jamo ᄀ (U+1100) ===");
        println!("Input: \"{}\"", test_jamo);

        let r1 = engine.translate_mmntw(&test_jamo);
        println!("Direct translation: {:?}", r1);

        let enc = engine.hangul_encode(&test_jamo);
        println!("After hangul_encode: \"{}\"", enc);

        let r2 = engine.translate_mmntw(&enc);
        println!("Translation of encoded: {:?}", r2);

        if let Ok(r2_str) = &r2 {
            let dec = engine.hangul_decode(r2_str);
            println!("After hangul_decode: \"{}\"", dec);
        }

        println!();

        // Syllable - OK
        let syllable = '가'; // U+AC00
        let test_syllable = format!("テスト{}です", syllable);
        println!("=== Syllable 가 (U+AC00) ===");
        println!("Input: \"{}\"", test_syllable);

        let r1 = engine.translate_mmntw(&test_syllable);
        println!("Direct translation: {:?}", r1);

        let enc = engine.hangul_encode(&test_syllable);
        println!("After hangul_encode: \"{}\"", enc);

        let r2 = engine.translate_mmntw(&enc);
        println!("Translation of encoded: {:?}", r2);

        if let Ok(r2_str) = &r2 {
            let dec = engine.hangul_decode(r2_str);
            println!("After hangul_decode: \"{}\"", dec);
        }
    });
}
