// EzTransEngine Tests
// Note: These tests require the EzTrans DLL (32-bit) to be present.
// Run with: cargo test --target i686-pc-windows-msvc --test engine_tests -- --include-ignored --test-threads=1

use eztrans_rs::EzTransEngine;
use serial_test::serial;
use std::sync::Mutex;

/// Wrapper to make EzTransEngine usable in static context
/// SAFETY: EzTrans DLL operations are only safe in single-threaded context,
/// which is enforced by #[serial] attribute and --test-threads=1
struct EngineWrapper(EzTransEngine);
unsafe impl Send for EngineWrapper {}
unsafe impl Sync for EngineWrapper {}

/// Global engine instance - initialized once and reused across all tests
/// This is necessary because EzTrans DLL has global state and doesn't handle
/// multiple initialize/terminate cycles well within a single process.
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

// ============================================
// Engine Initialization Tests
// ============================================

#[test]
#[ignore]
#[serial]
fn test_engine_new() {
    with_engine(|engine| {
        // If we got here, the engine was created successfully
        assert!(!engine.module.is_invalid());
    });
}

#[test]
fn test_engine_new_invalid_path() {
    let result = EzTransEngine::new("invalid/path/to/dll.dll");
    assert!(result.is_err());
}

#[test]
#[ignore]
#[serial]
fn test_engine_initialize_ex() {
    with_engine(|engine| {
        // Engine is already initialized via with_engine()
        // Verify engine has initialize_ex function loaded
        assert!(engine.initialize_ex.is_some());
    });
}

#[test]
#[ignore]
#[serial]
fn test_engine_has_required_functions() {
    with_engine(|engine| {
        // Check essential functions are loaded
        assert!(engine.initialize_ex.is_some() || engine.initialize.is_some());
        assert!(engine.terminate.is_some());
        assert!(engine.translate_mmntw.is_some() || engine.translate_mmnt.is_some());
        assert!(engine.free_mem.is_some());
    });
}

// ============================================
// Hangul Encode/Decode Tests
// ============================================

#[test]
#[ignore]
#[serial]
fn test_hangul_encode_basic() {
    with_engine(|engine| {
        // Test Korean character encoding
        let input = "안녕하세요";
        let encoded = engine.hangul_encode(input);

        // Should contain +x prefixes for Korean characters
        assert!(encoded.contains("+x"));
        assert!(!encoded.contains("안"));
    });
}

#[test]
#[ignore]
#[serial]
fn test_hangul_encode_mixed() {
    with_engine(|engine| {
        // Test mixed content (Korean + Japanese + ASCII)
        let input = "Hello 안녕 こんにちは";
        let encoded = engine.hangul_encode(input);

        // ASCII and Japanese should remain unchanged
        assert!(encoded.contains("Hello"));
        assert!(encoded.contains("こんにちは"));
        // Korean should be encoded
        assert!(!encoded.contains("안녕"));
    });
}

#[test]
#[ignore]
#[serial]
fn test_hangul_encode_at_symbol() {
    with_engine(|engine| {
        // @ symbol should be encoded
        let input = "test@example.com";
        let encoded = engine.hangul_encode(input);

        assert!(encoded.contains("+x0040")); // @ = U+0040
    });
}

#[test]
#[ignore]
#[serial]
fn test_hangul_encode_special_chars() {
    with_engine(|engine| {
        // Special characters from the special_chars set
        let input = "테스트♥심볼♠";
        let encoded = engine.hangul_encode(input);

        // Should encode both Korean and special symbols
        assert!(encoded.contains("+X")); // Special chars use +X
    });
}

#[test]
#[ignore]
#[serial]
fn test_hangul_decode_basic() {
    with_engine(|engine| {
        // Test decoding
        let encoded = "+xC548+xB155"; // 안녕
        let decoded = engine.hangul_decode(encoded);

        assert_eq!(decoded, "안녕");
    });
}

#[test]
#[ignore]
#[serial]
fn test_hangul_encode_decode_roundtrip() {
    with_engine(|engine| {
        let original = "안녕하세요 Hello 世界";
        let encoded = engine.hangul_encode(original);
        let decoded = engine.hangul_decode(&encoded);

        assert_eq!(decoded, original);
    });
}

#[test]
#[ignore]
#[serial]
fn test_hangul_decode_invalid_hex() {
    with_engine(|engine| {
        // Invalid hex should be preserved
        let input = "+xGGGG test";
        let decoded = engine.hangul_decode(input);

        assert_eq!(decoded, "+xGGGG test");
    });
}

#[test]
#[ignore]
#[serial]
fn test_hangul_decode_incomplete() {
    with_engine(|engine| {
        // Incomplete sequence should be preserved
        let input = "+x12 test";
        let decoded = engine.hangul_decode(input);

        assert_eq!(decoded, "+x12 test");
    });
}

// ============================================
// is_hangul_range Tests
// ============================================

#[test]
#[ignore]
#[serial]
fn test_is_hangul_range_syllables() {
    with_engine(|engine| {
        // Hangul Syllables block (AC00-D7A3)
        assert!(engine.is_hangul_range('가' as u32)); // U+AC00
        assert!(engine.is_hangul_range('힣' as u32)); // U+D7A3
    });
}

#[test]
#[ignore]
#[serial]
fn test_is_hangul_range_jamo() {
    with_engine(|engine| {
        // Hangul Jamo block (1100-11FF)
        assert!(engine.is_hangul_range(0x1100));
        assert!(engine.is_hangul_range(0x11FF));

        // Hangul Compatibility Jamo (3130-318F)
        assert!(engine.is_hangul_range(0x3130));
        assert!(engine.is_hangul_range(0x318F));
    });
}

