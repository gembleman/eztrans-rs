// Default Translate Detection Test
// Tests default_translate for ALL valid Unicode codepoints (U+0000 to U+10FFFF) and detects:
// 1. "?" in translation
// 2. "□" in translation
// 3. Translation differs from original (only for non-Korean characters)
// Run with: cargo test --target i686-pc-windows-msvc --test default_translate_detection_test -- --include-ignored --test-threads=1 --nocapture

use eztrans_rs::EzTransEngine;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
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

/// 유효한 유니코드 범위 (surrogate 제외)
const UNICODE_RANGES: [(u32, u32); 3] = [
    (0x0000, 0xD7FF),    // BMP before surrogates
    (0xE000, 0xFFFF),    // BMP after surrogates
    (0x10000, 0x10FFFF), // Supplementary planes
];

/// 허용하는 문자 목록
const ALLOWED_CHARS: [u32; 161] = [
    0x003000, 0x003001, 0x003002, 0x003007, 0x00301C, 0x0030FC, 0x00FF01, 0x00FF05, 0x00FF08,
    0x00FF09, 0x00FF0A, 0x00FF0C, 0x00FF0E, 0x00FF0F, 0x00FF10, 0x00FF11, 0x00FF12, 0x00FF13,
    0x00FF14, 0x00FF15, 0x00FF16, 0x00FF17, 0x00FF18, 0x00FF19, 0x00FF1A, 0x00FF1D, 0x00FF21,
    0x00FF22, 0x00FF23, 0x00FF24, 0x00FF25, 0x00FF26, 0x00FF27, 0x00FF28, 0x00FF29, 0x00FF2A,
    0x00FF2B, 0x00FF2C, 0x00FF2D, 0x00FF2E, 0x00FF2F, 0x00FF30, 0x00FF31, 0x00FF32, 0x00FF33,
    0x00FF34, 0x00FF35, 0x00FF36, 0x00FF37, 0x00FF38, 0x00FF39, 0x00FF3A, 0x00FF41, 0x00FF42,
    0x00FF43, 0x00FF44, 0x00FF45, 0x00FF46, 0x00FF47, 0x00FF48, 0x00FF49, 0x00FF4A, 0x00FF4B,
    0x00FF4C, 0x00FF4D, 0x00FF4E, 0x00FF4F, 0x00FF50, 0x00FF51, 0x00FF52, 0x00FF53, 0x00FF54,
    0x00FF55, 0x00FF56, 0x00FF57, 0x00FF58, 0x00FF59, 0x00FF5A, 0x00FF5E, 0x00FF61, 0x00FF64,
    0x00FF70, 0x0000B4, 0x0000B5, 0x0000B8, 0x0000BA, 0x0000BB, 0x002010, 0x002014, 0x002015,
    0x002022, 0x002025, 0x002032, 0x002033, 0x002266, 0x002267, 0x0025A1, 0x00266F, 0x00301D,
    0x00301F, 0x003054, 0x003055, 0x00305E, 0x00305F, 0x003062, 0x003065, 0x00306A, 0x003079,
    0x00307E, 0x003085, 0x003086, 0x003087, 0x003088, 0x00308A, 0x00308C, 0x003090, 0x003091,
    0x00309B, 0x00309C, 0x0030A3, 0x0030A5, 0x0030A9, 0x0030C3, 0x0030E3, 0x0030E5, 0x0030E7,
    0x0030EE, 0x0030F0, 0x0030F1, 0x0030F2, 0x0030F3, 0x0030F5, 0x0030F6, 0x0030FB, 0x003231,
    0x004E00, 0x004E03, 0x004E09, 0x004E5D, 0x004E8C, 0x004E94, 0x00516B, 0x00516D, 0x005341,
    0x0056DB, 0x00FF62, 0x00FF63, 0x00FF65, 0x00FF66, 0x00FF67, 0x00FF68, 0x00FF69, 0x00FF6A,
    0x00FF6B, 0x00FF6C, 0x00FF6D, 0x00FF6E, 0x00FF6F, 0x00FF9D, 0x00FF9E, 0x00FF9F,
];

