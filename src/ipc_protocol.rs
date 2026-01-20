// IPC Protocol definitions (matching TransEngineIPC.h)

use std::mem::size_of;

pub const PIPE_NAME: &str = "\\\\.\\pipe\\AnemoneTransEngine";

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Initialize = 1,
    Terminate = 2,
    TranslateMMNT = 3,
    TranslateMMNTW = 4,
    ReloadUserDict = 5,
    SetProperty = 6,
    Shutdown = 7,
    Ping = 8,
}

impl TryFrom<u32> for Command {
    type Error = crate::EzTransError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Command::Initialize),
            2 => Ok(Command::Terminate),
            3 => Ok(Command::TranslateMMNT),
            4 => Ok(Command::TranslateMMNTW),
            5 => Ok(Command::ReloadUserDict),
            6 => Ok(Command::SetProperty),
            7 => Ok(Command::Shutdown),
            8 => Ok(Command::Ping),
            _ => Err(crate::EzTransError::InvalidCommand(value)),
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Status {
    Success = 0,
    Error = 1,
    NotInitialized = 2,
    InvalidParameter = 3,
}

#[repr(C, packed(8))]
#[derive(Debug, Clone, Copy)]
pub struct MessageHeader {
    pub command: u32,
    pub payload_size: u32,
    pub request_id: u64,
}

#[repr(C, packed(8))]
#[derive(Debug)]
pub struct InitializeRequest {
    pub engine_path: [u16; 260], // MAX_PATH
}

#[repr(C, packed(8))]
#[derive(Debug, Clone, Copy)]
pub struct InitializeResponse {
    pub status: Status,
    pub success: bool,
}

#[repr(C, packed(8))]
pub struct TranslateMMNTRequest {
    pub data0: u32,
    pub text: [u8; 4096],
}

#[repr(C, packed(8))]
pub struct TranslateMMNTResponse {
    pub status: Status,
    pub result_code: i32,
    pub translated: [u8; 4096],
}

#[repr(C, packed(8))]
pub struct TranslateMMNTWRequest {
    pub data0: u32,
    pub text: [u16; 4096],
}

#[repr(C, packed(8))]
pub struct TranslateMMNTWResponse {
    pub status: Status,
    pub result_code: i32,
    pub translated: [u16; 4096],
}

#[repr(C, packed(8))]
#[derive(Debug, Clone, Copy)]
pub struct GenericResponse {
    pub status: Status,
}

#[repr(C, packed(8))]
#[derive(Debug, Clone, Copy)]
pub struct SetPropertyRequest {
    pub property_id: i32,
    pub value: i32,
}

// Safety checks for struct sizes
const _: () = {
    assert!(size_of::<MessageHeader>() == 16);
    assert!(size_of::<InitializeResponse>() == 8);
    assert!(size_of::<GenericResponse>() == 4);
};
