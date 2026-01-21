// Thread Safety Test for EzTrans DLL
// This test determines if the DLL can handle concurrent translations
//
// Run with: cargo test --target i686-pc-windows-msvc --test thread_safety_test -- --ignored --nocapture

use eztrans_rs::EzTransEngine;
use std::sync::{Arc, Barrier, atomic::{AtomicUsize, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

fn get_engine_paths() -> (String, String) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dll_path = format!("{}/../eztrans_dll/J2KEngine.dll", manifest_dir);
    let dat_path = format!("{}/../eztrans_dll/Dat", manifest_dir);
    (dll_path, dat_path)
}

/// Wrapper to allow sharing EzTransEngine across threads
/// WARNING: This is intentionally unsafe - we're testing if the DLL can handle it
struct UnsafeEngineWrapper(EzTransEngine);
unsafe impl Send for UnsafeEngineWrapper {}
unsafe impl Sync for UnsafeEngineWrapper {}

/// Check if output looks corrupted (contains garbage characters)
fn is_corrupted(input: &str, output: &str) -> bool {
    // Empty output for non-empty input is suspicious
    if !input.is_empty() && output.is_empty() {
        return true;
    }

    // Check for common corruption patterns
    // 1. Contains null characters
    if output.contains('\0') {
        return true;
    }

    // 2. Contains ASCII control characters (except newline, tab)
    for c in output.chars() {
        if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
            return true;
        }
    }

    // 3. Output contains garbage-looking patterns (random ASCII mixed with Korean)
    // Valid Korean output should mostly be Hangul, punctuation, or spaces
    let korean_count = output.chars().filter(|c| {
        let code = *c as u32;
        // Hangul syllables or common punctuation
        (code >= 0xAC00 && code <= 0xD7A3) ||  // Hangul
        (code >= 0x3000 && code <= 0x303F) ||  // CJK punctuation
        c.is_ascii_punctuation() ||
        c.is_whitespace()
    }).count();

    let total = output.chars().count();
    if total > 5 {
        // If less than 50% is valid Korean/punctuation, likely corrupted
        let ratio = korean_count as f64 / total as f64;
        if ratio < 0.3 {
            return true;
        }
    }

    false
}

// ============================================
// Test 1: Sequential baseline (control test)
// ============================================
#[test]
#[ignore]
fn test_sequential_baseline() {
    println!("\n=== Sequential Baseline Test ===");

    let (dll_path, dat_path) = get_engine_paths();
    let engine = EzTransEngine::new(&dll_path).expect("Failed to load DLL");
    engine.initialize_ex("CSUSER123455", &dat_path).expect("Failed to initialize");

    let test_texts = vec![
        "おはようございます。",
        "こんにちは。",
        "こんばんは。",
        "ありがとうございます。",
        "今日はいい天気ですね。",
    ];

    let start = Instant::now();
    let mut success_count = 0;
    let iterations = 20;

    for i in 0..iterations {
        let text = &test_texts[i % test_texts.len()];
        match engine.translate_mmntw(text) {
            Ok(result) => {
                success_count += 1;
                if i < 5 {
                    println!("  [{}] '{}' -> '{}'", i, text, result);
                }
            }
            Err(e) => {
                println!("  [{}] ERROR: {:?}", i, e);
            }
        }
    }

    let elapsed = start.elapsed();
    println!("\nSequential Results:");
    println!("  Success: {}/{}", success_count, iterations);
    println!("  Time: {:?}", elapsed);
    println!("  Rate: {:.1} translations/sec", iterations as f64 / elapsed.as_secs_f64());

    assert_eq!(success_count, iterations, "Sequential baseline should have 100% success");
}

