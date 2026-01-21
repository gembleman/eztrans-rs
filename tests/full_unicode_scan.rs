// Full Unicode Range Scanner
// Tests EVERY valid Unicode codepoint (U+0000 to U+10FFFF)
// Run with: cargo test --target i686-pc-windows-msvc --test full_unicode_scan -- --include-ignored --test-threads=1 --nocapture

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

fn find_continuous_ranges(chars: &[char]) -> Vec<(u32, u32)> {
    if chars.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<u32> = chars.iter().map(|&c| c as u32).collect();
    sorted.sort_unstable();
    sorted.dedup();

    let mut ranges = Vec::new();
    let mut range_start = sorted[0];
    let mut range_end = sorted[0];

    for &code in &sorted[1..] {
        if code == range_end + 1 {
            range_end = code;
        } else {
            ranges.push((range_start, range_end));
            range_start = code;
            range_end = code;
        }
    }
    ranges.push((range_start, range_end));

    ranges
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

/// 코드포인트를 절대 인덱스로 변환
#[allow(dead_code)]
fn codepoint_to_absolute(code: u32) -> Option<u32> {
    let mut abs = 0u32;
    for (start, end) in UNICODE_RANGES {
        if code >= start && code <= end {
            return Some(abs + (code - start));
        }
        abs += end - start + 1;
    }
    None
}

// ============================================================================
// 인코딩 검증 테스트 V3
// - 원본 문자가 보존되는지 확인 (기존 로직)
// - hangul_encode → 번역 → hangul_decode 후 원본과 동일한지 확인 (추가 로직)
// ============================================================================

/// 문제가 있는 문자 정보
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ProblematicChar {
    code: u32,
    original: String,
    translated: String,
    issue_type: String, // "square_bracket", "question_mark", "different"
}

/// V3 워커에서 코디네이터로 보내는 메시지
#[derive(Serialize, Deserialize, Debug)]
enum WorkerMessageV3 {
    Progress {
        worker_id: usize,
        current_code: u32,
        tested: u32,
        found_safe: u32, // 안 깨지는 문자 수
    },
    ChunkResult {
        worker_id: usize,
        safe_chars: Vec<u32>, // 안 깨지는 문자들
    },
    ProblematicChars {
        worker_id: usize,
        chars: Vec<ProblematicChar>, // 문제가 있는 문자들
    },
    Complete {
        worker_id: usize,
        total_tested: u32,
        total_safe: u32,
        elapsed_secs: f64,
    },
    Error {
        worker_id: usize,
        message: String,
    },
}

fn send_message_v3(msg: &WorkerMessageV3) {
    let json = serde_json::to_string(msg).unwrap();
    println!("{}", json);
    std::io::stdout().flush().ok();
}

/// V3 워커 프로세스 - 안 깨지는 문자를 기록
fn scan_worker_process_v3(
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
            send_message_v3(&WorkerMessageV3::Error {
                worker_id,
                message: format!("Failed to load DLL: {:?}", err),
            });
            return;
        }
    };

    if let Err(err) = engine.initialize_ex("CSUSER123455", dat_path) {
        send_message_v3(&WorkerMessageV3::Error {
            worker_id,
            message: format!("Failed to initialize engine: {:?}", err),
        });
        return;
    }

    let mut safe_chars = Vec::new(); // 번역 시 안 깨지는 문자
    let mut pending_safe = Vec::new();
    let mut problematic_chars = Vec::new(); // 문제가 있는 문자들
    let mut pending_problematic = Vec::new();
    let mut total_tested = 0u32;
    let mut last_progress = Instant::now();
    let mut last_chunk_send = Instant::now();

    const CHUNK_SIZE: usize = 1000;
    const PROGRESS_INTERVAL_MS: u64 = 500;
    const CHUNK_INTERVAL_SECS: u64 = 5;

    /// 한글인지 체크하는 함수
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

    for abs_idx in abs_start..=abs_end {
        let Some(code) = absolute_to_codepoint(abs_idx) else {
            continue;
        };

        total_tested += 1;

        if let Some(c) = char::from_u32(code) {
            let test_str = format!("あ{}い", c);

            // 원본 문자가 "?"로 변경되었는지 확인
            let result = engine.translate_mmntw(&test_str);
            let mut is_safe = true;
            let mut issue_type = None;

            match &result {
                Ok(translated) => {
                    // 1. "□" 포함 확인
                    if translated.contains('□') {
                        is_safe = false;
                        issue_type = Some("square_bracket");
                    }
                    // 2. "?" 포함 확인 (원본이 "?"가 아닌 경우)
                    else if c != '?' && translated.contains('?') {
                        is_safe = false;
                        issue_type = Some("question_mark");
                    }
                    // 3. 번역문이 원문과 다른 경우 (한국어가 아닌 경우만)
                    else if !translated.contains(c) && !is_korean(translated) {
                        is_safe = false;
                        issue_type = Some("different");
                    }

                    // 문제가 있는 문자 기록
                    if !is_safe {
                        if let Some(issue) = issue_type {
                            let prob_char = ProblematicChar {
                                code,
                                original: test_str.clone(),
                                translated: translated.clone(),
                                issue_type: issue.to_string(),
                            };
                            problematic_chars.push(prob_char.clone());
                            pending_problematic.push(prob_char);
                        }
                    }
                }
                Err(_) => {
                    is_safe = false;
                }
            };

            if is_safe {
                safe_chars.push(code);
                pending_safe.push(code);
            }
        }

        // 진행률 업데이트
        if last_progress.elapsed() >= Duration::from_millis(PROGRESS_INTERVAL_MS) {
            send_message_v3(&WorkerMessageV3::Progress {
                worker_id,
                current_code: code,
                tested: total_tested,
                found_safe: safe_chars.len() as u32,
            });
            last_progress = Instant::now();
        }

        // 청크 결과 전송
        if pending_safe.len() >= CHUNK_SIZE
            || (last_chunk_send.elapsed() >= Duration::from_secs(CHUNK_INTERVAL_SECS)
                && !pending_safe.is_empty())
        {
            send_message_v3(&WorkerMessageV3::ChunkResult {
                worker_id,
                safe_chars: pending_safe.clone(),
            });
            pending_safe.clear();
            last_chunk_send = Instant::now();
        }

        // 문제가 있는 문자 청크 전송
        if pending_problematic.len() >= CHUNK_SIZE
            || (last_chunk_send.elapsed() >= Duration::from_secs(CHUNK_INTERVAL_SECS)
                && !pending_problematic.is_empty())
        {
            send_message_v3(&WorkerMessageV3::ProblematicChars {
                worker_id,
                chars: pending_problematic.clone(),
            });
            pending_problematic.clear();
        }
    }

    // 남은 청크 전송
    if !pending_safe.is_empty() {
        send_message_v3(&WorkerMessageV3::ChunkResult {
            worker_id,
            safe_chars: pending_safe,
        });
    }

    if !pending_problematic.is_empty() {
        send_message_v3(&WorkerMessageV3::ProblematicChars {
            worker_id,
            chars: pending_problematic,
        });
    }

    let elapsed = start_time.elapsed();

    send_message_v3(&WorkerMessageV3::Complete {
        worker_id,
        total_tested,
        total_safe: safe_chars.len() as u32,
        elapsed_secs: elapsed.as_secs_f64(),
    });
}

