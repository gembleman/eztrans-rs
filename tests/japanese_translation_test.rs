// Japanese Translation Validation Test
// Tests Japanese character ranges and validates translation output
// Run with: cargo test --target i686-pc-windows-msvc --test japanese_translation_test -- --include-ignored --test-threads=1 --nocapture

use eztrans_rs::EzTransEngine;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn get_engine_paths() -> (String, String) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dll_path = format!("{}/eztrans_dll/J2KEngine.dll", manifest_dir);
    let dat_path = format!("{}/eztrans_dll/Dat", manifest_dir);
    (dll_path, dat_path)
}

// Japanese Unicode ranges (실용적인 일본어 범위)
const JAPANESE_RANGES: [(u32, u32); 6] = [
    (0x3000, 0x303F), // Japanese-style punctuation
    (0x3040, 0x309F), // Hiragana
    (0x30A0, 0x30FF), // Katakana
    (0xFF00, 0xFFEF), // Full-width roman characters and half-width katakana
    (0x4E00, 0x9FAF), // CJK unified ideographs - Common and uncommon kanji
    (0x3400, 0x4DBF), // CJK unified ideographs Extension A - Rare kanji
];

/// 일본어 범위의 총 코드포인트 수 계산
fn total_japanese_codepoints() -> u32 {
    JAPANESE_RANGES.iter().map(|(s, e)| e - s + 1).sum()
}

/// 절대 인덱스를 일본어 코드포인트로 변환
fn absolute_to_japanese_codepoint(abs_index: u32) -> Option<u32> {
    let mut remaining = abs_index;
    for (start, end) in JAPANESE_RANGES {
        let range_size = end - start + 1;
        if remaining < range_size {
            return Some(start + remaining);
        }
        remaining -= range_size;
    }
    None
}

/// 한국어 문자인지 확인 (한글 범위 체크)
fn is_korean_char(c: char) -> bool {
    let code = c as u32;
    matches!(
        code,
        0xAC00..=0xD7A3 | // Hangul Syllables
        0x1100..=0x11FF | // Hangul Jamo
        0x3130..=0x318F | // Hangul Compatibility Jamo
        0xA960..=0xA97F | // Hangul Jamo Extended-A
        0xD7B0..=0xD7FF   // Hangul Jamo Extended-B
    )
}

/// 문자열에 한국어가 포함되어 있는지 확인
fn contains_korean(s: &str) -> bool {
    s.chars().any(is_korean_char)
}

/// CSV 레코드 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NonKoreanResult {
    codepoint: String,
    character: String,
    translation: String,
    has_korean: bool,
    error: String,
}

/// 워커 메시지
#[derive(Serialize, Deserialize, Debug)]
enum WorkerMessage {
    Progress {
        worker_id: usize,
        current_code: u32,
        tested: u32,
        non_korean_count: u32,
    },
    ChunkResult {
        worker_id: usize,
        results: Vec<NonKoreanResult>,
    },
    Complete {
        worker_id: usize,
        total_tested: u32,
        non_korean_count: u32,
        elapsed_secs: f64,
    },
    Error {
        worker_id: usize,
        message: String,
    },
}

fn send_message(msg: &WorkerMessage) {
    let json = serde_json::to_string(msg).unwrap();
    println!("{}", json);
    std::io::stdout().flush().ok();
}

