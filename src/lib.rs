#![allow(non_camel_case_types)]
mod error;
pub use error::{EzTransError, TransErr};

use std::collections::HashSet;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fmt::Write;
use std::path::Path;
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::core::{Error as WindowsError, PCSTR};

// Type definitions for all EzTrans engine functions
pub type J2K_FreeMem = unsafe extern "stdcall" fn(*mut c_void);
pub type J2K_GetPriorDict = unsafe extern "stdcall" fn() -> c_int;
pub type J2K_GetProperty = unsafe extern "stdcall" fn(c_int) -> c_int;
pub type J2K_Initialize = unsafe extern "stdcall" fn() -> c_int;
pub type J2K_InitializeEx = unsafe extern "stdcall" fn(*const c_char, *const c_char) -> c_int;
pub type J2K_ReloadUserDict = unsafe extern "stdcall" fn() -> c_int;
pub type J2K_SetDelJPN = unsafe extern "stdcall" fn(c_int) -> c_int;
pub type J2K_SetField = unsafe extern "stdcall" fn(c_int) -> c_int;
pub type J2K_SetHnj2han = unsafe extern "stdcall" fn(c_int) -> c_int;
pub type J2K_SetJWin = unsafe extern "stdcall" fn(c_int) -> c_int;
pub type J2K_SetPriorDict = unsafe extern "stdcall" fn(*const c_char) -> c_int;
pub type J2K_SetProperty = unsafe extern "stdcall" fn(c_int, c_int) -> c_int;
pub type J2K_StopTranslation = unsafe extern "stdcall" fn() -> c_int;
pub type J2K_Terminate = unsafe extern "stdcall" fn() -> c_int;
pub type J2K_TranslateChat = unsafe extern "stdcall" fn(*const c_char) -> *mut c_char;
pub type J2K_TranslateFM = unsafe extern "stdcall" fn(*const c_char) -> *mut c_char;
/// 알 수 없는 함수
pub type J2K_TranslateMM = unsafe extern "stdcall" fn(*const c_char) -> *mut c_char;
pub type J2K_TranslateMMEx = unsafe extern "stdcall" fn(c_int, *const c_char) -> *mut c_char;
/// EHND를 사용하지 않는 번역 함수
pub type J2K_TranslateMMNT = unsafe extern "stdcall" fn(c_int, *const c_char) -> *mut c_char;
/// EHND를 사용하는 번역 함수
pub type J2K_TranslateMMNTW = unsafe extern "stdcall" fn(c_int, *const u16) -> *mut u16;

/// EzTrans 엔진을 관리하는 구조체
pub struct EzTransEngine {
    pub module: HMODULE,
    /// 이지트랜스 엔진이 처리할 수 없는 문자가 문자열에 들어있는지 확인하는 역할.
    pub special_chars: HashSet<char>,

    // 함수 포인터들
    pub free_mem: Option<J2K_FreeMem>,
    pub get_prior_dict: Option<J2K_GetPriorDict>,
    pub get_property: Option<J2K_GetProperty>,
    pub initialize: Option<J2K_Initialize>,
    /// 확장 초기화 함수, 이게 안 된다면, enhd가 적용되지 않은 eztrans dll이라는 얘기.
    /// 따라서, translate_mmntw를 쓸 수 없다.
    pub initialize_ex: Option<J2K_InitializeEx>,
    pub reload_user_dict: Option<J2K_ReloadUserDict>,
    pub set_del_jpn: Option<J2K_SetDelJPN>,
    pub set_field: Option<J2K_SetField>,
    pub set_hnj2han: Option<J2K_SetHnj2han>,
    pub set_jwin: Option<J2K_SetJWin>,
    pub set_prior_dict: Option<J2K_SetPriorDict>,
    pub set_property: Option<J2K_SetProperty>,
    pub stop_translation: Option<J2K_StopTranslation>,
    pub terminate: Option<J2K_Terminate>,
    pub translate_chat: Option<J2K_TranslateChat>,
    pub translate_fm: Option<J2K_TranslateFM>,
    /// 일반 번역 모드에서 텍스트를 번역합니다.
    pub translate_mm: Option<J2K_TranslateMM>,
    /// 확장된 일반 번역 모드에서 텍스트를 번역합니다.
    pub translate_mmex: Option<J2K_TranslateMMEx>,
    /// EHND를 사용하지 않는 번역 함수 - No Thread 모드에서 텍스트를 번역합니다. (멀티스레드 환경에서 유용)
    pub translate_mmnt: Option<J2K_TranslateMMNT>,
    /// EHND를 사용하는 번역 함수 - No Thread 모드에서 와이드 문자열(Unicode) 텍스트를 번역합니다.
    pub translate_mmntw: Option<J2K_TranslateMMNTW>,
}