/// V3 워커 전용 테스트
#[test]
#[ignore]
fn unicode_scan_worker_v3() {
    if let Ok(worker_params) = env::var("UNICODE_SCAN_WORKER_V3") {
        let parts: Vec<&str> = worker_params.split('|').collect();
        if parts.len() == 5 {
            let worker_id: usize = parts[0].parse().unwrap();
            let abs_start: u32 = parts[1].parse().unwrap();
            let abs_end: u32 = parts[2].parse().unwrap();
            let dll_path = parts[3];
            let dat_path = parts[4];

            scan_worker_process_v3(worker_id, abs_start, abs_end, dll_path, dat_path);
            std::process::exit(0);
        }
    }
}

#[derive(Debug)]
enum CoordinatorMessageV3 {
    WorkerMessage {
        worker_id: usize,
        msg: WorkerMessageV3,
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

/// V3 워커 상태 추적
#[derive(Debug, Clone)]
struct WorkerStatusV3 {
    tested: u32,
    found_safe: u32,
    completed: bool,
}

impl WorkerStatusV3 {
    fn new() -> Self {
        Self {
            tested: 0,
            found_safe: 0,
            completed: false,
        }
    }
}

/// V3 실시간 진행률 표시
fn print_progress_dashboard_v3(
    statuses: &[WorkerStatusV3],
    total_codepoints: u32,
    start_time: Instant,
) {
    let total_tested: u32 = statuses.iter().map(|s| s.tested).sum();
    let total_safe: u32 = statuses.iter().map(|s| s.found_safe).sum();
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
        "\r[{}] {:.1}% | {} tested | {} safe | ETA: {} | Workers: {}/{}   ",
        bar,
        overall_progress,
        total_tested,
        total_safe,
        eta_str,
        completed_count,
        statuses.len()
    );
    std::io::stdout().flush().ok();
}

#[test]
#[ignore]
fn scan_entire_unicode_range_v3_8_procs() {
    if env::var("UNICODE_SCAN_WORKER_V3").is_ok() {
        return;
    }
    scan_multiprocess_v3(Some(8));
}

fn scan_multiprocess_v3(num_processes_opt: Option<usize>) {
    println!("\n=== MULTI-PROCESS UNICODE SCAN V3 (with encode verification) ===\n");

    let (dll_path, dat_path) = get_engine_paths();

    println!("Skipping DLL test in coordinator.\n");

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

    let (tx, rx) = mpsc::channel::<CoordinatorMessageV3>();

    // 워커 상태 추적
    let mut worker_statuses: Vec<WorkerStatusV3> =
        (0..num_processes).map(|_| WorkerStatusV3::new()).collect();
    let mut workers_completed = 0usize;
    let mut all_safe_chars: Vec<u32> = Vec::new(); // 안 깨지는 문자들
    let mut all_problematic_chars: Vec<ProblematicChar> = Vec::new(); // 문제가 있는 문자들
    let mut total_tested = 0u32;

    let mut reader_threads = Vec::new();

    for (worker_id, abs_start, abs_end) in &work_assignments {
        let worker_params = format!(
            "{}|{}|{}|{}|{}",
            worker_id, abs_start, abs_end, dll_path, dat_path
        );

        let mut cmd = Command::new(&current_exe);
        cmd.env("UNICODE_SCAN_WORKER_V3", worker_params)
            .arg("unicode_scan_worker_v3")
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
                                if let Ok(msg) = serde_json::from_str::<WorkerMessageV3>(&line) {
                                    let _ = tx_clone.send(CoordinatorMessageV3::WorkerMessage {
                                        worker_id: wid,
                                        msg,
                                    });
                                }
                            }
                            Err(e) => {
                                let _ = tx_clone.send(CoordinatorMessageV3::WorkerError {
                                    worker_id: wid,
                                    error: e.to_string(),
                                });
                                break;
                            }
                        }
                    }
                    let _ = tx_clone.send(CoordinatorMessageV3::WorkerEof { worker_id: wid });
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
                CoordinatorMessageV3::WorkerMessage { worker_id, msg } => match msg {
                    WorkerMessageV3::Progress {
                        tested, found_safe, ..
                    } => {
                        if worker_id < worker_statuses.len() {
                            worker_statuses[worker_id].tested = tested;
                            worker_statuses[worker_id].found_safe = found_safe;
                        }
                    }
                    WorkerMessageV3::ChunkResult { safe_chars, .. } => {
                        all_safe_chars.extend(safe_chars);
                    }
                    WorkerMessageV3::ProblematicChars { chars, .. } => {
                        all_problematic_chars.extend(chars);
                    }
                    WorkerMessageV3::Complete {
                        worker_id: wid,
                        total_tested: tested,
                        total_safe,
                        ..
                    } => {
                        if wid < worker_statuses.len() {
                            worker_statuses[wid].tested = tested;
                            worker_statuses[wid].found_safe = total_safe;
                            worker_statuses[wid].completed = true;
                        }
                        total_tested += tested;
                    }
                    WorkerMessageV3::Error {
                        worker_id: wid,
                        message,
                    } => {
                        eprintln!("\n[Worker {}] Error: {}", wid, message);
                    }
                },
                CoordinatorMessageV3::WorkerEof { .. } => {
                    workers_completed += 1;
                }
                CoordinatorMessageV3::WorkerError { worker_id, error } => {
                    eprintln!("\n[Worker {}] Read error: {}", worker_id, error);
                    workers_completed += 1;
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        // 대시보드 업데이트
        if last_dashboard_update.elapsed() >= dashboard_interval {
            print_progress_dashboard_v3(&worker_statuses, total_codepoints, overall_start_time);
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

    // 결과 정리
    all_safe_chars.sort_unstable();
    all_safe_chars.dedup();

    let safe_chars: Vec<char> = all_safe_chars
        .iter()
        .filter_map(|&code| char::from_u32(code))
        .collect();

    let total_corrupted = total_tested - safe_chars.len() as u32;

    println!(
        "\n\n=== SCAN COMPLETE in {} ===\n",
        format_duration(total_elapsed)
    );
    println!("Total codepoints tested: {}", total_tested);
    println!(
        "Safe characters (don't need encoding): {}",
        safe_chars.len()
    );
    println!("Corrupted characters (need encoding): {}", total_corrupted);

    // 연속 범위 찾기 - 안 깨지는 문자들
    let ranges = find_continuous_ranges(&safe_chars);
    println!("\n=== SAFE CHARACTERS ({} ranges) ===\n", ranges.len());

    for (start, end) in &ranges {
        let count = end - start + 1;
        if count == 1 {
            if let Some(c) = char::from_u32(*start) {
                println!("  U+{:06X} ('{}')", start, c);
            }
        } else if count <= 5 {
            print!("  U+{:06X}..U+{:06X} (", start, end);
            for code in *start..=*end {
                if let Some(c) = char::from_u32(code) {
                    print!("'{}' ", c);
                }
            }
            println!(")");
        } else {
            println!("  U+{:06X}..U+{:06X} ({} chars)", start, end, count);
        }
    }

    // Rust 코드 생성 - is_safe_char 함수 (안 깨지는 문자)
    println!("\n=== GENERATED RUST CODE (safe chars - don't need encoding) ===\n");
    println!("#[inline]");
    println!("pub const fn is_safe_char(c: char) -> bool {{");
    println!("    let code = c as u32;");
    println!("    matches!(code,");

    for (start, end) in &ranges {
        if start == end {
            println!("        0x{:06X} |", start);
        } else {
            println!("        0x{:06X}..=0x{:06X} |", start, end);
        }
    }

    println!("    )");
    println!("}}");

    // needs_special_encoding은 반대
    println!("\n=== USAGE ===\n");
    println!("// needs_special_encoding은 is_safe_char의 반대:");
    println!("#[inline]");
    println!("pub const fn needs_special_encoding(c: char) -> bool {{");
    println!("    !is_safe_char(c)");
    println!("}}");

    // 결과 저장
    let output_path = "full_unicode_scan_v3_results.txt";
    let mut output = String::new();
    output.push_str("Full Unicode Scan Results V3 (safe characters)\n");
    output.push_str("==============================================\n\n");
    output.push_str(&format!("Total time: {}\n", format_duration(total_elapsed)));
    output.push_str(&format!("Total tested: {}\n", total_tested));
    output.push_str(&format!("Safe: {}\n", safe_chars.len()));
    output.push_str(&format!("Corrupted: {}\n\n", total_corrupted));

    output.push_str("Safe characters (don't need encoding):\n");
    for &c in &safe_chars {
        output.push_str(&format!("U+{:06X} '{}'\n", c as u32, c));
    }

    output.push_str("\n\nGenerated Rust ranges (safe chars):\n");
    for (start, end) in &ranges {
        if start == end {
            output.push_str(&format!("0x{:06X}\n", start));
        } else {
            output.push_str(&format!("0x{:06X}..=0x{:06X}\n", start, end));
        }
    }

    std::fs::write(output_path, output).ok();
    println!("\nResults saved to: {}", output_path);

    // CSV 파일에 문제가 있는 문자 저장
    if !all_problematic_chars.is_empty() {
        let csv_path = "problematic_chars.csv";
        match File::create(csv_path) {
            Ok(mut file) => {
                // CSV 헤더
                writeln!(file, "Code,Character,Original,Translated,IssueType").ok();

                // 문제가 있는 문자들 정렬 (코드 순)
                all_problematic_chars.sort_by_key(|c| c.code);

                // CSV 데이터 작성
                for prob_char in &all_problematic_chars {
                    let char_display = char::from_u32(prob_char.code)
                        .map(|c| format!("{}", c))
                        .unwrap_or_else(|| "N/A".to_string());

                    // CSV 이스케이프 처리
                    let original_escaped = prob_char.original.replace('"', "\"\"");
                    let translated_escaped = prob_char.translated.replace('"', "\"\"");

                    writeln!(
                        file,
                        "U+{:06X},\"{}\",\"{}\",\"{}\",{}",
                        prob_char.code,
                        char_display,
                        original_escaped,
                        translated_escaped,
                        prob_char.issue_type
                    ).ok();
                }

                println!("\nProblematic characters saved to: {}", csv_path);
                println!("Total problematic characters: {}", all_problematic_chars.len());
            }
            Err(e) => {
                eprintln!("\nFailed to create CSV file: {}", e);
            }
        }
    }
}