/// 워커 프로세스 - 일본어 번역 검증
fn japanese_scan_worker_process(
    worker_id: usize,
    abs_start: u32,
    abs_end: u32,
    dll_path: &str,
    dat_path: &str,
) {
    let start_time = Instant::now();

    let engine = match EzTransEngine::new(dll_path) {
        Ok(e) => e,
        Err(err) => {
            send_message(&WorkerMessage::Error {
                worker_id,
                message: format!("Failed to load DLL: {:?}", err),
            });
            return;
        }
    };

    if let Err(err) = engine.initialize_ex("CSUSER123455", dat_path) {
        send_message(&WorkerMessage::Error {
            worker_id,
            message: format!("Failed to initialize engine: {:?}", err),
        });
        return;
    }

    let mut non_korean_results = Vec::new();
    let mut pending_results = Vec::new();
    let mut total_tested = 0u32;
    let mut non_korean_count = 0u32;
    let mut last_progress = Instant::now();
    let mut last_chunk_send = Instant::now();

    const CHUNK_SIZE: usize = 1000;
    const PROGRESS_INTERVAL_MS: u64 = 500;
    const CHUNK_INTERVAL_SECS: u64 = 5;

    for abs_idx in abs_start..=abs_end {
        let Some(code) = absolute_to_japanese_codepoint(abs_idx) else {
            continue;
        };

        let Some(c) = char::from_u32(code) else {
            continue;
        };

        total_tested += 1;

        // 일본어 문자를 번역
        let test_str = c.to_string();
        match engine.translate_mmntw(&test_str) {
            Ok(translated) => {
                // 번역 결과에 한국어가 없거나, 원본과 동일한 경우 기록
                let has_korean = contains_korean(&translated);
                let is_unchanged = translated == test_str;

                if !has_korean || is_unchanged {
                    non_korean_count += 1;
                    let result = NonKoreanResult {
                        codepoint: format!("U+{:04X}", code),
                        character: c.to_string(),
                        translation: translated.clone(),
                        has_korean,
                        error: String::new(),
                    };
                    non_korean_results.push(result.clone());
                    pending_results.push(result);
                }
            }
            Err(e) => {
                non_korean_count += 1;
                let result = NonKoreanResult {
                    codepoint: format!("U+{:04X}", code),
                    character: c.to_string(),
                    translation: String::new(),
                    has_korean: false,
                    error: format!("{:?}", e),
                };
                non_korean_results.push(result.clone());
                pending_results.push(result);
            }
        }

        // 진행률 업데이트
        if last_progress.elapsed() >= Duration::from_millis(PROGRESS_INTERVAL_MS) {
            send_message(&WorkerMessage::Progress {
                worker_id,
                current_code: code,
                tested: total_tested,
                non_korean_count,
            });
            last_progress = Instant::now();
        }

        // 청크 결과 전송
        if pending_results.len() >= CHUNK_SIZE
            || (last_chunk_send.elapsed() >= Duration::from_secs(CHUNK_INTERVAL_SECS)
                && !pending_results.is_empty())
        {
            send_message(&WorkerMessage::ChunkResult {
                worker_id,
                results: pending_results.clone(),
            });
            pending_results.clear();
            last_chunk_send = Instant::now();
        }
    }

    // 남은 청크 전송
    if !pending_results.is_empty() {
        send_message(&WorkerMessage::ChunkResult {
            worker_id,
            results: pending_results,
        });
    }

    let elapsed = start_time.elapsed();

    send_message(&WorkerMessage::Complete {
        worker_id,
        total_tested,
        non_korean_count,
        elapsed_secs: elapsed.as_secs_f64(),
    });
}

/// 워커 전용 테스트
#[test]
#[ignore]
fn japanese_scan_worker() {
    if let Ok(worker_params) = env::var("JAPANESE_SCAN_WORKER") {
        let parts: Vec<&str> = worker_params.split('|').collect();
        if parts.len() == 5 {
            let worker_id: usize = parts[0].parse().unwrap();
            let abs_start: u32 = parts[1].parse().unwrap();
            let abs_end: u32 = parts[2].parse().unwrap();
            let dll_path = parts[3];
            let dat_path = parts[4];

            japanese_scan_worker_process(worker_id, abs_start, abs_end, dll_path, dat_path);
            std::process::exit(0);
        }
    }
}

#[derive(Debug)]
enum CoordinatorMessage {
    WorkerMessage {
        worker_id: usize,
        msg: WorkerMessage,
    },
    WorkerEof {
        #[allow(dead_code)]
        worker_id: usize,
    },
    WorkerError {
        worker_id: usize,
        error: String,
    },
}

/// 워커 상태 추적
#[derive(Debug, Clone)]
struct WorkerStatus {
    tested: u32,
    non_korean_count: u32,
    completed: bool,
}