impl EzTransEngine {
    /// EzTrans 엔진을 초기화합니다.
    pub fn new<P: AsRef<Path>>(dll_path: P) -> Result<Self, EzTransError> {
        // DLL 경로를 문자열로 변환
        let path_str = dll_path
            .as_ref()
            .to_str()
            .ok_or(EzTransError::InvalidPath)?;

        // CString으로 변환 (null 종료 문자열)
        let c_path = CString::new(path_str)?;

        // DLL 로드
        let module = unsafe {
            LoadLibraryA(PCSTR(c_path.as_ptr() as *const u8))
                .map_err(|e: WindowsError| EzTransError::DllLoadError(e.to_string()))?
        };

        let special_chars: HashSet<char> = [
            '↔', '◁', '◀', '▷', '▶', '♤', '♠', '♡', '♥', '♧', '♣', '⊙', '◈', '▣', '◐', '◑', '▒',
            '▤', '▥', '▨', '▧', '▦', '▩', '♨', '☏', '☎', '☜', '☞', '↕', '↗', '↙', '↖', '↘', '♩',
            '♬', '㉿', '㈜', '㏇', '™', '㏂', '㏘', '＂', '＇', '∼', 'ˇ', '˘', '˝', '¡', '˚', '˙',
            '˛', '¿', 'ː', '∏', '￦', '℉', '€', '㎕', '㎖', '㎗', 'ℓ', '㎘', '㎣', '㎤', '㎥',
            '㎦', '㎙', '㎚', '㎛', '㎟', '㎠', '㎢', '㏊', '㎍', '㏏', '㎈', '㎉', '㏈', '㎧',
            '㎨', '㎰', '㎱', '㎲', '㎳', '㎴', '㎵', '㎶', '㎷', '㎸', '㎀', '㎁', '㎂', '㎃',
            '㎄', '㎺', '㎻', '㎼', '㎽', '㎾', '㎿', '㎐', '㎑', '㎒', '㎓', '㎔', 'Ω', '㏀',
            '㏁', '㎊', '㎋', '㎌', '㏖', '㏅', '㎭', '㎮', '㎯', '㏛', '㎩', '㎪', '㎫', '㎬',
            '㏝', '㏐', '㏓', '㏃', '㏉', '㏜', '㏆', '┒', '┑', '┚', '┙', '┖', '┕', '┎', '┍', '┞',
            '┟', '┡', '┢', '┦', '┧', '┪', '┭', '┮', '┵', '┶', '┹', '┺', '┽', '┾', '╀', '╁', '╃',
            '╄', '╅', '╆', '╇', '╈', '╉', '╊', '┱', '┲', 'ⅰ', 'ⅱ', 'ⅲ', 'ⅳ', 'ⅴ', 'ⅵ', 'ⅶ', 'ⅷ',
            'ⅸ', 'ⅹ', '½', '⅓', '⅔', '¼', '¾', '⅛', '⅜', '⅝', '⅞', 'ⁿ', '₁', '₂', '₃', '₄', 'Ŋ',
            'đ', 'Ħ', 'Ĳ', 'Ŀ', 'Ł', 'Œ', 'Ŧ', 'ħ', 'ı', 'ĳ', 'ĸ', 'ŀ', 'ł', 'œ', 'ŧ', 'ŋ', 'ŉ',
            '㉠', '㉡', '㉢', '㉣', '㉤', '㉥', '㉦', '㉧', '㉨', '㉩', '㉪', '㉫', '㉬', '㉭',
            '㉮', '㉯', '㉰', '㉱', '㉲', '㉳', '㉴', '㉵', '㉶', '㉷', '㉸', '㉹', '㉺', '㉻',
            '㈀', '㈁', '㈂', '㈃', '㈄', '㈅', '㈆', '㈇', '㈈', '㈉', '㈊', '㈋', '㈌', '㈍',
            '㈎', '㈏', '㈐', '㈑', '㈒', '㈓', '㈔', '㈕', '㈖', '㈗', '㈘', '㈙', '㈚', '㈛',
            'ⓐ', 'ⓑ', 'ⓒ', 'ⓓ', 'ⓔ', 'ⓕ', 'ⓖ', 'ⓗ', 'ⓘ', 'ⓙ', 'ⓚ', 'ⓛ', 'ⓜ', 'ⓝ', 'ⓞ', 'ⓟ', 'ⓠ',
            'ⓡ', 'ⓢ', 'ⓣ', 'ⓤ', 'ⓥ', 'ⓦ', 'ⓧ', 'ⓨ', 'ⓩ', '①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧',
            '⑨', '⑩', '⑪', '⑫', '⑬', '⑭', '⑮', '⒜', '⒝', '⒞', '⒟', '⒠', '⒡', '⒢', '⒣', '⒤', '⒥',
            '⒦', '⒧', '⒨', '⒩', '⒪', '⒫', '⒬', '⒭', '⒮', '⒯', '⒰', '⒱', '⒲', '⒳', '⒴', '⒵', '⑴',
            '⑵', '⑶', '⑷', '⑸', '⑹', '⑺', '⑻', '⑼', '⑽', '⑾', '⑿', '⒀', '⒁', '⒂',
        ]
        .iter()
        .cloned()
        .collect();

        // 엔진 인스턴스 생성
        let mut engine = Self {
            module,
            special_chars,
            free_mem: None,
            get_prior_dict: None,
            get_property: None,
            initialize: None,
            initialize_ex: None,
            reload_user_dict: None,
            set_del_jpn: None,
            set_field: None,
            set_hnj2han: None,
            set_jwin: None,
            set_prior_dict: None,
            set_property: None,
            stop_translation: None,
            terminate: None,
            translate_chat: None,
            translate_fm: None,
            translate_mm: None,
            translate_mmex: None,
            translate_mmnt: None,
            translate_mmntw: None,
        };

        // 필요한 함수 포인터들 로드

        engine.load_functions()?;

        Ok(engine)
    }

