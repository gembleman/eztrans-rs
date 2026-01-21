// Thread-Local Engine Test for EzTrans DLL
// Tests if each thread can have its own independent DLL instance
//
// Run with: cargo test --target i686-pc-windows-msvc --test thread_local_test -- --ignored --nocapture

use eztrans_rs::EzTransEngine;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

fn get_engine_paths() -> (String, String) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dll_path = format!("{}/../eztrans_dll/J2KEngine.dll", manifest_dir);
    let dat_path = format!("{}/../eztrans_dll/Dat", manifest_dir);
    (dll_path, dat_path)
}

/// Check if output looks corrupted
fn is_corrupted(input: &str, output: &str) -> bool {
    if !input.is_empty() && output.is_empty() {
        return true;
    }

    if output.contains('\0') {
        return true;
    }

    for c in output.chars() {
        if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
            return true;
        }
    }

    let korean_count = output
        .chars()
        .filter(|c| {
            let code = *c as u32;
            (code >= 0xAC00 && code <= 0xD7A3)
                || (code >= 0x3000 && code <= 0x303F)
                || c.is_ascii_punctuation()
                || c.is_whitespace()
        })
        .count();

    let total = output.chars().count();
    if total > 5 {
        let ratio = korean_count as f64 / total as f64;
        if ratio < 0.3 {
            return true;
        }
    }

    false
}