impl WorkerStatus {
    fn new() -> Self {
        Self {
            tested: 0,
            non_korean_count: 0,
            completed: false,
        }
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

/// 실시간 진행률 표시
fn print_progress_dashboard(
    statuses: &[WorkerStatus],
    total_codepoints: u32,
    start_time: Instant,
) {
    let total_tested: u32 = statuses.iter().map(|s| s.tested).sum();
    let total_non_korean: u32 = statuses.iter().map(|s| s.non_korean_count).sum();
    let completed_count = statuses.iter().filter(|s| s.completed).count();

    let elapsed = start_time.elapsed();
    let overall_progress = (total_tested as f64 / total_codepoints as f64) * 100.0;

    // ETA 계산
    let eta_str = if total_tested > 0 && elapsed.as_secs() > 0 {
        let rate = total_tested as f64 / elapsed.as_secs_f64();
        let remaining = total_codepoints.saturating_sub(total_tested);
        let eta_secs = (remaining as f64 / rate) as u64;
        format_duration(Duration::from_secs(eta_secs))
    } else {
        "calculating...".to_string()
    };

    // 진행률 바
    let bar_width = 30;
    let filled = ((overall_progress / 100.0) * bar_width as f64) as usize;
    let bar: String = (0..bar_width)
        .map(|i| if i < filled { '#' } else { '-' })
        .collect();

    print!(
        "\r[{}] {:.1}% | {} tested | {} non-Korean | ETA: {} | Workers: {}/{}   ",
        bar,
        overall_progress,
        total_tested,
        total_non_korean,
        eta_str,
        completed_count,
        statuses.len()
    );
    std::io::stdout().flush().ok();
}

#[test]
#[ignore]
fn scan_japanese_chars_8_procs() {
    if env::var("JAPANESE_SCAN_WORKER").is_ok() {
        return;
    }
    scan_japanese_multiprocess(Some(8));
}

fn scan_japanese_multiprocess(num_processes_opt: Option<usize>) {
    println!("\n=== JAPANESE CHARACTER TRANSLATION TEST ===\n");

    let (dll_path, dat_path) = get_engine_paths();

    let num_processes = num_processes_opt.unwrap_or_else(|| num_cpus::get().min(8));
    let total_codepoints = total_japanese_codepoints();

    println!("Configuration:");
    println!("  Worker processes: {}", num_processes);
    println!("  CPU cores: {}", num_cpus::get());
    println!("  Total Japanese codepoints: {}", total_codepoints);
    println!();

    println!("Japanese Unicode Ranges:");
    for (start, end) in &JAPANESE_RANGES {
        println!(
            "  U+{:04X}..U+{:04X} ({} chars)",
            start,
            end,
            end - start + 1
        );
    }
    println!();

    // 작업 분배
    let chunk_size = total_codepoints / num_processes as u32;
    let mut work_assignments: Vec<(usize, u32, u32)> = Vec::new();

    for worker_id in 0..num_processes {
        let abs_start = worker_id as u32 * chunk_size;
        let abs_end = if worker_id == num_processes - 1 {
            total_codepoints - 1
        } else {
            (worker_id as u32 + 1) * chunk_size - 1
        };
        work_assignments.push((worker_id, abs_start, abs_end));
    }

    println!("Work distribution:");
    for (worker_id, abs_start, abs_end) in &work_assignments {
        let start_code = absolute_to_japanese_codepoint(*abs_start).unwrap_or(0);
        let end_code = absolute_to_japanese_codepoint(*abs_end).unwrap_or(0);
        println!(
            "  Worker {}: U+{:04X}..U+{:04X} ({} codepoints)",
            worker_id,
            start_code,
            end_code,
            abs_end - abs_start + 1
        );
    }
    println!();

    let overall_start_time = Instant::now();
    let current_exe = env::current_exe().expect("Failed to get current exe path");

    let (tx, rx) = mpsc::channel::<CoordinatorMessage>();

    let mut worker_statuses: Vec<WorkerStatus> =
        (0..num_processes).map(|_| WorkerStatus::new()).collect();
    let mut workers_completed = 0usize;
    let mut all_non_korean_results: Vec<NonKoreanResult> = Vec::new();
    let mut total_tested = 0u32;

    let mut reader_threads = Vec::new();

    for (worker_id, abs_start, abs_end) in &work_assignments {
        let worker_params = format!(
            "{}|{}|{}|{}|{}",
            worker_id, abs_start, abs_end, dll_path, dat_path
        );

        let mut cmd = Command::new(&current_exe);
        cmd.env("JAPANESE_SCAN_WORKER", worker_params)
            .arg("japanese_scan_worker")
            .arg("--exact")
            .arg("--ignored")
            .arg("--nocapture")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("Failed to get stdout");
                let tx_clone = tx.clone();
                let wid = *worker_id;

                let reader_thread = thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        match line {
                            Ok(line) => {
                                if let Ok(msg) = serde_json::from_str::<WorkerMessage>(&line) {
                                    let _ = tx_clone.send(CoordinatorMessage::WorkerMessage {
                                        worker_id: wid,
                                        msg,
                                    });
                                }
                            }
                            Err(e) => {
                                let _ = tx_clone.send(CoordinatorMessage::WorkerError {
                                    worker_id: wid,
                                    error: e.to_string(),
                                });
                                break;
                            }
                        }
                    }
                    let _ = tx_clone.send(CoordinatorMessage::WorkerEof { worker_id: wid });
                    let _ = child.wait();
                });

