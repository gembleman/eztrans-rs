use eztrans_rs::EzTransEngine;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct InputRecord {
    code: String,
    character: String,
    translation: String,
    issue_type: Option<String>,
    category: Option<String>,
    char_name: Option<String>,
    trans_name: Option<String>,
    accept: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutputRecord {
    code: String,
    character: String,
    original_translation: String,
    eztrans_translation: String,
    issue_type: String,
    category: String,
    char_name: String,
    trans_name: String,
    matches: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    // 경로 설정
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dll_path = base_path.join("eztrans_dll").join("J2KEngine.dll");
    let dat_path = base_path.join("eztrans_dll").join("Dat");
    let input_csv = base_path.join("non_fullwidth_conversions.csv");
    let output_csv = base_path.join("eztrans_translation_results.csv");

    println!("=== EzTrans CSV 번역기 ===\n");

    // CSV 파일 확인
    if !input_csv.exists() {
        eprintln!("오류: {} 파일을 찾을 수 없습니다.", input_csv.display());
        return Ok(());
    }

    println!("입력 CSV: {}", input_csv.display());
    println!("출력 CSV: {}", output_csv.display());

    // EzTrans 엔진 초기화
    println!("\nEzTrans 엔진 로드 중: {}", dll_path.display());
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

    // CSV 읽기
    println!("CSV 파일 읽는 중...");
    let file = File::open(&input_csv)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);

    let mut output_records = Vec::new();
    let mut total = 0;
    let mut translated = 0;
    let mut skipped = 0;
    let mut matches = 0;

    for result in reader.deserialize() {
        let record: InputRecord = result?;
        total += 1;

        // accept가 TRUE인 경우 건너뛰기
        if let Some(ref accept) = record.accept {
            if accept.trim().eq_ignore_ascii_case("TRUE") {
                skipped += 1;
                if total % 50 == 0 {
                    println!("진행 중... {}/{} 처리됨 (번역: {}, 건너뜀: {})",
                        total, total, translated, skipped);
                }
                continue;
            }
        }

        // 빈 문자열이거나 번역할 수 없는 경우 건너뛰기
        if record.character.is_empty() {
            skipped += 1;
            if total % 50 == 0 {
                println!("진행 중... {}/{} 처리됨 (번역: {}, 건너뜀: {})",
                    total, total, translated, skipped);
            }
            continue;
        }

        // 번역 실행
        match engine.default_translate(&record.character) {
            Ok(eztrans_result) => {
                translated += 1;

                // 번역 결과가 원본 translation과 일치하는지 확인
                let is_match = eztrans_result.trim() == record.translation.trim();
                if is_match {
                    matches += 1;
                }

                output_records.push(OutputRecord {
                    code: record.code.clone(),
                    character: record.character.clone(),
                    original_translation: record.translation.clone(),
                    eztrans_translation: eztrans_result,
                    issue_type: record.issue_type.unwrap_or_else(|| "N/A".to_string()),
                    category: record.category.unwrap_or_else(|| "N/A".to_string()),
                    char_name: record.char_name.unwrap_or_else(|| "N/A".to_string()),
                    trans_name: record.trans_name.unwrap_or_else(|| "N/A".to_string()),
                    matches: is_match,
                });

                // 진행 상황 출력
                if total % 50 == 0 {
                    println!("진행 중... {}/{} 처리됨 (번역: {}, 건너뜀: {}, 일치: {})",
                        total, total, translated, skipped, matches);
                }
            }
            Err(e) => {
                eprintln!("번역 오류 ({}): {} - {}", record.code, record.character, e);
                skipped += 1;
            }
        }
    }

    println!("\n번역 완료!");
    println!("총 레코드: {}", total);
    println!("번역 성공: {}", translated);
    println!("건너뜀: {}", skipped);
    println!("일치: {} ({:.1}%)", matches, (matches as f64 / translated as f64) * 100.0);

    // 결과를 CSV로 저장
    println!("\n결과 저장 중: {}", output_csv.display());
    let output_file = File::create(&output_csv)?;
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(output_file);

    for record in output_records {
        writer.serialize(record)?;
    }

    writer.flush()?;
    println!("저장 완료!");

    // 불일치 항목 요약
    println!("\n=== 불일치 항목 샘플 (처음 10개) ===");
    let output_file = File::open(&output_csv)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(output_file);

    let mut count = 0;
    for result in reader.deserialize::<OutputRecord>() {
        if let Ok(record) = result {
            if !record.matches && count < 10 {
                println!("\n코드: {}", record.code);
                println!("원본: {}", record.character);
                println!("기대값: {}", record.original_translation);
                println!("실제값: {}", record.eztrans_translation);
                count += 1;
            }
        }
    }

    Ok(())
}