// ============================================
// Test 1: Thread-Local Engine - Each thread creates its own engine
// ============================================
#[test]
#[ignore]
fn test_thread_local_separate_engines() {
    println!("\n=== Thread-Local Engine Test (Separate Instances) ===");
    println!("Each thread creates and owns its own EzTransEngine instance.\n");

    let (dll_path, dat_path) = get_engine_paths();

    let num_threads = 4;
    let iterations_per_thread = 25;
    let barrier = Arc::new(Barrier::new(num_threads));

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    let init_error_count = Arc::new(AtomicUsize::new(0));

    let test_texts = vec![
        ("おはようございます。", "안녕"),
        ("こんにちは。", "안녕"),
        ("こんばんは。", "안녕"),
        ("ありがとうございます。", "감사"),
        ("今日はいい天気ですね。", "오늘"),
        ("私は学生です。", "학생"),
        ("日本語を勉強しています。", "일본어"),
        ("明日は雨が降るでしょう。", "내일"),
    ];

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let dll_path = dll_path.clone();
            let dat_path = dat_path.clone();
            let barrier = Arc::clone(&barrier);
            let success = Arc::clone(&success_count);
            let errors = Arc::clone(&error_count);
            let init_errors = Arc::clone(&init_error_count);
            let texts = test_texts.clone();

            thread::spawn(move || {
                println!("  Thread {} starting, creating engine...", thread_id);

                // Each thread creates its OWN engine instance
                let engine = match EzTransEngine::new(&dll_path) {
                    Ok(e) => e,
                    Err(e) => {
                        println!("  Thread {} FAILED to load DLL: {:?}", thread_id, e);
                        init_errors.fetch_add(1, Ordering::SeqCst);
                        return;
                    }
                };

                // Initialize engine
                if let Err(e) = engine.initialize_ex("CSUSER123455", &dat_path) {
                    println!("  Thread {} FAILED to initialize: {:?}", thread_id, e);
                    init_errors.fetch_add(1, Ordering::SeqCst);
                    return;
                }

                println!("  Thread {} engine initialized successfully!", thread_id);

                // Wait for all threads to initialize
                barrier.wait();

                // Now run translations
                for i in 0..iterations_per_thread {
                    let (text, expected_substr) =
                        &texts[(thread_id * iterations_per_thread + i) % texts.len()];

                    match engine.translate_mmntw(text) {
                        Ok(translated) => {
                            if is_corrupted(text, &translated) {
                                errors.fetch_add(1, Ordering::SeqCst);
                                println!(
                                    "  Thread {} CORRUPTED: '{}' -> '{}'",
                                    thread_id, text, translated
                                );
                            } else if !translated.contains(expected_substr) {
                                errors.fetch_add(1, Ordering::SeqCst);
                                println!(
                                    "  Thread {} WRONG: '{}' -> '{}' (expected '{}')",
                                    thread_id, text, translated, expected_substr
                                );
                            } else {
                                success.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                        Err(e) => {
                            errors.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} ERROR: {:?}", thread_id, e);
                        }
                    }
                }

                println!("  Thread {} completed.", thread_id);
                // Engine is automatically dropped here
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.join();
    }

    let elapsed = start.elapsed();
    let total = num_threads * iterations_per_thread;
    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);
    let init_errors = init_error_count.load(Ordering::SeqCst);

    println!("\nThread-Local Engine Results ({} threads):", num_threads);
    println!("  Initialization Errors: {}", init_errors);
    println!("  Total translations attempted: {}", total);
    println!("  Success: {}", successes);
    println!("  Errors: {}", errors);
    println!("  Time: {:?}", elapsed);

    if init_errors > 0 {
        println!("\n✗ DLL cannot be loaded multiple times in same process!");
        println!("  This means Thread-Local approach will NOT work.");
    } else if errors == 0 {
        println!("\n✓ Thread-Local engines work! Each thread can have its own DLL instance.");
        println!(
            "  Rate: {:.1} translations/sec",
            total as f64 / elapsed.as_secs_f64()
        );
    } else {
        println!("\n✗ Thread-Local approach has issues ({} errors)", errors);
    }
}

// ============================================
// Test 2: Thread-Local with thread_local! macro
// ============================================
#[test]
#[ignore]
fn test_thread_local_macro() {
    println!("\n=== Thread-Local Macro Test ===");
    println!("Using thread_local! macro for per-thread engines.\n");

    let (dll_path, dat_path) = get_engine_paths();

    // Store paths for thread_local! access
    let dll_path_static: &'static str = Box::leak(dll_path.into_boxed_str());
    let dat_path_static: &'static str = Box::leak(dat_path.into_boxed_str());

    thread_local! {
        static THREAD_ENGINE: RefCell<Option<EzTransEngine>> = const { RefCell::new(None) };
    }

    let num_threads = 4;
    let iterations_per_thread = 20;
    let barrier = Arc::new(Barrier::new(num_threads));

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let barrier = Arc::clone(&barrier);
            let success = Arc::clone(&success_count);
            let errors = Arc::clone(&error_count);

            thread::spawn(move || {
                // Initialize thread-local engine
                let init_result = THREAD_ENGINE.with(|cell| {
                    let mut engine_opt = cell.borrow_mut();
                    if engine_opt.is_none() {
                        match EzTransEngine::new(dll_path_static) {
                            Ok(engine) => {
                                if let Err(e) =
                                    engine.initialize_ex("CSUSER123455", dat_path_static)
                                {
                                    println!("  Thread {} init failed: {:?}", thread_id, e);
                                    return Err(format!("Init failed: {:?}", e));
                                }
                                *engine_opt = Some(engine);
                                println!("  Thread {} engine created via thread_local!", thread_id);
                                Ok(())
                            }
                            Err(e) => {
                                println!("  Thread {} DLL load failed: {:?}", thread_id, e);
                                Err(format!("DLL load failed: {:?}", e))
                            }
                        }
                    } else {
                        Ok(())
                    }
                });

                if init_result.is_err() {
                    errors.fetch_add(iterations_per_thread, Ordering::SeqCst);
                    return;
                }

                barrier.wait();

                // Run translations
                for i in 0..iterations_per_thread {
                    let text = if i % 2 == 0 {
                        "おはよう"
                    } else {
                        "こんにちは"
                    };

                    let result = THREAD_ENGINE.with(|cell| {
                        let engine_opt = cell.borrow();
                        if let Some(ref engine) = *engine_opt {
                            engine.translate_mmntw(text)
                        } else {
                            Err(eztrans_rs::EzTransError::FunctionLoadError(
                                "No engine".to_string(),
                            ))
                        }
                    });

                    match result {
                        Ok(translated) if !is_corrupted(text, &translated) => {
                            success.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {
                            errors.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }

                println!("  Thread {} completed.", thread_id);
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.join();
    }

    let elapsed = start.elapsed();
    let total = num_threads * iterations_per_thread;
    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);

    println!("\nThread-Local Macro Results:");
    println!("  Total: {}", total);
    println!("  Success: {}", successes);
    println!("  Errors: {}", errors);
    println!("  Time: {:?}", elapsed);

    if errors == 0 {
        println!("\n✓ thread_local! macro approach works!");
    } else {
        println!("\n✗ thread_local! approach failed ({} errors)", errors);
    }
}

// ============================================
// Test 3: Staggered engine initialization
// ============================================
#[test]
#[ignore]
fn test_staggered_init() {
    println!("\n=== Staggered Initialization Test ===");
    println!("Testing with delays between engine initializations.\n");

    let (dll_path, dat_path) = get_engine_paths();

    let num_threads = 2;  // Reduced for testing
    let iterations_per_thread = 10;

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    let init_success = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let dll_path = dll_path.clone();
            let dat_path = dat_path.clone();
            let success = Arc::clone(&success_count);
            let errors = Arc::clone(&error_count);
            let init_ok = Arc::clone(&init_success);

            thread::spawn(move || {
                // Stagger initialization: each thread waits before initializing
                thread::sleep(std::time::Duration::from_millis(thread_id as u64 * 500));

                println!(
                    "  Thread {} starting initialization at {:?}",
                    thread_id,
                    Instant::now()
                );

                let engine = match EzTransEngine::new(&dll_path) {
                    Ok(e) => e,
                    Err(e) => {
                        println!("  Thread {} DLL load FAILED: {:?}", thread_id, e);
                        errors.fetch_add(iterations_per_thread, Ordering::SeqCst);
                        return;
                    }
                };

                if let Err(e) = engine.initialize_ex("CSUSER123455", &dat_path) {
                    println!("  Thread {} init FAILED: {:?}", thread_id, e);
                    errors.fetch_add(iterations_per_thread, Ordering::SeqCst);
                    return;
                }

                init_ok.fetch_add(1, Ordering::SeqCst);
                println!("  Thread {} initialized successfully!", thread_id);

                // Run translations
                for i in 0..iterations_per_thread {
                    let text = match i % 3 {
                        0 => "おはよう",
                        1 => "こんにちは",
                        _ => "ありがとう",
                    };

                    match engine.translate_mmntw(text) {
                        Ok(translated) if !is_corrupted(text, &translated) => {
                            success.fetch_add(1, Ordering::SeqCst);
                        }
                        Ok(translated) => {
                            errors.fetch_add(1, Ordering::SeqCst);
                            println!(
                                "  Thread {} corrupted output: '{}' -> '{}'",
                                thread_id, text, translated
                            );
                        }
                        Err(e) => {
                            errors.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} translation error: {:?}", thread_id, e);
                        }
                    }
                }

                println!("  Thread {} finished translations.", thread_id);
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.join();
    }

    let elapsed = start.elapsed();
    let init_count = init_success.load(Ordering::SeqCst);
    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);

    println!("\nStaggered Init Results:");
    println!("  Engines initialized: {}/{}", init_count, num_threads);
    println!("  Success: {}", successes);
    println!("  Errors: {}", errors);
    println!("  Total time: {:?}", elapsed);

    if init_count == num_threads && errors == 0 {
        println!("\n✓ Staggered initialization works!");
    } else if init_count < num_threads {
        println!("\n✗ Cannot create multiple DLL instances in same process");
    } else {
        println!("\n✗ Some translations failed even with staggered init");
    }
}

// ============================================
// Test 4: Sequential multi-engine test (baseline)
// ============================================
#[test]
#[ignore]
fn test_sequential_multi_engine() {
    println!("\n=== Sequential Multi-Engine Test ===");
    println!("Testing if we can even create multiple engines sequentially.\n");

    let (dll_path, dat_path) = get_engine_paths();

    println!("Creating first engine...");
    let engine1 = EzTransEngine::new(&dll_path).expect("Failed to load DLL 1");
    engine1
        .initialize_ex("CSUSER123455", &dat_path)
        .expect("Failed to init 1");
    println!("  Engine 1 created and initialized.");

    // Test first engine
    let result1 = engine1.translate_mmntw("こんにちは").unwrap();
    println!("  Engine 1 translation: 'こんにちは' -> '{}'", result1);

    println!("\nCreating second engine (while first is still alive)...");
    match EzTransEngine::new(&dll_path) {
        Ok(engine2) => {
            println!("  Second DLL loaded successfully.");
            match engine2.initialize_ex("CSUSER123455", &dat_path) {
                Ok(_) => {
                    println!("  Engine 2 initialized successfully!");

                    // Test both engines
                    let r1 = engine1.translate_mmntw("おはよう").unwrap();
                    let r2 = engine2.translate_mmntw("さようなら").unwrap();

                    println!("\n  Engine 1: 'おはよう' -> '{}'", r1);
                    println!("  Engine 2: 'さようなら' -> '{}'", r2);

                    // Check if outputs are correct
                    if r1.contains("안녕") && r2.contains("안녕") {
                        println!("\n✓ Multiple engines CAN coexist in same process!");
                        println!("  Thread-Local approach should work!");
                    } else {
                        println!("\n⚠ Engines created but outputs may be incorrect");
                        println!(
                            "  Engine 1 result valid: {}",
                            r1.contains("안녕") || r1.contains("좋은")
                        );
                        println!(
                            "  Engine 2 result valid: {}",
                            r2.contains("안녕") || r2.contains("작별")
                        );
                    }
                }
                Err(e) => {
                    println!("  Engine 2 init FAILED: {:?}", e);
                    println!("\n✗ Cannot initialize multiple engines");
                }
            }
        }
        Err(e) => {
            println!("  Second DLL load FAILED: {:?}", e);
            println!("\n✗ Cannot load DLL multiple times");
            println!("  Thread-Local approach will NOT work.");
        }
    }

    // Drop engine1 first, then try again
    drop(engine1);
    println!("\nEngine 1 dropped. Creating engine 3...");

    let engine3 = EzTransEngine::new(&dll_path).expect("Failed to load DLL 3");
    engine3
        .initialize_ex("CSUSER123455", &dat_path)
        .expect("Failed to init 3");
    let r3 = engine3.translate_mmntw("ありがとう").unwrap();
    println!("  Engine 3: 'ありがとう' -> '{}'", r3);
    println!("  Sequential create-destroy-create works: ✓");
}

// ============================================
// Test 5: Two engines with mutex-protected translation
// Each thread has its own engine, but translations are serialized
// ============================================
#[test]
#[ignore]
fn test_two_engines_serialized() {
    use std::sync::Mutex;

    println!("\n=== Two Engines with Serialized Translation ===");
    println!("Each thread has own engine, but translations are mutex-protected.\n");

    let (dll_path, dat_path) = get_engine_paths();

    // Create a global translation lock
    let translate_lock = Arc::new(Mutex::new(()));

    let num_threads = 2;
    let iterations_per_thread = 20;
    let barrier = Arc::new(Barrier::new(num_threads));

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let dll_path = dll_path.clone();
            let dat_path = dat_path.clone();
            let barrier = Arc::clone(&barrier);
            let translate_lock = Arc::clone(&translate_lock);
            let success = Arc::clone(&success_count);
            let errors = Arc::clone(&error_count);

            thread::spawn(move || {
                // Stagger DLL loading
                thread::sleep(std::time::Duration::from_millis(thread_id as u64 * 200));

                println!("  Thread {} creating engine...", thread_id);

                let engine = match EzTransEngine::new(&dll_path) {
                    Ok(e) => e,
                    Err(e) => {
                        println!("  Thread {} DLL load failed: {:?}", thread_id, e);
                        errors.fetch_add(iterations_per_thread, Ordering::SeqCst);
                        return;
                    }
                };

                if let Err(e) = engine.initialize_ex("CSUSER123455", &dat_path) {
                    println!("  Thread {} init failed: {:?}", thread_id, e);
                    errors.fetch_add(iterations_per_thread, Ordering::SeqCst);
                    return;
                }

                println!("  Thread {} engine ready!", thread_id);
                barrier.wait();

                // Run translations with global lock
                for i in 0..iterations_per_thread {
                    let text = if i % 2 == 0 { "おはよう" } else { "こんにちは" };

                    // Acquire global lock before translation
                    let _guard = translate_lock.lock().unwrap();

                    match engine.translate_mmntw(text) {
                        Ok(translated) if !is_corrupted(text, &translated) => {
                            success.fetch_add(1, Ordering::SeqCst);
                        }
                        Ok(translated) => {
                            errors.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} corrupted: '{}' -> '{}'", thread_id, text, translated);
                        }
                        Err(e) => {
                            errors.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} error: {:?}", thread_id, e);
                        }
                    }
                    // Lock released here
                }

                println!("  Thread {} completed.", thread_id);
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.join();
    }

    let elapsed = start.elapsed();
    let total = num_threads * iterations_per_thread;
    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);

    println!("\nSerialized Translation Results:");
    println!("  Total: {}", total);
    println!("  Success: {}", successes);
    println!("  Errors: {}", errors);
    println!("  Time: {:?}", elapsed);

    if errors == 0 {
        println!("\n✓ Multiple engines work when translations are serialized!");
        println!("  But this negates the benefit of multithreading.");
    } else {
        println!("\n✗ Even serialized translations failed!");
    }
}

// ============================================
// Test 6: Single thread, alternating between two engines
// ============================================
#[test]
#[ignore]
fn test_single_thread_two_engines() {
    println!("\n=== Single Thread, Two Engines Test ===");
    println!("Testing if two engines can be used alternately in same thread.\n");

    let (dll_path, dat_path) = get_engine_paths();

    println!("Creating engine 1...");
    let engine1 = EzTransEngine::new(&dll_path).expect("Failed to create engine 1");
    engine1.initialize_ex("CSUSER123455", &dat_path).expect("Failed to init engine 1");
    println!("  Engine 1 ready.");

    println!("Creating engine 2...");
    let engine2 = EzTransEngine::new(&dll_path).expect("Failed to create engine 2");
    engine2.initialize_ex("CSUSER123455", &dat_path).expect("Failed to init engine 2");
    println!("  Engine 2 ready.");

    println!("\nAlternating translations:");
    let mut engine1_success = 0;
    let mut engine2_success = 0;
    let mut engine1_fail = 0;
    let mut engine2_fail = 0;

    for i in 0..20 {
        // Engine 1 translation
        let text1 = "おはよう";
        match engine1.translate_mmntw(text1) {
            Ok(result) if !result.is_empty() && result.contains("안녕") => {
                engine1_success += 1;
                if i < 3 { println!("  [{}] Engine1: '{}' -> '{}'", i, text1, result); }
            }
            Ok(result) => {
                engine1_fail += 1;
                println!("  [{}] Engine1 FAIL: '{}' -> '{}'", i, text1, result);
            }
            Err(e) => {
                engine1_fail += 1;
                println!("  [{}] Engine1 ERROR: {:?}", i, e);
            }
        }

        // Engine 2 translation
        let text2 = "こんにちは";
        match engine2.translate_mmntw(text2) {
            Ok(result) if !result.is_empty() && result.contains("안녕") => {
                engine2_success += 1;
                if i < 3 { println!("  [{}] Engine2: '{}' -> '{}'", i, text2, result); }
            }
            Ok(result) => {
                engine2_fail += 1;
                println!("  [{}] Engine2 FAIL: '{}' -> '{}'", i, text2, result);
            }
            Err(e) => {
                engine2_fail += 1;
                println!("  [{}] Engine2 ERROR: {:?}", i, e);
            }
        }
    }

    println!("\nResults:");
    println!("  Engine 1: {}/20 success, {} failed", engine1_success, engine1_fail);
    println!("  Engine 2: {}/20 success, {} failed", engine2_success, engine2_fail);

    if engine1_fail == 0 && engine2_fail == 0 {
        println!("\n✓ Both engines work correctly when alternating!");
    } else if engine1_fail == 0 && engine2_fail > 0 {
        println!("\n✗ Only the FIRST engine works. Second engine is broken.");
        println!("  DLL has process-global state that only supports ONE instance.");
    } else if engine1_fail > 0 && engine2_fail == 0 {
        println!("\n✗ Only the SECOND engine works. First engine was overwritten.");
        println!("  DLL initialization replaces previous instance.");
    } else {
        println!("\n✗ Both engines have issues.");
    }
}

// ============================================
// Test 7: Check if LoadLibrary returns same handle
// ============================================
#[test]
#[ignore]
fn test_dll_handle_identity() {
    println!("\n=== DLL Handle Identity Test ===");
    println!("Check if multiple LoadLibrary calls return the same handle.\n");

    let (dll_path, dat_path) = get_engine_paths();

    let engine1 = EzTransEngine::new(&dll_path).expect("Failed to create engine 1");
    engine1.initialize_ex("CSUSER123455", &dat_path).expect("Failed to init engine 1");

    let engine2 = EzTransEngine::new(&dll_path).expect("Failed to create engine 2");
    // Don't initialize engine2 to see if the handle is the same

    println!("  Engine 1 HMODULE: {:?}", engine1.module);
    println!("  Engine 2 HMODULE: {:?}", engine2.module);

    if engine1.module == engine2.module {
        println!("\n✗ Both engines share the SAME DLL handle!");
        println!("  LoadLibrary returns the same HMODULE for already-loaded DLLs.");
        println!("  This means Thread-Local engines are actually sharing the same DLL instance.");
        println!("  The DLL's global state is process-wide, not per-handle.");
    } else {
        println!("\n✓ Engines have different DLL handles.");
        println!("  This is unexpected - Windows should return the same handle.");
    }

    // Test if engine2 can translate without being initialized
    println!("\nTesting if engine2 works without explicit initialization...");
    match engine2.translate_mmntw("テスト") {
        Ok(result) => {
            println!("  Engine2 (uninitialized) translated: 'テスト' -> '{}'", result);
            if !result.is_empty() {
                println!("  ⚠ Engine2 works because it shares DLL state with Engine1!");
            }
        }
        Err(e) => {
            println!("  Engine2 (uninitialized) failed: {:?}", e);
        }
    }
}

/// Wrapper to allow sharing EzTransEngine across threads
/// WARNING: This is intentionally unsafe - we're testing thread behavior
struct UnsafeEngineWrapper(EzTransEngine);
unsafe impl Send for UnsafeEngineWrapper {}
unsafe impl Sync for UnsafeEngineWrapper {}

// ============================================
// Test 8: Engine in spawned thread using main thread's init
// ============================================
#[test]
#[ignore]
fn test_cross_thread_engine_use() {
    println!("\n=== Cross-Thread Engine Usage Test ===");
    println!("Initialize in main thread, use in spawned thread.\n");

    let (dll_path, dat_path) = get_engine_paths();

    // Create and initialize in main thread
    println!("Main thread: Creating and initializing engine...");
    let engine = EzTransEngine::new(&dll_path).expect("Failed to create engine");
    engine.initialize_ex("CSUSER123455", &dat_path).expect("Failed to init");

    // Test in main thread first
    let main_result = engine.translate_mmntw("メインスレッド").unwrap();
    println!("  Main thread translation: 'メインスレッド' -> '{}'", main_result);

    let wrapper = Arc::new(UnsafeEngineWrapper(engine));
    let wrapper_clone = Arc::clone(&wrapper);

    // Use in spawned thread
    let handle = thread::spawn(move || {
        let wrapper = wrapper_clone;
        println!("  Spawned thread: Attempting translation...");
        match wrapper.0.translate_mmntw("サブスレッド") {
            Ok(result) => {
                println!("  Spawned thread translation: 'サブスレッド' -> '{}'", result);
                if result.contains("서브") || result.contains("스레드") || !result.is_empty() {
                    println!("  ✓ Cross-thread usage works!");
                    true
                } else {
                    println!("  ✗ Got empty or wrong result");
                    false
                }
            }
            Err(e) => {
                println!("  ✗ Spawned thread translation failed: {:?}", e);
                false
            }
        }
    });

    let success = handle.join().unwrap();

    if success {
        println!("\n✓ Engine initialized in one thread CAN be used in another!");
        println!("  This means Thread-Local with shared init MIGHT work.");
    } else {
        println!("\n✗ Engine cannot be used across threads.");
    }
}

// ============================================
// Test 9: Heavy concurrent load test
// ============================================
#[test]
#[ignore]
fn test_thread_local_heavy_load() {
    println!("\n=== Heavy Concurrent Load Test ===");
    println!("Testing Thread-Local engines under heavy concurrent load.\n");

    let (dll_path, dat_path) = get_engine_paths();

    let num_threads = 8;
    let iterations_per_thread = 50;

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(num_threads));

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|_thread_id| {
            let dll_path = dll_path.clone();
            let dat_path = dat_path.clone();
            let barrier = Arc::clone(&barrier);
            let success = Arc::clone(&success_count);
            let errors = Arc::clone(&error_count);

            thread::spawn(move || {
                // Create thread-local engine
                let engine = match EzTransEngine::new(&dll_path) {
                    Ok(e) => e,
                    Err(_) => {
                        errors.fetch_add(iterations_per_thread, Ordering::SeqCst);
                        return;
                    }
                };

                if engine.initialize_ex("CSUSER123455", &dat_path).is_err() {
                    errors.fetch_add(iterations_per_thread, Ordering::SeqCst);
                    return;
                }

                // Wait for all threads
                barrier.wait();

                // Heavy translation load
                for i in 0..iterations_per_thread {
                    let text = match i % 5 {
                        0 => "おはようございます。今日も頑張りましょう。",
                        1 => "こんにちは。お元気ですか？",
                        2 => "日本語から韓国語への翻訳テストです。",
                        3 => "今日は天気がいいですね。散歩に行きましょう。",
                        _ => "ありがとうございます。とても嬉しいです。",
                    };

                    match engine.translate_mmntw(text) {
                        Ok(translated) if !is_corrupted(text, &translated) => {
                            success.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {
                            errors.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.join();
    }

    let elapsed = start.elapsed();
    let total = num_threads * iterations_per_thread;
    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);

    println!("Heavy Load Results ({} threads, {} ops each):", num_threads, iterations_per_thread);
    println!("  Total: {}", total);
    println!("  Success: {}", successes);
    println!("  Errors: {}", errors);
    println!("  Time: {:?}", elapsed);
    println!(
        "  Throughput: {:.1} translations/sec",
        successes as f64 / elapsed.as_secs_f64()
    );

    let success_rate = (successes as f64 / total as f64) * 100.0;
    println!("  Success Rate: {:.1}%", success_rate);

    if success_rate >= 99.0 {
        println!("\n✓ Thread-Local engines handle heavy load well!");
    } else if success_rate >= 90.0 {
        println!("\n⚠ Thread-Local mostly works but has some issues");
    } else {
        println!("\n✗ Thread-Local approach has significant issues");
    }
}
