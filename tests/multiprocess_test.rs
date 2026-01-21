// 멀티프로세싱 기본 테스트
// cargo test --target i686-pc-windows-msvc --test multiprocess_test test_basic_multiprocess -- --nocapture

use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::env;
use std::time::Instant;

// ============================================================================
// 방법 1: 별도의 exe를 사용하지 않고 CMD를 통해 간단한 명령 실행
// ============================================================================

#[test]
fn test_cmd_multiprocess() {
    println!("\n=== CMD Multiprocess Test ===\n");

    let num_workers = 4;
    println!("Spawning {} workers via cmd.exe...\n", num_workers);

    let start = Instant::now();
    let mut children = Vec::new();

    for i in 0..num_workers {
        // cmd /C echo 명령어로 간단한 출력 테스트
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", &format!("echo RESULT:worker={},value={}", i, i * 100)])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(child) => {
                println!("Spawned cmd worker {}", i);
                children.push((i, child));
            }
            Err(e) => {
                eprintln!("Failed to spawn worker {}: {}", i, e);
            }
        }
    }

    println!("\nWaiting for workers...\n");

    let mut results = Vec::new();
    for (id, mut child) in children {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("[Worker {}] {}", id, line);
                if line.starts_with("RESULT:") {
                    results.push((id, line));
                }
            }
        }

        let status = child.wait().expect("Failed to wait");
        println!("Worker {} exited with: {}", id, status);
    }

    let elapsed = start.elapsed();
    println!("\n=== Results ===");
    println!("Completed in {:?}", elapsed);
    println!("Results collected: {}/{}", results.len(), num_workers);

    assert_eq!(results.len(), num_workers, "Not all workers returned results");
    println!("\nTest PASSED!");
}

// ============================================================================
// 방법 2: PowerShell 스크립트로 더 복잡한 작업 실행
// ============================================================================

#[test]
fn test_powershell_multiprocess() {
    println!("\n=== PowerShell Multiprocess Test ===\n");

    let num_workers = 4;
    let start = Instant::now();
    let mut children = Vec::new();

    for i in 0..num_workers {
        let script = format!(
            r#"
            $sum = 0
            for ($j = 0; $j -lt 10000; $j++) {{
                $sum += $j * {}
            }}
            Write-Output "RESULT:worker={},sum=$sum"
            "#,
            i, i
        );

        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", &script])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(child) => {
                println!("Spawned PowerShell worker {}", i);
                children.push((i, child));
            }
            Err(e) => {
                eprintln!("Failed to spawn worker {}: {}", i, e);
            }
        }
    }

    println!("\nWaiting for workers...\n");

    let mut results = Vec::new();
    for (id, mut child) in children {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let line = line.trim().to_string();
                if !line.is_empty() {
                    println!("[Worker {}] {}", id, line);
                    if line.starts_with("RESULT:") {
                        results.push((id, line));
                    }
                }
            }
        }

        let status = child.wait().expect("Failed to wait");
        println!("Worker {} exited with: {}", id, status);
    }

    let elapsed = start.elapsed();
    println!("\n=== Results ===");
    println!("Completed in {:?}", elapsed);
    println!("Results collected: {}/{}", results.len(), num_workers);

    assert_eq!(results.len(), num_workers, "Not all workers returned results");
    println!("\nTest PASSED!");
}

// ============================================================================
// 방법 3: 테스트 바이너리를 워커로 사용하되, 특수 테스트 이름 필터 사용
// ============================================================================

fn run_worker_task(params: &str) {
    // 워커로 실행될 때
    let parts: Vec<&str> = params.split('|').collect();
    let worker_id: u32 = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
    let iterations: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);

    let mut sum = 0u64;
    for i in 0..iterations {
        sum = sum.wrapping_add(i * worker_id as u64);
    }

    // JSON 형식으로 출력
    println!("{{\"type\":\"result\",\"worker\":{},\"sum\":{}}}", worker_id, sum);
    std::io::stdout().flush().ok();
}

#[test]
fn worker_task_runner() {
    // 환경변수로 워커 모드 확인
    if let Ok(params) = env::var("WORKER_TASK_PARAMS") {
        run_worker_task(&params);
        std::process::exit(0);
    }
    // 일반 테스트로 실행된 경우 아무것도 하지 않음
}

