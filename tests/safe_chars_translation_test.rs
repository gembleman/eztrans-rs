// Safe Characters Translation Test
// Reads safe_chars_range.txt and tests translation for each character
// Run with: cargo test --target i686-pc-windows-msvc --test safe_chars_translation_test -- --include-ignored --test-threads=1 --nocapture

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

/// CSV 레코드 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TranslationResult {
    codepoint: String,
    character: String,
    translation: String,
    error: String,
}

/// 워커 메시지
#[derive(Serialize, Deserialize, Debug)]
enum WorkerMessage {
    Progress {
        worker_id: usize,
        current_code: u32,
        tested: u32,
    },
    ChunkResult {
        worker_id: usize,
        results: Vec<TranslationResult>,
    },
    Complete {
        worker_id: usize,
        total_tested: u32,
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

/// safe_chars_range.txt 파일을 파싱하여 코드포인트 벡터 반환
fn parse_safe_chars_file(file_path: &str) -> std::io::Result<Vec<u32>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut codepoints = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 범위 형식: 0x000000..=0x000080
        if line.contains("..=") {
            let parts: Vec<&str> = line.split("..=").collect();
            if parts.len() == 2 {
                let start = u32::from_str_radix(parts[0].trim_start_matches("0x"), 16).ok();
                let end = u32::from_str_radix(parts[1].trim_start_matches("0x"), 16).ok();

                if let (Some(start), Some(end)) = (start, end) {
                    for code in start..=end {
                        codepoints.push(code);
                    }
                }
            }
        } else {
            // 단일 값: 0x00029E
            if let Some(hex) = line.strip_prefix("0x") {
                if let Ok(code) = u32::from_str_radix(hex, 16) {
                    codepoints.push(code);
                }
            }
        }
    }

    Ok(codepoints)
}