// ============================================
// Test 2: Multi-threaded with single shared engine (detect corruption)
// ============================================
#[test]
#[ignore]
fn test_multithread_shared_engine() {
    println!("\n=== Multi-threaded Shared Engine Test ===");
    println!("Testing if DLL can handle concurrent access from multiple threads...\n");

    let (dll_path, dat_path) = get_engine_paths();
    let engine = EzTransEngine::new(&dll_path).expect("Failed to load DLL");
    engine.initialize_ex("CSUSER123455", &dat_path).expect("Failed to initialize");

    let engine = Arc::new(UnsafeEngineWrapper(engine));

    let num_threads = 4;
    let iterations_per_thread = 25;
    let barrier = Arc::new(Barrier::new(num_threads));

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    let crash_count = Arc::new(AtomicUsize::new(0));
    let corrupted_count = Arc::new(AtomicUsize::new(0));

    let test_texts = vec![
        ("おはようございます。", "안녕하세요"),  // Expected substring
        ("こんにちは。", "안녕"),
        ("こんばんは。", "안녕"),
        ("ありがとうございます。", "감사"),
        ("今日はいい天気ですね。", "오늘"),
        ("私は学生です。", "학생"),
        ("日本語を勉強しています。", "일본어"),
        ("明日は雨が降るでしょう。", "내일"),
    ];

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads).map(|thread_id| {
        let engine = Arc::clone(&engine);
        let barrier = Arc::clone(&barrier);
        let success = Arc::clone(&success_count);
        let errors = Arc::clone(&error_count);
        let crashes = Arc::clone(&crash_count);
        let corrupted = Arc::clone(&corrupted_count);
        let texts = test_texts.clone();

        thread::spawn(move || {
            // Wait for all threads to be ready
            barrier.wait();

            for i in 0..iterations_per_thread {
                let (text, expected_substr) = &texts[(thread_id * iterations_per_thread + i) % texts.len()];

                // Use catch_unwind to detect panics/crashes
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    engine.0.translate_mmntw(text)
                }));

                match result {
                    Ok(Ok(translated)) => {
                        // Check for corruption
                        if is_corrupted(text, &translated) {
                            corrupted.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} CORRUPTED: '{}' -> '{}'", thread_id, text, translated);
                        } else if !translated.contains(expected_substr) {
                            // Output doesn't contain expected Korean
                            corrupted.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} WRONG OUTPUT: '{}' -> '{}' (expected '{}')",
                                thread_id, text, translated, expected_substr);
                        } else {
                            success.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    Ok(Err(e)) => {
                        errors.fetch_add(1, Ordering::SeqCst);
                        println!("  Thread {} ERROR: {:?}", thread_id, e);
                    }
                    Err(_) => {
                        crashes.fetch_add(1, Ordering::SeqCst);
                        println!("  Thread {} CRASHED!", thread_id);
                    }
                }

                // Small delay to increase chance of race conditions
                thread::sleep(Duration::from_micros(50));
            }
        })
    }).collect();

    // Wait for all threads
    for handle in handles {
        let _ = handle.join();
    }

    let elapsed = start.elapsed();
    let total = num_threads * iterations_per_thread;
    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);
    let crashes = crash_count.load(Ordering::SeqCst);
    let corrupted = corrupted_count.load(Ordering::SeqCst);

    println!("\nMulti-threaded Results ({} threads, {} total):", num_threads, total);
    println!("  Success: {}", successes);
    println!("  Errors: {}", errors);
    println!("  Crashes: {}", crashes);
    println!("  Corrupted: {}", corrupted);
    println!("  Time: {:?}", elapsed);

    let failure_count = errors + crashes + corrupted;
    if failure_count == 0 {
        println!("\n✓ DLL appears to be THREAD-SAFE in this run");
    } else {
        println!("\n✗ DLL is NOT THREAD-SAFE ({} failures detected)", failure_count);
    }
}

