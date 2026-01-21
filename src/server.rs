// Named Pipe Server implementation

use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{FILE_FLAGS_AND_ATTRIBUTES, ReadFile, WriteFile};
use windows::Win32::System::Pipes::*;
use windows::core::PCWSTR;

use crate::ipc_protocol::*;
use crate::{EzTransEngine, EzTransError};

// Constants from Windows SDK
const PIPE_ACCESS_DUPLEX: FILE_FLAGS_AND_ATTRIBUTES = FILE_FLAGS_AND_ATTRIBUTES(0x00000003);

pub struct TransProxyServer {
    pipe_handle: HANDLE,
    engine: Option<EzTransEngine>,
    initialized: bool,
    running: bool,
}

impl TransProxyServer {
    pub fn new() -> Self {
        Self {
            pipe_handle: INVALID_HANDLE_VALUE,
            engine: None,
            initialized: false,
            running: true,
        }
    }

    pub fn start(&mut self) -> Result<(), EzTransError> {
        unsafe {
            let pipe_name: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();

            self.pipe_handle = CreateNamedPipeW(
                PCWSTR(pipe_name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                1,    // Max instances
                8192, // Out buffer size
                8192, // In buffer size
                0,    // Timeout
                None, // Security attributes
            );

            if self.pipe_handle == INVALID_HANDLE_VALUE {
                return Err(EzTransError::PipeError(
                    "Failed to create named pipe".to_string(),
                ));
            }

            // Wait for client connection
            ConnectNamedPipe(self.pipe_handle, None)?;

            Ok(())
        }
    }

    pub fn run(&mut self) {
        while self.running {
            if let Err(e) = self.process_request() {
                eprintln!("Error processing request: {}", e);
                break;
            }
        }
    }

    fn read_message<T>(&self, buffer: &mut T) -> Result<(), EzTransError> {
        unsafe {
            let mut bytes_read = 0u32;
            ReadFile(
                self.pipe_handle,
                Some(std::slice::from_raw_parts_mut(
                    buffer as *mut T as *mut u8,
                    size_of::<T>(),
                )),
                Some(&mut bytes_read),
                None,
            )?;

            if bytes_read as usize != size_of::<T>() {
                return Err(EzTransError::IncompleteRead);
            }

            Ok(())
        }
    }

    fn write_message<T>(&self, buffer: &T) -> Result<(), EzTransError> {
        unsafe {
            let mut bytes_written = 0u32;
            WriteFile(
                self.pipe_handle,
                Some(std::slice::from_raw_parts(
                    buffer as *const T as *const u8,
                    size_of::<T>(),
                )),
                Some(&mut bytes_written),
                None,
            )?;

            if bytes_written as usize != size_of::<T>() {
                return Err(EzTransError::IncompleteWrite);
            }

            Ok(())
        }
    }

    fn process_request(&mut self) -> Result<(), EzTransError> {
        let mut header = MessageHeader {
            command: 0,
            payload_size: 0,
            request_id: 0,
        };

        self.read_message(&mut header)?;

        let command = Command::try_from(header.command)?;

        match command {
            Command::Initialize => self.handle_initialize(),
            Command::Terminate => self.handle_terminate(),
            Command::TranslateMMNT => self.handle_translate_mmnt(),
            Command::TranslateMMNTW => self.handle_translate_mmntw(),
            Command::ReloadUserDict => self.handle_reload_user_dict(),
            Command::SetProperty => self.handle_set_property(),
            Command::Shutdown => {
                self.running = false;
                Ok(())
            }
            Command::Ping => {
                let response = GenericResponse {
                    status: Status::Success,
                };
                self.write_message(&response)
            }
        }
    }

    fn handle_initialize(&mut self) -> Result<(), EzTransError> {
        let mut request = InitializeRequest {
            engine_path: [0; 260],
        };
        self.read_message(&mut request)?;

        // UTF-16 경로 파싱
        let path_str = String::from_utf16_lossy(&request.engine_path);
        let path_str = path_str.trim_end_matches('\0');

        let dll_path = format!("{}\\J2KEngine.dll", path_str);
        let dat_path = format!("{}\\Dat", path_str);

        // EzTransEngine 사용 (중복 코드 제거)
        match EzTransEngine::new(&dll_path) {
            Ok(engine) => match engine.initialize_ex("CSUSER123455", &dat_path) {
                Ok(_) => {
                    self.engine = Some(engine);
                    self.initialized = true;
                    let response = InitializeResponse {
                        status: Status::Success,
                        success: true,
                    };
                    self.write_message(&response)
                }
                Err(_) => {
                    let response = InitializeResponse {
                        status: Status::Error,
                        success: false,
                    };
                    self.write_message(&response)
                }
            },
            Err(_) => {
                let response = InitializeResponse {
                    status: Status::Error,
                    success: false,
                };
                self.write_message(&response)
            }
        }
    }

    fn handle_terminate(&mut self) -> Result<(), EzTransError> {
        if let Some(ref engine) = self.engine {
            let _ = engine.terminate();
        }
        self.engine = None;
        self.initialized = false;

        let response = GenericResponse {
            status: Status::Success,
        };
        self.write_message(&response)
    }

    fn handle_translate_mmnt(&mut self) -> Result<(), EzTransError> {
        let mut request = TranslateMMNTRequest {
            data0: 0,
            text: [0; 4096],
        };
        self.read_message(&mut request)?;

        let mut response = TranslateMMNTResponse {
            status: Status::Success,
            result_code: -1,
            translated: [0; 4096],
        };

        if let Some(ref engine) = self.engine {
            // 입력 텍스트 추출 (null 종료까지)
            let text_len = request.text.iter().position(|&x| x == 0).unwrap_or(4096);

            // Shift-JIS → UTF-8 디코딩
            let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&request.text[..text_len]);

            // 번역 (한글 인코딩 포함)
            match engine.translate_mmnt(&decoded) {
                Ok(translated) => {
                    // UTF-8 → EUC-KR 인코딩
                    let (encoded, _, _) = encoding_rs::EUC_KR.encode(&translated);
                    let len = encoded.len().min(4096);
                    response.translated[..len].copy_from_slice(&encoded[..len]);
                    response.result_code = 0;
                    response.status = Status::Success;
                }
                Err(_) => {
                    response.status = Status::Error;
                }
            }
        } else {
            response.status = Status::NotInitialized;
        }

        self.write_message(&response)
    }

