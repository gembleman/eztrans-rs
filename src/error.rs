use std::ffi::NulError;

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
    #[error("Failed to load function: {0}")]
    FunctionLoadError(String),
    #[error("Failed to call function: {0}")]
    FunctionCallFailed(String),
    #[error("Pipe error: {0}")]
    PipeError(String),
    #[error("Incomplete read")]
    IncompleteRead,
    #[error("Incomplete write")]
    IncompleteWrite,
    #[error("Invalid command: {0}")]
    InvalidCommand(u32),
    #[error("Windows error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

#[derive(Error, Debug, Clone)]
pub enum TransErr {
    #[error("TRANSLATE func returned a null pointer")]
    NullPointer,
    #[error("EUC-KR decoding failed")]
    EucKrDecodeFailed,
}