// ============================================
// Test 3: Multi-threaded with mutex protection
// ============================================
#[test]
#[ignore]
fn test_multithread_with_mutex() {
    use std::sync::Mutex;

    println!("\n=== Multi-threaded with Mutex Protection ===");
    println!("Testing thread safety with mutex synchronization...\n");

    let (dll_path, dat_path) = get_engine_paths();
    let engine = EzTransEngine::new(&dll_path).expect("Failed to load DLL");
    engine.initialize_ex("CSUSER123455", &dat_path).expect("Failed to initialize");

    let engine = Arc::new(Mutex::new(UnsafeEngineWrapper(engine)));

    let num_threads = 4;
    let iterations_per_thread = 25;
    let barrier = Arc::new(Barrier::new(num_threads));

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let test_texts = vec![
        "おはようございます。",
        "こんにちは。",
        "こんばんは。",
        "ありがとうございます。",
    ];

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads).map(|thread_id| {
        let engine = Arc::clone(&engine);
        let barrier = Arc::clone(&barrier);
        let success = Arc::clone(&success_count);
        let errors = Arc::clone(&error_count);
        let texts = test_texts.clone();

        thread::spawn(move || {
            barrier.wait();

            for i in 0..iterations_per_thread {
                let text = &texts[(thread_id + i) % texts.len()];

                // Lock the mutex before accessing engine
                let guard = engine.lock().unwrap();
                match guard.0.translate_mmntw(text) {
                    Ok(result) => {
                        if !is_corrupted(text, &result) {
                            success.fetch_add(1, Ordering::SeqCst);
                        } else {
                            errors.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} CORRUPTED even with mutex!", thread_id);
                        }
                    }
                    Err(e) => {
                        errors.fetch_add(1, Ordering::SeqCst);
                        println!("  Thread {} ERROR: {:?}", thread_id, e);
                    }
                }
                drop(guard); // Explicit unlock
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let total = num_threads * iterations_per_thread;
    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);

    println!("Mutex-protected Results ({} threads):", num_threads);
    println!("  Success: {}/{}", successes, total);
    println!("  Errors: {}", errors);
    println!("  Time: {:?}", elapsed);
    println!("  Rate: {:.1} translations/sec", total as f64 / elapsed.as_secs_f64());

    if errors == 0 {
        println!("\n✓ Mutex protection works correctly");
    }
}

// ============================================
// Test 4: Repeated stress test (run multiple times to catch race conditions)
// ============================================
#[test]
#[ignore]
fn test_repeated_stress() {
    println!("\n=== Repeated Stress Test ===");
    println!("Running multiple iterations to catch intermittent failures...\n");

    let (dll_path, dat_path) = get_engine_paths();

    let num_rounds = 5;
    let mut total_failures = 0;
    let mut total_ops = 0;

    for round in 0..num_rounds {
        // Create fresh engine each round
        let engine = EzTransEngine::new(&dll_path).expect("Failed to load DLL");
        engine.initialize_ex("CSUSER123455", &dat_path).expect("Failed to initialize");
        let engine = Arc::new(UnsafeEngineWrapper(engine));

        let num_threads = 4;
        let iterations = 20;
        let barrier = Arc::new(Barrier::new(num_threads));

        let failures = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..num_threads).map(|_| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            let failures = Arc::clone(&failures);

            thread::spawn(move || {
                barrier.wait();

                for _ in 0..iterations {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        engine.0.translate_mmntw("こんにちは")
                    }));

                    match result {
                        Ok(Ok(s)) if !is_corrupted("こんにちは", &s) && s.contains("안녕") => {}
                        _ => { failures.fetch_add(1, Ordering::SeqCst); }
                    }
                }
            })
        }).collect();

        for h in handles {
            let _ = h.join();
        }

        let round_failures = failures.load(Ordering::SeqCst);
        let round_ops = num_threads * iterations;
        total_failures += round_failures;
        total_ops += round_ops;

        println!("  Round {}: {}/{} failures", round + 1, round_failures, round_ops);

        // Small delay between rounds
        thread::sleep(Duration::from_millis(100));
    }

    println!("\nTotal Results:");
    println!("  Operations: {}", total_ops);
    println!("  Failures: {}", total_failures);
    println!("  Failure Rate: {:.2}%", total_failures as f64 / total_ops as f64 * 100.0);

    if total_failures > 0 {
        println!("\n✗ DLL is NOT THREAD-SAFE ({} failures across {} rounds)", total_failures, num_rounds);
    } else {
        println!("\n? No failures detected (may need more iterations)");
    }
}

// ============================================
// Test 5: Rapid fire test (maximum contention)
// ============================================
#[test]
#[ignore]
fn test_rapid_fire() {
    println!("\n=== Rapid Fire Test (Maximum Contention) ===");
    println!("Testing with NO delays between calls...\n");

    let (dll_path, dat_path) = get_engine_paths();
    let engine = EzTransEngine::new(&dll_path).expect("Failed to load DLL");
    engine.initialize_ex("CSUSER123455", &dat_path).expect("Failed to initialize");

    let engine = Arc::new(UnsafeEngineWrapper(engine));

    let num_threads = 8;
    let iterations_per_thread = 50;
    let barrier = Arc::new(Barrier::new(num_threads));

    let success = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads).map(|thread_id| {
        let engine = Arc::clone(&engine);
        let barrier = Arc::clone(&barrier);
        let success = Arc::clone(&success);
        let fail = Arc::clone(&fail);

        thread::spawn(move || {
            barrier.wait();

            for i in 0..iterations_per_thread {
                let text = if i % 2 == 0 { "おはよう" } else { "こんにちは" };

                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    engine.0.translate_mmntw(text)
                }));

                match result {
                    Ok(Ok(r)) if !r.is_empty() && !is_corrupted(text, &r) => {
                        success.fetch_add(1, Ordering::SeqCst);
                    }
                    _ => {
                        fail.fetch_add(1, Ordering::SeqCst);
                        if fail.load(Ordering::SeqCst) <= 10 {
                            println!("  Thread {} iteration {} failed", thread_id, i);
                        }
                    }
                }
                // NO DELAY - maximum contention
            }
        })
    }).collect();

    for h in handles {
        let _ = h.join();
    }

    let elapsed = start.elapsed();
    let total = num_threads * iterations_per_thread;
    let successes = success.load(Ordering::SeqCst);
    let failures = fail.load(Ordering::SeqCst);

    println!("\nRapid Fire Results ({} threads):", num_threads);
    println!("  Total: {}", total);
    println!("  Success: {}", successes);
    println!("  Failures: {}", failures);
    println!("  Time: {:?}", elapsed);
    println!("  Rate: {:.1} ops/sec", total as f64 / elapsed.as_secs_f64());

    let failure_rate = failures as f64 / total as f64 * 100.0;
    println!("\nFailure Rate: {:.1}%", failure_rate);

    if failure_rate > 5.0 {
        println!("\n✗ HIGH FAILURE RATE - DLL is clearly NOT thread-safe");
    } else if failure_rate > 0.0 {
        println!("\n✗ Some failures detected - DLL has thread safety issues");
    } else {
        println!("\n? No failures in this run");
    }
}

