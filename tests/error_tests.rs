// Error Type Tests

use eztrans_rs::{EzTransError, TransErr};

#[test]
fn test_trans_err_display_null_pointer() {
    let err = TransErr::NullPointer;
    assert_eq!(format!("{}", err), "TRANSLATE func returned a null pointer");
}

#[test]
fn test_trans_err_display_euc_kr_decode_failed() {
    let err = TransErr::EucKrDecodeFailed;
    assert_eq!(format!("{}", err), "EUC-KR decoding failed");
}

#[test]
fn test_trans_err_debug() {
    let err = TransErr::NullPointer;
    assert_eq!(format!("{:?}", err), "NullPointer");

    let err = TransErr::EucKrDecodeFailed;
    assert_eq!(format!("{:?}", err), "EucKrDecodeFailed");
}

#[test]
fn test_trans_err_clone() {
    let err1 = TransErr::NullPointer;
    let err2 = err1.clone();
    assert_eq!(format!("{}", err1), format!("{}", err2));
}

#[test]
fn test_eztrans_error_translation_error() {
    let err = EzTransError::TranslationError(TransErr::NullPointer);
    assert!(format!("{}", err).contains("Failed to translate"));
}

#[test]
fn test_eztrans_error_invalid_path() {
    let err = EzTransError::InvalidPath;
    assert_eq!(format!("{}", err), "Invalid dll path");
}

#[test]
fn test_eztrans_error_dll_load_error() {
    let err = EzTransError::DllLoadError("test error".to_string());
    assert!(format!("{}", err).contains("Failed to load dll"));
    assert!(format!("{}", err).contains("test error"));
}

#[test]
fn test_eztrans_error_function_load_error() {
    let err = EzTransError::FunctionLoadError("J2K_Initialize not found".to_string());
    assert!(format!("{}", err).contains("Failed to load funtion"));
}

#[test]
fn test_eztrans_error_function_call_failed() {
    let err = EzTransError::FunctionCallFailed("return code -1".to_string());
    assert!(format!("{}", err).contains("Failed to call function"));
}

#[test]
fn test_eztrans_error_from_trans_err() {
    let trans_err = TransErr::NullPointer;
    let ez_err: EzTransError = trans_err.into();

    match ez_err {
        EzTransError::TranslationError(inner) => {
            assert!(matches!(inner, TransErr::NullPointer));
        }
        _ => panic!("Expected TranslationError"),
    }
}

#[test]
fn test_eztrans_error_from_utf16_error() {
    // Create an invalid UTF-16 sequence
    let invalid_utf16 = vec![0xD800u16]; // Unpaired surrogate
    let result = String::from_utf16(&invalid_utf16);

    if let Err(utf16_err) = result {
        let ez_err: EzTransError = utf16_err.into();
        assert!(format!("{}", ez_err).contains("utf16 error"));
    }
}

#[test]
fn test_eztrans_error_debug() {
    let err = EzTransError::InvalidPath;
    assert!(format!("{:?}", err).contains("InvalidPath"));
}