    fn handle_translate_mmntw(&mut self) -> Result<(), EzTransError> {
        let mut request = TranslateMMNTWRequest {
            data0: 0,
            text: [0; 4096],
        };
        self.read_message(&mut request)?;

        let mut response = TranslateMMNTWResponse {
            status: Status::Success,
            result_code: -1,
            translated: [0; 4096],
        };

        if let Some(ref engine) = self.engine {
            // UTF-16 → String 변환
            let text_len = request.text.iter().position(|&x| x == 0).unwrap_or(4096);
            let input = String::from_utf16_lossy(&request.text[..text_len]);

            // 번역 (한글 인코딩/디코딩 자동 포함)
            match engine.default_translate(&input) {
                Ok(translated) => {
                    // String → UTF-16 변환
                    let utf16: Vec<u16> = translated.encode_utf16().collect();
                    let len = utf16.len().min(4095);
                    response.translated[..len].copy_from_slice(&utf16[..len]);
                    response.translated[len] = 0; // null 종료
                    response.result_code = 0;
                    response.status = Status::Success;
                }
                Err(_) => {
                    response.status = Status::Error;
                }
            }
        } else {
            response.status = Status::NotInitialized;
        }

        self.write_message(&response)
    }

    fn handle_reload_user_dict(&mut self) -> Result<(), EzTransError> {
        if let Some(ref engine) = self.engine {
            let _ = engine.reload_user_dict();
        }

        let response = GenericResponse {
            status: Status::Success,
        };
        self.write_message(&response)
    }

    fn handle_set_property(&mut self) -> Result<(), EzTransError> {
        let mut request = SetPropertyRequest {
            property_id: 0,
            value: 0,
        };
        self.read_message(&mut request)?;

        let response = if let Some(ref engine) = self.engine {
            match engine.set_property(request.property_id, request.value) {
                Ok(_) => GenericResponse {
                    status: Status::Success,
                },
                Err(_) => GenericResponse {
                    status: Status::Error,
                },
            }
        } else {
            GenericResponse {
                status: Status::NotInitialized,
            }
        };

        self.write_message(&response)
    }
}

impl Drop for TransProxyServer {
    fn drop(&mut self) {
        if self.pipe_handle != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = DisconnectNamedPipe(self.pipe_handle);
                let _ = CloseHandle(self.pipe_handle);
            }
        }
    }
}
