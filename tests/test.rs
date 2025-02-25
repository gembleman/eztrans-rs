use eztrans_rs::{EzTransEngine, EzTransError};

#[test]
fn test_t() -> Result<(), EzTransError> {
    // EzTrans 엔진 초기화
    let ez_trans =
        EzTransEngine::new("C:/Program Files (x86)/ChangShinSoft/ezTrans XP/J2KEngine.dll")?;

    // 엔진 초기화
    ez_trans.initialize_ex(
        "CSUSER123455",
        "C:/Program Files (x86)/ChangShinSoft/ezTrans XP/Dat",
    )?;

    const TEXT: &str = "가나다라おはようございます。";

    for i in 0..50 {
        let transleted = ez_trans.default_translate(TEXT)?;
        println!("{}: {}", i, transleted);
    }

    ez_trans.terminate()?;

    Ok(())
}