/// 워커 프로세스 - 문자 번역
fn safe_chars_worker_process(
    worker_id: usize,
    start_idx: usize,
    end_idx: usize,
    dll_path: &str,
    dat_path: &str,
    safe_chars_path: &str,
) {
    let start_time = Instant::now();

    // safe_chars_range.txt 파싱
    let codepoints = match parse_safe_chars_file(safe_chars_path) {
        Ok(cp) => cp,
        Err(e) => {
            send_message(&WorkerMessage::Error {
                worker_id,
                message: format!("Failed to parse safe_chars_range.txt: {:?}", e),
            });
            return;
        }
    };

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

    let mut results = Vec::new();
    let mut pending_results = Vec::new();
    let mut total_tested = 0u32;
    let mut last_progress = Instant::now();
    let mut last_chunk_send = Instant::now();

    const CHUNK_SIZE: usize = 1000;
    const PROGRESS_INTERVAL_MS: u64 = 500;
    const CHUNK_INTERVAL_SECS: u64 = 5;

    let end_idx = end_idx.min(codepoints.len());

    for idx in start_idx..end_idx {
        let code = codepoints[idx];

        let Some(c) = char::from_u32(code) else {
            continue;
        };

        total_tested += 1;

        // 문자를 번역 - char_ranges를 일부러 사용하지 않음.
        let test_str = c.to_string();
        match engine.translate_mmntw(&test_str) {
            Ok(translated) => {
                let result = TranslationResult {
                    codepoint: format!("U+{:04X}", code),
                    character: c.to_string(),
                    translation: translated.clone(),
                    error: String::new(),
                };
                results.push(result.clone());
                pending_results.push(result);
            }
            Err(e) => {
                let result = TranslationResult {
                    codepoint: format!("U+{:04X}", code),
                    character: c.to_string(),
                    translation: String::new(),
                    error: format!("{:?}", e),
                };
                results.push(result.clone());
                pending_results.push(result);
            }
        }

        // 진행률 업데이트
        if last_progress.elapsed() >= Duration::from_millis(PROGRESS_INTERVAL_MS) {
            send_message(&WorkerMessage::Progress {
                worker_id,
                current_code: code,
                tested: total_tested,
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
        elapsed_secs: elapsed.as_secs_f64(),
    });
}

/// 워커 전용 테스트
#[test]
#[ignore]
fn safe_chars_worker() {
    if let Ok(worker_params) = env::var("SAFE_CHARS_WORKER") {
        let parts: Vec<&str> = worker_params.split('|').collect();
        if parts.len() == 6 {
            let worker_id: usize = parts[0].parse().unwrap();
            let start_idx: usize = parts[1].parse().unwrap();
            let end_idx: usize = parts[2].parse().unwrap();
            let dll_path = parts[3];
            let dat_path = parts[4];
            let safe_chars_path = parts[5];

            safe_chars_worker_process(
                worker_id,
                start_idx,
                end_idx,
                dll_path,
                dat_path,
                safe_chars_path,
            );
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
    completed: bool,
}

impl WorkerStatus {
    fn new() -> Self {
        Self {
            tested: 0,
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
fn print_progress_dashboard(statuses: &[WorkerStatus], total_chars: usize, start_time: Instant) {
    let total_tested: u32 = statuses.iter().map(|s| s.tested).sum();
    let completed_count = statuses.iter().filter(|s| s.completed).count();

    let elapsed = start_time.elapsed();
    let overall_progress = (total_tested as f64 / total_chars as f64) * 100.0;

    // ETA 계산
    let eta_str = if total_tested > 0 && elapsed.as_secs() > 0 {
        let rate = total_tested as f64 / elapsed.as_secs_f64();
        let remaining = (total_chars as u32).saturating_sub(total_tested);
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
        "\r[{}] {:.1}% | {} tested | ETA: {} | Workers: {}/{}   ",
        bar,
        overall_progress,
        total_tested,
        eta_str,
        completed_count,
        statuses.len()
    );
    std::io::stdout().flush().ok();
}

#[test]
#[ignore]
fn scan_safe_chars_8_procs() {
    if env::var("SAFE_CHARS_WORKER").is_ok() {
        return;
    }
    scan_safe_chars_multiprocess(Some(8));
}

fn scan_safe_chars_multiprocess(num_processes_opt: Option<usize>) {
    println!("\n=== SAFE CHARACTERS TRANSLATION TEST ===\n");

    let (dll_path, dat_path) = get_engine_paths();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let safe_chars_path = format!("{}/safe_chars_range.txt", manifest_dir);

    // safe_chars_range.txt 파싱
    let codepoints = match parse_safe_chars_file(&safe_chars_path) {
        Ok(cp) => cp,
        Err(e) => {
            eprintln!("Failed to parse safe_chars_range.txt: {:?}", e);
            panic!("Cannot proceed without safe_chars_range.txt");
        }
    };

    let num_processes = num_processes_opt.unwrap_or_else(|| num_cpus::get().min(8));
    let total_chars = codepoints.len();

    println!("Configuration:");
    println!("  Worker processes: {}", num_processes);
    println!("  CPU cores: {}", num_cpus::get());
    println!("  Total characters to test: {}", total_chars);
    println!();

    // 작업 분배
    let chunk_size = total_chars / num_processes;
    let mut work_assignments: Vec<(usize, usize, usize)> = Vec::new();

    for worker_id in 0..num_processes {
        let start_idx = worker_id * chunk_size;
        let end_idx = if worker_id == num_processes - 1 {
            total_chars
        } else {
            (worker_id + 1) * chunk_size
        };
        work_assignments.push((worker_id, start_idx, end_idx));
    }

    println!("Work distribution:");
    for (worker_id, start_idx, end_idx) in &work_assignments {
        println!(
            "  Worker {}: index {}..{} ({} chars)",
            worker_id,
            start_idx,
            end_idx,
            end_idx - start_idx
        );
    }
    println!();

    let overall_start_time = Instant::now();
    let current_exe = env::current_exe().expect("Failed to get current exe path");

    let (tx, rx) = mpsc::channel::<CoordinatorMessage>();

    let mut worker_statuses: Vec<WorkerStatus> =
        (0..num_processes).map(|_| WorkerStatus::new()).collect();
    let mut workers_completed = 0usize;
    let mut all_results: Vec<TranslationResult> = Vec::new();
    let mut total_tested = 0u32;

    let mut reader_threads = Vec::new();

    for (worker_id, start_idx, end_idx) in &work_assignments {
        let worker_params = format!(
            "{}|{}|{}|{}|{}|{}",
            worker_id, start_idx, end_idx, dll_path, dat_path, safe_chars_path
        );

        let mut cmd = Command::new(&current_exe);
        cmd.env("SAFE_CHARS_WORKER", worker_params)
            .arg("safe_chars_worker")
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
                    WorkerMessage::Progress { tested, .. } => {
                        if worker_id < worker_statuses.len() {
                            worker_statuses[worker_id].tested = tested;
                        }
                    }
                    WorkerMessage::ChunkResult { results, .. } => {
                        all_results.extend(results);
                    }
                    WorkerMessage::Complete {
                        worker_id: wid,
                        total_tested: tested,
                        ..
                    } => {
                        if wid < worker_statuses.len() {
                            worker_statuses[wid].tested = tested;
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
            print_progress_dashboard(&worker_statuses, total_chars, overall_start_time);
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
    println!("Total characters tested: {}", total_tested);
    println!("Total results: {}", all_results.len());

    // CSV 파일로 저장 (UTF-8 BOM 포함)
    let csv_path = "safe_chars_translations.csv";

    // UTF-8 BOM을 포함한 파일 생성
    let mut file = File::create(csv_path).expect("Failed to create CSV file");
    file.write_all("\u{FEFF}".as_bytes())
        .expect("Failed to write BOM");

    let mut wtr = csv::Writer::from_writer(file);

    // CSV 헤더
    wtr.write_record(&["Codepoint", "Character", "Translation", "Error"])
        .expect("Failed to write CSV header");

    // CSV 데이터
    for result in &all_results {
        wtr.write_record(&[
            &result.codepoint,
            &result.character,
            &result.translation,
            &result.error,
        ])
        .expect("Failed to write CSV record");
    }

    wtr.flush().expect("Failed to flush CSV writer");

    println!("\nResults saved to: {}", csv_path);

    // 통계 출력
    let error_count = all_results.iter().filter(|r| !r.error.is_empty()).count();
    let success_count = all_results.len() - error_count;

    println!("\n=== STATISTICS ===");
    println!("Successful translations: {}", success_count);
    println!("Translation errors: {}", error_count);
}