                reader_threads.push(reader_thread);
                println!("Spawned worker {}", worker_id);
            }
            Err(e) => {
                eprintln!("Failed to spawn worker {}: {}", worker_id, e);
                workers_completed += 1;
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    drop(tx);

    println!("\nProcessing...\n");

    let mut last_dashboard_update = Instant::now();
    let dashboard_interval = Duration::from_millis(500);

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(coord_msg) => match coord_msg {
                CoordinatorMessage::WorkerMessage { worker_id, msg } => match msg {
                    WorkerMessage::Progress {
                        tested,
                        non_korean_count,
                        ..
                    } => {
                        if worker_id < worker_statuses.len() {
                            worker_statuses[worker_id].tested = tested;
                            worker_statuses[worker_id].non_korean_count = non_korean_count;
                        }
                    }
                    WorkerMessage::ChunkResult { results, .. } => {
                        all_non_korean_results.extend(results);
                    }
                    WorkerMessage::Complete {
                        worker_id: wid,
                        total_tested: tested,
                        non_korean_count,
                        ..
                    } => {
                        if wid < worker_statuses.len() {
                            worker_statuses[wid].tested = tested;
                            worker_statuses[wid].non_korean_count = non_korean_count;
                            worker_statuses[wid].completed = true;
                        }
                        total_tested += tested;
                    }
                    WorkerMessage::Error {
                        worker_id: wid,
                        message,
                    } => {
                        eprintln!("\n[Worker {}] Error: {}", wid, message);
                    }
                },
                CoordinatorMessage::WorkerEof { .. } => {
                    workers_completed += 1;
                }
                CoordinatorMessage::WorkerError { worker_id, error } => {
                    eprintln!("\n[Worker {}] Read error: {}", worker_id, error);
                    workers_completed += 1;
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        // 대시보드 업데이트
        if last_dashboard_update.elapsed() >= dashboard_interval {
            print_progress_dashboard(&worker_statuses, total_codepoints, overall_start_time);
            last_dashboard_update = Instant::now();
        }

        if workers_completed >= num_processes {
            break;
        }
    }

    for thread in reader_threads {
        let _ = thread.join();
    }

    let total_elapsed = overall_start_time.elapsed();

    println!(
        "\n\n=== SCAN COMPLETE in {} ===\n",
        format_duration(total_elapsed)
    );
    println!("Total Japanese characters tested: {}", total_tested);
    println!("Non-Korean translations found: {}", all_non_korean_results.len());

    // CSV 파일로 저장
    let csv_path = "japanese_non_korean_translations.csv";
    let mut wtr = csv::Writer::from_path(csv_path).expect("Failed to create CSV file");

    // CSV 헤더
    wtr.write_record(&["Codepoint", "Character", "Translation", "Has Korean", "Error"])
        .expect("Failed to write CSV header");

    // CSV 데이터
    for result in &all_non_korean_results {
        wtr.write_record(&[
            &result.codepoint,
            &result.character,
            &result.translation,
            &result.has_korean.to_string(),
            &result.error,
        ])
        .expect("Failed to write CSV record");
    }

    wtr.flush().expect("Failed to flush CSV writer");

    println!("\nResults saved to: {}", csv_path);

    // 통계 출력
    let error_count = all_non_korean_results.iter().filter(|r| !r.error.is_empty()).count();
    let unchanged_count = all_non_korean_results
        .iter()
        .filter(|r| r.translation == r.character && r.error.is_empty())
        .count();

    println!("\n=== STATISTICS ===");
    println!("Translation errors: {}", error_count);
    println!("Unchanged (not translated): {}", unchanged_count);
    println!("Translated to non-Korean: {}", all_non_korean_results.len() - error_count - unchanged_count);
}
