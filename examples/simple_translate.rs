use eztrans_rs::EzTransEngine;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR은 컴파일 타임에 eztrans-rs 디렉토리를 가리킴
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("eztrans_dll");
    let dll_path = base_path.join("J2KEngine.dll");
    let dat_path = base_path.join("Dat");

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
    println!("초기화 완료!");

    // 번역할 텍스트
    let text = "蜜ドル辞典";

    println!("\n원문: {}", text);
    println!("번역 중...");

    // 번역 실행
    let translated = engine.default_translate(text)?;

    println!("번역 결과: {}", translated);

    Ok(())
}