/// 총 유효 코드포인트 수 계산
fn total_valid_codepoints() -> u32 {
    UNICODE_RANGES.iter().map(|(s, e)| e - s + 1).sum()
}

/// 절대 인덱스를 실제 코드포인트로 변환 (surrogate 건너뜀)
fn absolute_to_codepoint(abs_index: u32) -> Option<u32> {
    let mut remaining = abs_index;
    for (start, end) in UNICODE_RANGES {
        let range_size = end - start + 1;
        if remaining < range_size {
            return Some(start + remaining);
        }
        remaining -= range_size;
    }
    None
}

/// 한글 범위 체크 함수
fn is_korean(s: &str) -> bool {
    s.chars().any(|c| {
        let code = c as u32;
        // 한글 음절 범위 (가-힣)
        (0xAC00..=0xD7A3).contains(&code) ||
        // 한글 자모 범위
        (0x1100..=0x11FF).contains(&code) ||
        (0x3130..=0x318F).contains(&code) ||
        (0xA960..=0xA97F).contains(&code) ||
        (0xD7B0..=0xD7FF).contains(&code)
    })
}

/// CSV 레코드 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DetectionResult {
    code: u32,
    character: String,
    translation: String,
    issue_type: String, // "question_mark", "square", "different", "none"
}