#[test]
fn test_self_multiprocess() {
    // 워커 모드면 종료
    if env::var("WORKER_TASK_PARAMS").is_ok() {
        return;
    }

    println!("\n=== Self-Spawn Multiprocess Test ===\n");

    let current_exe = env::current_exe().expect("Failed to get exe path");
    println!("Executable: {:?}\n", current_exe);

    let num_workers = 4;
    let start = Instant::now();
    let mut children = Vec::new();

    for i in 0..num_workers {
        let mut cmd = Command::new(&current_exe);
        cmd.env("WORKER_TASK_PARAMS", format!("{}|100000", i))
            .arg("worker_task_runner")  // 특정 테스트만 실행
            .arg("--exact")
            .arg("--nocapture")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(child) => {
                println!("Spawned worker {}", i);
                children.push((i, child));
            }
            Err(e) => {
                eprintln!("Failed to spawn worker {}: {}", i, e);
            }
        }
    }

    println!("\nWaiting for workers...\n");

    let mut results = Vec::new();
    for (id, mut child) in children {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                // JSON 결과만 파싱
                if line.starts_with("{") && line.contains("\"type\":\"result\"") {
                    println!("[Worker {}] {}", id, line);
                    results.push((id, line));
                }
            }
        }

        let status = child.wait().expect("Failed to wait");
        if !status.success() {
            println!("Worker {} exited with: {:?}", id, status);
        }
    }

    let elapsed = start.elapsed();
    println!("\n=== Results ===");
    println!("Completed in {:?}", elapsed);
    println!("Results collected: {}/{}", results.len(), num_workers);

    for (id, result) in &results {
        println!("  Worker {}: {}", id, result);
    }

    assert_eq!(results.len(), num_workers, "Not all workers returned results");
    println!("\nTest PASSED!");
}

// ============================================================================
// 방법 4: 실시간 파이프 통신 테스트
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct WorkerMessage {
    msg_type: String,
    worker_id: u32,
    value: u64,
}

fn run_streaming_worker(params: &str) {
    let parts: Vec<&str> = params.split('|').collect();
    let worker_id: u32 = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);

    // 여러 진행 메시지 전송
    for i in 0..5 {
        let msg = WorkerMessage {
            msg_type: "progress".to_string(),
            worker_id,
            value: i * 20,
        };
        println!("{}", serde_json::to_string(&msg).unwrap());
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // 완료 메시지
    let msg = WorkerMessage {
        msg_type: "complete".to_string(),
        worker_id,
        value: 100,
    };
    println!("{}", serde_json::to_string(&msg).unwrap());
    std::io::stdout().flush().ok();
}

#[test]
fn streaming_worker_runner() {
    if let Ok(params) = env::var("STREAMING_WORKER_PARAMS") {
        run_streaming_worker(&params);
        std::process::exit(0);
    }
}

#[test]
fn test_streaming_multiprocess() {
    if env::var("STREAMING_WORKER_PARAMS").is_ok() {
        return;
    }

    println!("\n=== Streaming Multiprocess Test ===\n");

    let current_exe = env::current_exe().expect("Failed to get exe path");

    let num_workers = 2;
    let start = Instant::now();
    let mut children = Vec::new();

    for i in 0..num_workers {
        let mut cmd = Command::new(&current_exe);
        cmd.env("STREAMING_WORKER_PARAMS", format!("{}", i))
            .arg("streaming_worker_runner")
            .arg("--exact")
            .arg("--nocapture")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(child) => {
                println!("Spawned streaming worker {}", i);
                children.push((i, child));
            }
            Err(e) => {
                eprintln!("Failed to spawn worker {}: {}", i, e);
            }
        }
    }

    println!("\nReceiving messages...\n");

    let mut all_messages = Vec::new();
    for (id, mut child) in children {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(msg) = serde_json::from_str::<WorkerMessage>(&line) {
                    println!("[Worker {}] {:?}", id, msg);
                    all_messages.push(msg);
                }
            }
        }

        child.wait().ok();
    }

    let elapsed = start.elapsed();
    println!("\n=== Results ===");
    println!("Completed in {:?}", elapsed);
    println!("Total messages: {}", all_messages.len());

    // 각 워커당 6개 메시지 (5 progress + 1 complete)
    assert_eq!(all_messages.len(), num_workers * 6, "Expected {} messages", num_workers * 6);
    println!("\nTest PASSED!");
}