    /// 공통 함수: 프로시저 주소를 가져오는 함수
    fn get_proc_address(&self, name: &str) -> Result<*const (), EzTransError> {
        let c_name = CString::new(name)?;
        unsafe {
            GetProcAddress(self.module, PCSTR(c_name.as_ptr() as *const u8))
                .map(|p| p as *const ())
                .ok_or_else(|| {
                    EzTransError::FunctionLoadError(format!("함수를 찾을 수 없음: {}", name))
                })
        }
    }

    /// 각 함수별 로드 메소드들
    fn load_free_mem(&self) -> Result<J2K_FreeMem, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_FreeMem")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_get_prior_dict(&self) -> Result<J2K_GetPriorDict, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_GetPriorDict")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_get_property(&self) -> Result<J2K_GetProperty, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_GetProperty")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_initialize(&self) -> Result<J2K_Initialize, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_Initialize")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_initialize_ex(&self) -> Result<J2K_InitializeEx, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_InitializeEx")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_reload_user_dict(&self) -> Result<J2K_ReloadUserDict, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_ReloadUserDict")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_set_del_jpn(&self) -> Result<J2K_SetDelJPN, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_SetDelJPN")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_set_field(&self) -> Result<J2K_SetField, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_SetField")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_set_hnj2han(&self) -> Result<J2K_SetHnj2han, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_SetHnj2han")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_set_jwin(&self) -> Result<J2K_SetJWin, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_SetJWin")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_set_prior_dict(&self) -> Result<J2K_SetPriorDict, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_SetPriorDict")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_set_property(&self) -> Result<J2K_SetProperty, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_SetProperty")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_stop_translation(&self) -> Result<J2K_StopTranslation, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_StopTranslation")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_terminate(&self) -> Result<J2K_Terminate, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_Terminate")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_translate_chat(&self) -> Result<J2K_TranslateChat, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_TranslateChat")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_translate_fm(&self) -> Result<J2K_TranslateFM, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_TranslateFM")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_translate_mm(&self) -> Result<J2K_TranslateMM, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_TranslateMM")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_translate_mmex(&self) -> Result<J2K_TranslateMMEx, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_TranslateMMEx")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_translate_mmnt(&self) -> Result<J2K_TranslateMMNT, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_TranslateMMNT")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    fn load_translate_mmntw(&self) -> Result<J2K_TranslateMMNTW, EzTransError> {
        let proc_addr = self.get_proc_address("J2K_TranslateMMNTW")?;
        Ok(unsafe { std::mem::transmute(proc_addr) })
    }

    /// DLL에서 함수 포인터들을 로드합니다.
    fn load_functions(&mut self) -> Result<(), EzTransError> {
        // 각 함수 포인터 로드 (필요한 것만 선택적으로)
        self.free_mem = self.load_free_mem().ok();
        self.get_prior_dict = self.load_get_prior_dict().ok();
        self.get_property = self.load_get_property().ok();
        self.initialize = self.load_initialize().ok();
        self.initialize_ex = self.load_initialize_ex().ok();
        self.reload_user_dict = self.load_reload_user_dict().ok();
        self.set_del_jpn = self.load_set_del_jpn().ok();
        self.set_field = self.load_set_field().ok();
        self.set_hnj2han = self.load_set_hnj2han().ok();
        self.set_jwin = self.load_set_jwin().ok();
        self.set_prior_dict = self.load_set_prior_dict().ok();
        self.set_property = self.load_set_property().ok();
        self.stop_translation = self.load_stop_translation().ok();
        self.terminate = self.load_terminate().ok();
        self.translate_chat = self.load_translate_chat().ok();
        self.translate_fm = self.load_translate_fm().ok();
        self.translate_mm = self.load_translate_mm().ok();
        self.translate_mmex = self.load_translate_mmex().ok();
        self.translate_mmnt = self.load_translate_mmnt().ok();
        self.translate_mmntw = self.load_translate_mmntw().ok();

        // 필수 함수들이 로드되었는지 확인
        if self.initialize.is_none() && self.initialize_ex.is_none() {
            return Err(EzTransError::FunctionLoadError(
                "필수 초기화 함수를 찾을 수 없습니다.".to_string(),
            ));
        }

        if self.terminate.is_none() {
            return Err(EzTransError::FunctionLoadError(
                "종료 함수를 찾을 수 없습니다.".to_string(),
            ));
        }

        Ok(())
    }

    /// EzTrans 엔진을 기본 설정으로 초기화합니다.
    pub fn initialize(&self) -> Result<(), EzTransError> {
        let initialize_fn = self.initialize.ok_or_else(|| {
            EzTransError::FunctionLoadError("초기화 함수가 로드되지 않았습니다.".to_string())
        })?;

        let result = unsafe { initialize_fn() };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "initialize 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// EzTrans 엔진을 사용자 정의 설정으로 초기화합니다.
    pub fn initialize_ex(&self, path1: &str, path2: &str) -> Result<(), EzTransError> {
        let initialize_ex_fn = self.initialize_ex.ok_or_else(|| {
            EzTransError::FunctionLoadError("확장 초기화 함수가 로드되지 않았습니다.".to_string())
        })?;

        let c_path1 = CString::new(path1)?;
        let c_path2 = CString::new(path2)?;

        let result = unsafe { initialize_ex_fn(c_path1.as_ptr(), c_path2.as_ptr()) };
        if result != 1 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "initialize_ex 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// EzTrans 엔진을 종료합니다.
    pub fn terminate(&self) -> Result<(), EzTransError> {
        let terminate_fn = self.terminate.ok_or_else(|| {
            EzTransError::FunctionLoadError("종료 함수가 로드되지 않았습니다.".to_string())
        })?;

        let result = unsafe { terminate_fn() };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "terminate 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// 일반 번역 모드에서 텍스트를 번역합니다.
    pub fn translate_mm(&self, text: &str) -> Result<String, EzTransError> {
        let translate_fn = self.translate_mm.ok_or_else(|| {
            EzTransError::FunctionLoadError("번역 함수가 로드되지 않았습니다.".to_string())
        })?;

        let c_text = CString::new(text)?;

        let result_ptr = unsafe { translate_fn(c_text.as_ptr()) };
        if result_ptr.is_null() {
            return Err(EzTransError::FunctionCallFailed(format!(
                "translate 함수가 실패했습니다. (포인터: {:?})",
                result_ptr
            )));
        }

        // C 문자열을 Rust 문자열로 변환
        let result = unsafe {
            let c_str = std::ffi::CStr::from_ptr(result_ptr);
            let string = c_str.to_string_lossy().into_owned();

            // 메모리 해제 (FreeMem 함수가 있는 경우)
            if let Some(free_mem_fn) = self.free_mem {
                free_mem_fn(result_ptr as *mut c_void);
            }

            string
        };

        Ok(result)
    }

    /// EHND를 사용하여 번역합니다.
    pub fn translate_mmntw(&self, input: &str) -> Result<String, EzTransError> {
        // Convert input to UTF-16 with NULL terminator
        let input_wide: Vec<u16> = input.encode_utf16().chain(std::iter::once(0)).collect();

        let translate_mmntw = self.translate_mmntw.ok_or_else(|| {
            EzTransError::FunctionLoadError(
                "translate_mmntw 함수가 로드되지 않았습니다.".to_string(),
            )
        })?;

        let ret = unsafe { translate_mmntw(0, input_wide.as_ptr()) };
        if ret.is_null() {
            return Err(EzTransError::TranslationError(TransErr::NullPointer));
        }

        // 안전하게 UTF-16 문자열 처리 후 메모리 해제
        let result = unsafe {
            let len = (0..).find(|&i| *ret.add(i) == 0).unwrap_or(0);
            let result = String::from_utf16(&std::slice::from_raw_parts(ret, len))?;

            // 메모리 해제
            if let Some(free_mem) = self.free_mem {
                free_mem(ret as *mut c_void);
            }

            result
        };

        Ok(result)
    }

    pub fn translate_mmnt(&self, input: &str) -> Result<String, EzTransError> {
        // Convert input to Shift-JIS
        let input_sjis = encoding_rs::SHIFT_JIS.encode(input).0.to_vec();

        let translate_mmnt = self.translate_mmnt.ok_or_else(|| {
            EzTransError::FunctionLoadError(
                "translate_mmnt 함수가 로드되지 않았습니다.".to_string(),
            )
        })?;

        let ret = unsafe { translate_mmnt(0, input_sjis.as_ptr() as *mut c_char) };
        if ret.is_null() {
            return Err(EzTransError::TranslationError(TransErr::NullPointer));
        }

        // EUC-KR에서 UTF-8로 변환 후 메모리 해제
        let result = unsafe {
            let c_str = CStr::from_ptr(ret);
            let (decoded, _, had_errors) = encoding_rs::EUC_KR.decode(c_str.to_bytes());

            // 메모리 해제
            if let Some(free_mem) = self.free_mem {
                free_mem(ret as *mut c_void);
            }

            if had_errors {
                return Err(EzTransError::TranslationError(TransErr::EucKrDecodeFailed));
            }

            decoded.into_owned()
        };

        Ok(result)
    }

    pub fn default_translate(&self, input: &str) -> Result<String, EzTransError> {
        // 인코딩이 필요한지 빠르게 확인 (한글/특수문자 있는지)
        let needs_encoding = input.chars().any(|c| {
            c == '@'
                || c == '\0'
                || self.is_hangul_range(c as u32)
                || self.special_chars.contains(&c)
        });

        // 필요한 경우만 인코딩 수행
        let encoded = if needs_encoding {
            self.hangul_encode(input)
        } else {
            input.to_string()
        };

        // EHND 또는 기본 번역 선택
        let translated = if self.initialize_ex.is_some() {
            self.translate_mmntw(&encoded)?
        } else {
            self.translate_mmnt(&encoded)?
        };

        // 필요한 경우만 디코딩
        let result = if needs_encoding {
            self.hangul_decode(&translated)
        } else {
            translated
        };

        Ok(result)
    }

    /// 한글 및 특수 문자를 16진수 유니코드로 인코딩
    pub fn hangul_encode(&self, input: &str) -> String {
        let mut output = String::with_capacity(input.len() * 2);

        for c in input.chars() {
            if c == '@' || c == '\0' || self.is_hangul_range(c as u32) {
                write!(&mut output, "+x{:04X}", c as u32).unwrap();
            } else if self.special_chars.contains(&c) {
                write!(&mut output, "+X{:04X}", c as u32).unwrap();
            } else {
                output.push(c);
            }
        }

        output
    }

    /// 한글 문자 범위 판별 (유니코드 범위 확인)
    #[inline]
    pub const fn is_hangul_range(&self, code: u32) -> bool {
        (code >= 0x1100 && code <= 0x11FF) || // Hangul Jamo
    (code >= 0x3130 && code <= 0x318F) || // Hangul Compatibility Jamo
    (code >= 0xA960 && code <= 0xA97F) || // Hangul Jamo Extended-A
    (code >= 0xAC00 && code <= 0xD7A3) || // Hangul Syllables
    (code >= 0xD7B0 && code <= 0xD7FF) // Hangul Jamo Extended-B
    }

    /// 16진수 유니코드로 인코딩된 문자열 디코딩
    pub fn hangul_decode(&self, input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '+' {
                if let Some(&next) = chars.peek() {
                    if next == 'x' || next == 'X' {
                        chars.next(); // 'x'/'X' 소비

                        // 4자리 16진수 추출
                        let hex: String = chars.by_ref().take(4).collect();

                        // 유효한 16진수면 디코딩
                        if hex.len() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                            if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                if let Some(decoded_char) = std::char::from_u32(code) {
                                    output.push(decoded_char);
                                    continue;
                                }
                            }
                        }

                        // 디코딩 실패 시 원본 문자열 유지
                        output.push('+');
                        if next == 'x' || next == 'X' {
                            output.push(next);
                        }
                        output.push_str(&hex);
                        continue;
                    }
                }
                output.push('+');
            } else {
                output.push(ch);
            }
        }

        output
    }

    /// 채팅 모드에서 일본어 텍스트를 한국어로 번역합니다.
    pub fn translate_chat(&self, text: &str) -> Result<String, EzTransError> {
        let translate_fn = self.translate_chat.ok_or_else(|| {
            EzTransError::FunctionLoadError("채팅 번역 함수가 로드되지 않았습니다.".to_string())
        })?;

        let c_text = CString::new(text)?;

        let result_ptr = unsafe { translate_fn(c_text.as_ptr()) };
        if result_ptr.is_null() {
            return Err(EzTransError::FunctionCallFailed(format!(
                "translate_chat 함수가 실패했습니다. (포인터: {:?})",
                result_ptr
            )));
        }

        // C 문자열을 Rust 문자열로 변환
        let result = unsafe {
            let c_str = std::ffi::CStr::from_ptr(result_ptr);
            let string = c_str.to_string_lossy().into_owned();

            // 메모리 해제 (FreeMem 함수가 있는 경우)
            if let Some(free_mem_fn) = self.free_mem {
                free_mem_fn(result_ptr as *mut c_void);
            }

            string
        };

        Ok(result)
    }

    /// 번역 분야를 설정합니다.
    pub fn set_field(&self, field: c_int) -> Result<(), EzTransError> {
        let set_field_fn = self.set_field.ok_or_else(|| {
            EzTransError::FunctionLoadError("분야 설정 함수가 로드되지 않았습니다.".to_string())
        })?;

        let result = unsafe { set_field_fn(field) };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "set_field 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// 한자를 한글로 변환하는 옵션을 설정합니다.
    pub fn set_hnj2han(&self, option: c_int) -> Result<(), EzTransError> {
        let set_hnj2han_fn = self.set_hnj2han.ok_or_else(|| {
            EzTransError::FunctionLoadError(
                "한자->한글 설정 함수가 로드되지 않았습니다.".to_string(),
            )
        })?;

        let result = unsafe { set_hnj2han_fn(option) };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "set_hnj2han 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// 사용자 사전을 다시 로드합니다.
    pub fn reload_user_dict(&self) -> Result<(), EzTransError> {
        let reload_fn = self.reload_user_dict.ok_or_else(|| {
            EzTransError::FunctionLoadError("사전 로드 함수가 로드되지 않았습니다.".to_string())
        })?;

        let result = unsafe { reload_fn() };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "reload_user_dict 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// 일본어 문장 구분 기능을 설정합니다.
    pub fn set_del_jpn(&self, option: c_int) -> Result<(), EzTransError> {
        let set_del_jpn_fn = self.set_del_jpn.ok_or_else(|| {
            EzTransError::FunctionLoadError(
                "일본어 문장 구분 함수가 로드되지 않았습니다.".to_string(),
            )
        })?;

        let result = unsafe { set_del_jpn_fn(option) };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "set_del_jpn 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// J-Win 모드를 설정합니다.
    pub fn set_jwin(&self, option: c_int) -> Result<(), EzTransError> {
        let set_jwin_fn = self.set_jwin.ok_or_else(|| {
            EzTransError::FunctionLoadError(
                "J-Win 모드 설정 함수가 로드되지 않았습니다.".to_string(),
            )
        })?;

        let result = unsafe { set_jwin_fn(option) };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "set_jwin 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// 사용자 사전의 우선순위를 설정합니다.
    pub fn set_prior_dict(&self, dict_path: &str) -> Result<(), EzTransError> {
        let set_prior_dict_fn = self.set_prior_dict.ok_or_else(|| {
            EzTransError::FunctionLoadError(
                "사전 우선순위 설정 함수가 로드되지 않았습니다.".to_string(),
            )
        })?;

        let c_path = CString::new(dict_path)?;

        let result = unsafe { set_prior_dict_fn(c_path.as_ptr()) };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "set_prior_dict 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// 특정 속성의 값을 설정합니다.
    pub fn set_property(&self, property_id: c_int, value: c_int) -> Result<(), EzTransError> {
        let set_property_fn = self.set_property.ok_or_else(|| {
            EzTransError::FunctionLoadError("속성 설정 함수가 로드되지 않았습니다.".to_string())
        })?;

        let result = unsafe { set_property_fn(property_id, value) };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(
                "set_property 함수가 실패했습니다.".to_string(),
            ));
        }

        Ok(())
    }

    /// 특정 속성의 현재 값을 가져옵니다.
    pub fn get_property(&self, property_id: c_int) -> Result<c_int, EzTransError> {
        let get_property_fn = self.get_property.ok_or_else(|| {
            EzTransError::FunctionLoadError("속성 조회 함수가 로드되지 않았습니다.".to_string())
        })?;

        let result = unsafe { get_property_fn(property_id) };
        // 속성 값 조회는 일반적으로 실패하지 않으므로 결과를 그대로 반환
        Ok(result)
    }

    /// 현재 진행 중인 번역 작업을 중지합니다.
    pub fn stop_translation(&self) -> Result<(), EzTransError> {
        let stop_fn = self.stop_translation.ok_or_else(|| {
            EzTransError::FunctionLoadError("번역 중지 함수가 로드되지 않았습니다.".to_string())
        })?;

        let result = unsafe { stop_fn() };
        if result != 0 {
            return Err(EzTransError::FunctionCallFailed(format!(
                "stop_translation 함수가 실패했습니다. (코드: {})",
                result
            )));
        }

        Ok(())
    }

    /// 전문 번역 모드에서 텍스트를 번역합니다.
    pub fn translate_fm(&self, text: &str) -> Result<String, EzTransError> {
        let translate_fn = self.translate_fm.ok_or_else(|| {
            EzTransError::FunctionLoadError("전문 번역 함수가 로드되지 않았습니다.".to_string())
        })?;

        let c_text = CString::new(text)?;

        let result_ptr = unsafe { translate_fn(c_text.as_ptr()) };
        if result_ptr.is_null() {
            return Err(EzTransError::FunctionCallFailed(format!(
                "translate_fm 함수가 실패했습니다. (포인터: {:?})",
                result_ptr
            )));
        }

        // C 문자열을 Rust 문자열로 변환
        let result = unsafe {
            let c_str = std::ffi::CStr::from_ptr(result_ptr);
            let string = c_str.to_string_lossy().into_owned();

            // 메모리 해제 (FreeMem 함수가 있는 경우)
            if let Some(free_mem_fn) = self.free_mem {
                free_mem_fn(result_ptr as *mut c_void);
            }

            string
        };

        Ok(result)
    }
}

// Drop 트레이트를 구현하여 자동으로 DLL을 언로드
impl Drop for EzTransEngine {
    fn drop(&mut self) {
        // 엔진 종료 시도 (에러는 무시)
        if let Some(terminate_fn) = self.terminate {
            unsafe {
                terminate_fn();
            }
        }

        // DLL 언로드
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}