#[test]
#[ignore]
#[serial]
fn test_is_hangul_range_non_hangul() {
    with_engine(|engine| {
        // ASCII
        assert!(!engine.is_hangul_range('A' as u32));
        assert!(!engine.is_hangul_range('z' as u32));

        // Japanese
        assert!(!engine.is_hangul_range('あ' as u32));
        assert!(!engine.is_hangul_range('漢' as u32));
    });
}

// ============================================
// Translation Tests
// ============================================

#[test]
#[ignore]
#[serial]
fn test_translate_mmntw() {
    with_engine(|engine| {
        let input = "おはようございます。";
        let result = engine.translate_mmntw(input);

        assert!(result.is_ok(), "Translation failed: {:?}", result.err());
        let translated = result.unwrap();
        assert!(!translated.is_empty());
        assert_ne!(translated, input);
    });
}

#[test]
#[ignore]
#[serial]
fn test_translate_mmnt() {
    with_engine(|engine| {
        let input = "こんにちは。";
        let result = engine.translate_mmnt(input);

        assert!(result.is_ok(), "Translation failed: {:?}", result.err());
        let translated = result.unwrap();
        assert!(!translated.is_empty());
    });
}

#[test]
#[ignore]
#[serial]
fn test_default_translate() {
    with_engine(|engine| {
        let input = "今日はいい天気ですね。";
        let result = engine.default_translate(input);

        assert!(result.is_ok(), "Translation failed: {:?}", result.err());
        let translated = result.unwrap();
        assert!(!translated.is_empty());
    });
}

#[test]
#[ignore]
#[serial]
fn test_default_translate_with_korean() {
    with_engine(|engine| {
        // Input with Korean that needs encoding
        let input = "가나다라おはようございます。";
        let result = engine.default_translate(input);

        assert!(result.is_ok(), "Translation failed: {:?}", result.err());
        let translated = result.unwrap();

        // Korean should be preserved
        assert!(translated.contains("가나다라"));
    });
}

#[test]
#[ignore]
#[serial]
fn test_default_translate_empty() {
    with_engine(|engine| {
        let input = "";
        let result = engine.default_translate(input);

        assert!(result.is_ok());
    });
}

#[test]
#[ignore]
#[serial]
fn test_translate_multiple_times() {
    with_engine(|engine| {
        let texts = [
            "おはようございます。",
            "こんにちは。",
            "こんばんは。",
            "ありがとうございます。",
            "すみません。",
        ];

        for text in &texts {
            let result = engine.default_translate(text);
            assert!(
                result.is_ok(),
                "Failed to translate '{}': {:?}",
                text,
                result.err()
            );
        }
    });
}

// ============================================
// Reload User Dict Test
// ============================================

#[test]
#[ignore]
#[serial]
fn test_reload_user_dict() {
    with_engine(|engine| {
        let result = engine.reload_user_dict();
        assert!(
            result.is_ok(),
            "Failed to reload user dict: {:?}",
            result.err()
        );
    });
}

// ============================================
// Property Tests
// ============================================

#[test]
#[ignore]
#[serial]
fn test_set_property() {
    with_engine(|engine| {
        // Check that set_property function is loaded
        assert!(engine.set_property.is_some());
    });
}

#[test]
#[ignore]
#[serial]
fn test_get_property() {
    with_engine(|engine| {
        // Check that get_property function is loaded
        assert!(engine.get_property.is_some());
    });
}

// ============================================
// Emoji Test
// ============================================

#[test]
#[ignore]
#[serial]
fn test_emoji_translation() {
    with_engine(|engine| {
        // 이모지가 포함된 일본어 텍스트
        let test_cases = [
            ("こんにちは😀", "단일 이모지"),
            ("ありがとう👨‍👩‍👧ございます", "ZWJ 시퀀스"),
            ("今日🇰🇷天気", "국기 이모지"),
            ("おはよう👋🏻", "피부색 이모지"),
            ("テスト😀😀😀テスト", "다중 이모지"),
        ];

        for (input, desc) in test_cases {
            println!("\n=== {} ===", desc);
            println!("입력: {}", input);

            // 이모지 코드포인트 확인
            print!("코드포인트: ");
            for c in input.chars() {
                if c as u32 >= 0x1F000 || c == '\u{200D}' {
                    print!("U+{:X} ", c as u32);
                }
            }
            println!();

            let result = engine.translate_mmntw(input);
            match result {
                Ok(translated) => {
                    println!("번역 결과: {}", translated);

                    // 이모지가 보존되었는지 확인
                    let input_emojis: Vec<char> = input.chars()
                        .filter(|c| *c as u32 >= 0x1F000 || *c == '\u{200D}')
                        .collect();
                    let output_emojis: Vec<char> = translated.chars()
                        .filter(|c| *c as u32 >= 0x1F000 || *c == '\u{200D}')
                        .collect();

                    println!("입력 이모지: {:?}", input_emojis);
                    println!("출력 이모지: {:?}", output_emojis);

                    if input_emojis == output_emojis {
                        println!("✓ 이모지 보존됨");
                    } else {
                        println!("✗ 이모지 변경/손실됨!");
                    }
                }
                Err(e) => {
                    println!("번역 실패: {:?}", e);
                }
            }
        }
    });
}

#[test]
#[ignore]
#[serial]
fn test_emoji_only() {
    with_engine(|engine| {
        // 이모지만 있는 경우
        let input = "😀";
        println!("입력: {}", input);

        let result = engine.translate_mmntw(input);
        match result {
            Ok(translated) => {
                println!("번역 결과: '{}'", translated);
                println!("번역 결과 바이트: {:?}", translated.as_bytes());
            }
            Err(e) => {
                println!("번역 실패: {:?}", e);
            }
        }
    });
}