// ============================================
// Test 6: Memory corruption detection test
// ============================================
#[test]
#[ignore]
fn test_memory_corruption() {
    println!("\n=== Memory Corruption Detection Test ===");
    println!("Checking if concurrent access causes memory corruption...\n");

    let (dll_path, dat_path) = get_engine_paths();
    let engine = EzTransEngine::new(&dll_path).expect("Failed to load DLL");
    engine.initialize_ex("CSUSER123455", &dat_path).expect("Failed to initialize");

    let engine = Arc::new(UnsafeEngineWrapper(engine));

    // Use distinct inputs that should produce clearly different outputs
    let inputs_outputs = vec![
        ("おはよう", vec!["안녕", "좋은 아침"]),
        ("さようなら", vec!["안녕", "작별"]),
        ("ありがとう", vec!["감사", "고마"]),
        ("すみません", vec!["실례", "미안", "죄송"]),
    ];

    let num_threads = 4;
    let iterations = 30;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mixed_output_count = Arc::new(AtomicUsize::new(0));
    let garbage_output_count = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..num_threads).map(|thread_id| {
        let engine = Arc::clone(&engine);
        let barrier = Arc::clone(&barrier);
        let mixed = Arc::clone(&mixed_output_count);
        let garbage = Arc::clone(&garbage_output_count);
        let success = Arc::clone(&success_count);
        let ios = inputs_outputs.clone();

        thread::spawn(move || {
            let (my_input, my_expected) = &ios[thread_id % ios.len()];

            barrier.wait();

            for _ in 0..iterations {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    engine.0.translate_mmntw(my_input)
                }));

                match result {
                    Ok(Ok(output)) => {
                        // Check for garbage
                        if is_corrupted(my_input, &output) {
                            garbage.fetch_add(1, Ordering::SeqCst);
                            println!("  Thread {} GARBAGE: '{}' -> '{}'", thread_id, my_input, output);
                        }
                        // Check if output contains expected Korean
                        else if my_expected.iter().any(|exp| output.contains(exp)) {
                            success.fetch_add(1, Ordering::SeqCst);
                        }
                        // Check if it contains output from another thread's input (mixing)
                        else {
                            // Check if it looks like output from a different input
                            let other_outputs: Vec<&str> = ios.iter()
                                .filter(|(inp, _)| inp != my_input)
                                .flat_map(|(_, exp)| exp.iter().map(|s| *s))
                                .collect();

                            if other_outputs.iter().any(|other| output.contains(other)) {
                                mixed.fetch_add(1, Ordering::SeqCst);
                                println!("  Thread {} MIXED OUTPUT: '{}' -> '{}' (expected one of {:?})",
                                    thread_id, my_input, output, my_expected);
                            } else {
                                // Unknown output - could be valid translation we didn't expect
                                success.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                    _ => {
                        garbage.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        })
    }).collect();

    for h in handles {
        let _ = h.join();
    }

    let total = num_threads * iterations;
    let successes = success_count.load(Ordering::SeqCst);
    let mixed = mixed_output_count.load(Ordering::SeqCst);
    let garbage = garbage_output_count.load(Ordering::SeqCst);

    println!("\nMemory Corruption Results:");
    println!("  Total operations: {}", total);
    println!("  Success: {}", successes);
    println!("  Mixed/Wrong output: {}", mixed);
    println!("  Garbage output: {}", garbage);

    if mixed > 0 {
        println!("\n✗ MEMORY CORRUPTION DETECTED: Outputs are getting mixed between threads!");
        println!("  This proves the DLL shares internal buffers unsafely.");
    }
    if garbage > 0 {
        println!("\n✗ GARBAGE OUTPUT DETECTED: Memory is being corrupted!");
    }
    if mixed == 0 && garbage == 0 {
        println!("\n? No obvious corruption detected in this run");
    }
}
