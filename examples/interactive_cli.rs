use eztrans_rs::EzTransEngine;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // DLL 경로 설정
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("eztrans_dll");
    let dll_path = base_path.join("J2KEngine.dll");
    let dat_path = base_path.join("Dat");

    println!("=== EzTrans 대화형 번역기 ===\n");
    println!("EzTrans 엔진 로드 중: {}", dll_path.display());

    // 엔진 초기화
    let engine = EzTransEngine::new(&dll_path)?;
    println!("엔진 로드 완료!");

    // 초기화
    if engine.initialize_ex.is_some() {
        println!("EHND 모드로 초기화 중...");
        println!("Dat 경로: {}", dat_path.display());
        engine.initialize_ex("CSUSER123455", dat_path.to_str().unwrap())?;
    } else {
        println!("기본 모드로 초기화 중...");
        engine.initialize()?;
    }
    println!("초기화 완료!\n");

    println!("사용법:");
    println!("  - 번역할 일본어 텍스트를 입력하세요");
    println!("  - 'quit' 또는 'exit'를 입력하면 종료합니다");
    println!("  - 빈 줄을 입력하면 건너뜁니다\n");

    // 대화형 루프
    loop {
        print!("번역할 텍스트 > ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();

        // 종료 명령어 확인
        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("\n프로그램을 종료합니다.");
            break;
        }

        // 빈 입력 건너뛰기
        if input.is_empty() {
            continue;
        }

        // 번역 실행
        match engine.default_translate(input) {
            Ok(translated) => {
                println!("원문: {}", input);
                println!("번역: {}\n", translated);
            }
            Err(e) => {
                eprintln!("번역 오류: {}\n", e);
            }
        }
    }

    Ok(())
}
