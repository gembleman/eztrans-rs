// IPC Protocol Tests

use std::mem::size_of;
use eztrans_rs::ipc_protocol::*;

#[test]
fn test_message_header_size() {
    // MessageHeader should be 16 bytes (4 + 4 + 8)
    assert_eq!(size_of::<MessageHeader>(), 16);
}

#[test]
fn test_initialize_response_size() {
    // InitializeResponse should be 8 bytes (4 + 4 for bool with padding)
    assert_eq!(size_of::<InitializeResponse>(), 8);
}

#[test]
fn test_generic_response_size() {
    // GenericResponse should be 4 bytes
    assert_eq!(size_of::<GenericResponse>(), 4);
}

#[test]
fn test_initialize_request_size() {
    // 260 * 2 = 520 bytes for UTF-16 path
    assert_eq!(size_of::<InitializeRequest>(), 520);
}

#[test]
fn test_translate_mmnt_request_size() {
    // 4 bytes (data0) + 4096 bytes (text)
    assert_eq!(size_of::<TranslateMMNTRequest>(), 4100);
}

#[test]
fn test_translate_mmnt_response_size() {
    // 4 bytes (status) + 4 bytes (result_code) + 4096 bytes (translated)
    assert_eq!(size_of::<TranslateMMNTResponse>(), 4104);
}

#[test]
fn test_translate_mmntw_request_size() {
    // 4 bytes (data0) + 4096 * 2 bytes (UTF-16 text) = 8196 bytes
    // (packed(8) alignment doesn't add padding for this layout)
    assert_eq!(size_of::<TranslateMMNTWRequest>(), 8196);
}

#[test]
fn test_translate_mmntw_response_size() {
    // 4 bytes (status) + 4 bytes (result_code) + 4096 * 2 bytes (UTF-16 translated)
    assert_eq!(size_of::<TranslateMMNTWResponse>(), 8200);
}

#[test]
fn test_command_values() {
    assert_eq!(Command::Initialize as u32, 1);
    assert_eq!(Command::Terminate as u32, 2);
    assert_eq!(Command::TranslateMMNT as u32, 3);
    assert_eq!(Command::TranslateMMNTW as u32, 4);
    assert_eq!(Command::ReloadUserDict as u32, 5);
    assert_eq!(Command::SetProperty as u32, 6);
    assert_eq!(Command::Shutdown as u32, 7);
    assert_eq!(Command::Ping as u32, 8);
}

#[test]
fn test_status_values() {
    assert_eq!(Status::Success as u32, 0);
    assert_eq!(Status::Error as u32, 1);
    assert_eq!(Status::NotInitialized as u32, 2);
    assert_eq!(Status::InvalidParameter as u32, 3);
}

#[test]
fn test_message_header_creation() {
    let header = MessageHeader {
        command: Command::Ping as u32,
        payload_size: 0,
        request_id: 12345,
    };

    assert_eq!(header.command, 8);
    assert_eq!(header.payload_size, 0);
    assert_eq!(header.request_id, 12345);
}

#[test]
fn test_buffer_initialization() {
    let mut request = TranslateMMNTRequest {
        data0: 0,
        text: [0; 4096],
    };

    // Test writing to buffer
    let test_str = b"Hello";
    request.text[..test_str.len()].copy_from_slice(test_str);

    assert_eq!(&request.text[..5], b"Hello");
    assert_eq!(request.text[5], 0);
}

#[test]
fn test_wide_buffer_initialization() {
    let mut request = TranslateMMNTWRequest {
        data0: 0,
        text: [0; 4096],
    };

    // Test writing UTF-16 to buffer
    let test_str = "테스트";
    let encoded: Vec<u16> = test_str.encode_utf16().collect();
    request.text[..encoded.len()].copy_from_slice(&encoded);

    let decoded = String::from_utf16_lossy(&request.text[..encoded.len()]);
    assert_eq!(decoded, test_str);
}

#[test]
fn test_set_property_request_size() {
    // 4 bytes (property_id) + 4 bytes (value) = 8 bytes
    assert_eq!(size_of::<SetPropertyRequest>(), 8);
}

#[test]
fn test_set_property_request_creation() {
    let request = SetPropertyRequest {
        property_id: 1,
        value: 100,
    };

    assert_eq!(request.property_id, 1);
    assert_eq!(request.value, 100);
}

#[test]
fn test_command_try_from_valid() {
    assert_eq!(Command::try_from(1).unwrap(), Command::Initialize);
    assert_eq!(Command::try_from(2).unwrap(), Command::Terminate);
    assert_eq!(Command::try_from(3).unwrap(), Command::TranslateMMNT);
    assert_eq!(Command::try_from(4).unwrap(), Command::TranslateMMNTW);
    assert_eq!(Command::try_from(5).unwrap(), Command::ReloadUserDict);
    assert_eq!(Command::try_from(6).unwrap(), Command::SetProperty);
    assert_eq!(Command::try_from(7).unwrap(), Command::Shutdown);
    assert_eq!(Command::try_from(8).unwrap(), Command::Ping);
}

#[test]
fn test_command_try_from_invalid() {
    assert!(Command::try_from(0).is_err());
    assert!(Command::try_from(9).is_err());
    assert!(Command::try_from(100).is_err());
}
