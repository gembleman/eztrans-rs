use std::{ffi::NulError, fmt};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EzTransError {
    #[error("Failed to translate: {0}")]
    TranslationError(#[from] TransErr),
    #[error("Invalid string: {0}")]
    InvalidString(#[from] NulError),
    #[error("utf16 error {0}")]
    Utf16Error(#[from] std::string::FromUtf16Error),
    #[error("Invalid dll path")]
    InvalidPath,
    #[error("Failed to load dll: {0}")]
    DllLoadError(String),
    #[error("Failed to load funtion: {0}")]
    FunctionLoadError(String),
    #[error("Failed to call function: {0}")]
    FunctionCallFailed(String),
}

#[derive(Error, Debug, Clone)]
pub enum TransErr {
    ///TRANSLATE_MMNTW or MMNT returned a null pointer
    NullPointer,
    ///Translation failed
    Failed,
    ///EUC-KR decoding failed
    EucKrDecodeFailed,
}
impl fmt::Display for TransErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransErr::NullPointer => write!(f, "TRANSLATE func returned a null pointer"),
            TransErr::Failed => write!(f, "Translation failed"),
            TransErr::EucKrDecodeFailed => write!(f, "EUC-KR decoding failed"),
        }
    }
}
