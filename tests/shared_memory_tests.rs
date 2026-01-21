// Shared Memory Tests

use eztrans_rs::EzTransEngine;
use std::{env, error::Error};
use windows_shared_memory::{Client, ReceiveMessage, Server};

#[test]
fn test_shared_memory_translation() -> Result<(), Box<dyn Error>> {
    let server = Server::new(None).unwrap();

    let data = "こんにちは。".as_bytes();

    // 프로젝트 내 eztrans_dll 폴더 경로 사용 (기본값)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let default_dll_path = format!("{}/../eztrans_dll/J2KEngine.dll", manifest_dir);
    let default_dat_path = format!("{}/../eztrans_dll/Dat", manifest_dir);

    // 환경변수로 경로 오버라이드 가능
    let args: Vec<String> = env::args().collect();
    let mut folder_path: Option<&str> = None;

    for arg in args.iter().skip(1) {
        if arg.starts_with("--folder_path=") {
            folder_path = Some(arg.trim_start_matches("--folder_path=").trim_matches('"'));
        }
    }

    // EzTrans 엔진 초기화
    let (dll_path, dat_path) = if let Some(path) = folder_path {
        (format!("{}/J2KEngine.dll", path), format!("{}/Dat", path))
    } else {
        (default_dll_path, default_dat_path)
    };

    let ez_trans = EzTransEngine::new(&dll_path)?;

    // 엔진 초기화
    ez_trans.initialize_ex("CSUSER123455", &dat_path)?;

    let client = Client::new(None)?;

    server.send(&data)?;

    loop {
        let receive_server = client.receive(Some(u32::MAX));

        if let ReceiveMessage::Message(recv_message) = receive_server {
            match ez_trans.default_translate(&recv_message) {
                Ok(translated) => {
                    client.send(&translated.as_bytes())?;
                }
                Err(error) => {
                    client.send(&format!("Translation error: {}", &error).as_bytes())?;
                }
            }
            break;
        } else if let ReceiveMessage::Exit = receive_server {
            break;
        } else if let ReceiveMessage::MessageError(e) = receive_server {
            eprintln!("Error: {}", e);
            break;
        } else if let ReceiveMessage::Timeout = receive_server {
            println!("Timeout");
        }
    }

    let receive_client = server.receive(Some(u32::MAX));

    match receive_client {
        ReceiveMessage::Message(recv_message) => {
            println!("서버가 받은 메세지: {:?}", recv_message);
        }
        ReceiveMessage::Exit => {
            println!("Exiting...");
        }
        ReceiveMessage::MessageError(e) => {
            eprintln!("Error: {}", e);
        }
        ReceiveMessage::Timeout => {
            println!("Timeout");
        }
    };

    // Terminate EzTransLib
    ez_trans.terminate()?;

    Ok(())
}