/// 워커 메시지
#[derive(Serialize, Deserialize, Debug)]
enum WorkerMessage {
    Progress {
        worker_id: usize,
        current_code: u32,
        tested: u32,
        detected: u32,
    },
    ChunkResult {
        worker_id: usize,
        results: Vec<DetectionResult>,
    },
    Complete {
        worker_id: usize,
        total_tested: u32,
        total_detected: u32,
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

/// 워커 프로세스 - 문자 번역 및 감지
fn detection_worker_process(
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

    let mut all_results = Vec::new();
    let mut pending_results = Vec::new();
    let mut total_tested = 0u32;
    let mut total_detected = 0u32;
    let mut last_progress = Instant::now();
    let mut last_chunk_send = Instant::now();

    const CHUNK_SIZE: usize = 1000;
    const PROGRESS_INTERVAL_MS: u64 = 500;
    const CHUNK_INTERVAL_SECS: u64 = 5;

    for abs_idx in abs_start..=abs_end {
        let Some(code) = absolute_to_codepoint(abs_idx) else {
            continue;
        };

        total_tested += 1;

        if let Some(c) = char::from_u32(code) {
            let test_str = c.to_string();

            // default_translate로 번역
            match engine.default_translate(&test_str) {
                Ok(translated) => {
                    let mut issue_type = "none".to_string();

                    // ALLOWED_CHARS에 포함되어 있는지 확인
                    let is_allowed = ALLOWED_CHARS.contains(&code);

                    // 1. "□" 포함 확인
                    if translated.contains('□') {
                        issue_type = if is_allowed {
                            "allowed".to_string()
                        } else {
                            "square".to_string()
                        };
                        total_detected += 1;
                    }
                    // 2. "?" 포함 확인 (원본이 "?"가 아닌 경우)
                    else if c != '?' && translated.contains('?') {
                        issue_type = if is_allowed {
                            "allowed".to_string()
                        } else {
                            "question_mark".to_string()
                        };
                        total_detected += 1;
                    }
                    // 3. 번역문이 원문과 다른 경우 (한국어가 아닌 경우만)
                    else if translated != test_str && !is_korean(&translated) {
                        issue_type = if is_allowed {
                            "allowed".to_string()
                        } else {
                            "different".to_string()
                        };
                        total_detected += 1;
                    }

                    // 문제가 있는 경우만 기록
                    if issue_type != "none" {
                        let result = DetectionResult {
                            code,
                            character: test_str,
                            translation: translated,
                            issue_type,
                        };
                        all_results.push(result.clone());
                        pending_results.push(result);
                    }
                }
                Err(e) => {
                    // 에러도 기록
                    let result = DetectionResult {
                        code,
                        character: c.to_string(),
                        translation: format!("ERROR: {:?}", e),
                        issue_type: "error".to_string(),
                    };
                    all_results.push(result.clone());
                    pending_results.push(result);
                    total_detected += 1;
                }
            };
        }

        // 진행률 업데이트
        if last_progress.elapsed() >= Duration::from_millis(PROGRESS_INTERVAL_MS) {
            send_message(&WorkerMessage::Progress {
                worker_id,
                current_code: code,
                tested: total_tested,
                detected: total_detected,
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
        total_detected,
        elapsed_secs: elapsed.as_secs_f64(),
    });
}

/// 워커 전용 테스트
#[test]
#[ignore]
fn detection_worker() {
    if let Ok(worker_params) = env::var("DETECTION_WORKER") {
        let parts: Vec<&str> = worker_params.split('|').collect();
        if parts.len() == 5 {
            let worker_id: usize = parts[0].parse().unwrap();
            let abs_start: u32 = parts[1].parse().unwrap();
            let abs_end: u32 = parts[2].parse().unwrap();
            let dll_path = parts[3];
            let dat_path = parts[4];

            detection_worker_process(worker_id, abs_start, abs_end, dll_path, dat_path);
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
    detected: u32,
    completed: bool,
}

impl WorkerStatus {
    fn new() -> Self {
        Self {
            tested: 0,
            detected: 0,
            completed: false,
        }
    }
}

/// 실시간 진행률 표시
fn print_progress_dashboard(statuses: &[WorkerStatus], total_codepoints: u32, start_time: Instant) {
    let total_tested: u32 = statuses.iter().map(|s| s.tested).sum();
    let total_detected: u32 = statuses.iter().map(|s| s.detected).sum();
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
        "\r[{}] {:.1}% | {} tested | {} detected | ETA: {} | Workers: {}/{}   ",
        bar,
        overall_progress,
        total_tested,
        total_detected,
        eta_str,
        completed_count,
        statuses.len()
    );
    std::io::stdout().flush().ok();
}

#[test]
#[ignore]
fn detect_unicode_issues_8_procs() {
    if env::var("DETECTION_WORKER").is_ok() {
        return;
    }
    detect_unicode_issues_multiprocess(Some(8));
}

fn detect_unicode_issues_multiprocess(num_processes_opt: Option<usize>) {
    println!("\n=== DEFAULT_TRANSLATE DETECTION TEST ===\n");
    println!("Testing ALL valid Unicode codepoints (U+0000 to U+10FFFF)\n");

    let (dll_path, dat_path) = get_engine_paths();

    let num_processes = num_processes_opt.unwrap_or_else(|| num_cpus::get().min(8));
    let total_codepoints = total_valid_codepoints();

    println!("Configuration:");
    println!("  Worker processes: {}", num_processes);
    println!("  CPU cores: {}", num_cpus::get());
    println!("  Total codepoints: {}", total_codepoints);
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
        let start_code = absolute_to_codepoint(*abs_start).unwrap_or(0);
        let end_code = absolute_to_codepoint(*abs_end).unwrap_or(0);
        println!(
            "  Worker {}: U+{:06X}..U+{:06X} ({} codepoints)",
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
    let mut all_results: Vec<DetectionResult> = Vec::new();
    let mut total_tested = 0u32;

    let mut reader_threads = Vec::new();

    for (worker_id, abs_start, abs_end) in &work_assignments {
        let worker_params = format!(
            "{}|{}|{}|{}|{}",
            worker_id, abs_start, abs_end, dll_path, dat_path
        );

        let mut cmd = Command::new(&current_exe);
        cmd.env("DETECTION_WORKER", worker_params)
            .arg("detection_worker")
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
                        tested, detected, ..
                    } => {
                        if worker_id < worker_statuses.len() {
                            worker_statuses[worker_id].tested = tested;
                            worker_statuses[worker_id].detected = detected;
                        }
                    }
                    WorkerMessage::ChunkResult { results, .. } => {
                        all_results.extend(results);
                    }
                    WorkerMessage::Complete {
                        worker_id: wid,
                        total_tested: tested,
                        total_detected,
                        ..
                    } => {
                        if wid < worker_statuses.len() {
                            worker_statuses[wid].tested = tested;
                            worker_statuses[wid].detected = total_detected;
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
    println!("Total codepoints tested: {}", total_tested);
    println!("Total issues detected: {}", all_results.len());

    // CSV 파일로 저장 (UTF-8 BOM 포함)
    let csv_path = "default_translate_detection.csv";

    match File::create(csv_path) {
        Ok(mut file) => {
            // UTF-8 BOM
            file.write_all("\u{FEFF}".as_bytes()).ok();

            // CSV 헤더
            writeln!(file, "Code,Character,Translation,IssueType").ok();

            // 결과 정렬 (코드 순)
            all_results.sort_by_key(|r| r.code);

            // CSV 데이터 작성
            for result in &all_results {
                let char_display = char::from_u32(result.code)
                    .map(|c| format!("{}", c))
                    .unwrap_or_else(|| "N/A".to_string());

                // CSV 이스케이프 처리
                let translation_escaped = result.translation.replace('"', "\"\"");

                writeln!(
                    file,
                    "U+{:06X},\"{}\",\"{}\",{}",
                    result.code, char_display, translation_escaped, result.issue_type
                )
                .ok();
            }

            println!("\nResults saved to: {}", csv_path);
        }
        Err(e) => {
            eprintln!("\nFailed to create CSV file: {}", e);
        }
    }

    // 통계 출력
    let question_mark_count = all_results
        .iter()
        .filter(|r| r.issue_type == "question_mark")
        .count();
    let square_count = all_results
        .iter()
        .filter(|r| r.issue_type == "square")
        .count();
    let different_count = all_results
        .iter()
        .filter(|r| r.issue_type == "different")
        .count();
    let error_count = all_results
        .iter()
        .filter(|r| r.issue_type == "error")
        .count();
    let allowed_count = all_results
        .iter()
        .filter(|r| r.issue_type == "allowed")
        .count();

    println!("\n=== STATISTICS ===");
    println!("Contains '?': {}", question_mark_count);
    println!("Contains '□': {}", square_count);
    println!("Differs from original (non-Korean): {}", different_count);
    println!("Translation errors: {}", error_count);
    println!("Allowed characters: {}", allowed_count);

    // 감지된 케이스 샘플 출력
    println!("\n=== DETECTED ISSUES (First 10 each) ===");

    println!("\nContains '?':");
    for (i, result) in all_results
        .iter()
        .filter(|r| r.issue_type == "question_mark")
        .take(10)
        .enumerate()
    {
        println!(
            "  {}. U+{:06X} ({}) -> {}",
            i + 1,
            result.code,
            result.character,
            result.translation
        );
    }

    println!("\nContains '□':");
    for (i, result) in all_results
        .iter()
        .filter(|r| r.issue_type == "square")
        .take(10)
        .enumerate()
    {
        println!(
            "  {}. U+{:06X} ({}) -> {}",
            i + 1,
            result.code,
            result.character,
            result.translation
        );
    }

    println!("\nDiffers from original (non-Korean):");
    for (i, result) in all_results
        .iter()
        .filter(|r| r.issue_type == "different")
        .take(10)
        .enumerate()
    {
        println!(
            "  {}. U+{:06X} ({}) -> {}",
            i + 1,
            result.code,
            result.character,
            result.translation
        );
    }

    println!("\nAllowed characters:");
    for (i, result) in all_results
        .iter()
        .filter(|r| r.issue_type == "allowed")
        .take(10)
        .enumerate()
    {
        println!(
            "  {}. U+{:06X} ({}) -> {}",
            i + 1,
            result.code,
            result.character,
            result.translation
        );
    }
}
