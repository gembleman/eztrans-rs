// Shared Memory 서버 진입점
use std::{env, error::Error, u32};
use eztrans_rs::EzTransEngine;
use windows_shared_memory::{Client, ReceiveMessage};

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mut folder_path: Option<&str> = None;

    for arg in args.iter().skip(1) {
        if arg.starts_with("--folder_path=") {
            folder_path = Some(arg.trim_start_matches("--folder_path=").trim_matches('"'));
        }
    }

    let ez_trans = if folder_path.is_none() {
        EzTransEngine::new("C:/Program Files (x86)/ChangShinSoft/ezTrans XP/J2KEngine.dll")?
    } else {
        EzTransEngine::new(format!("{}/J2KEngine.dll", folder_path.unwrap()))?
    };

    ez_trans.initialize_ex(
        "CSUSER123455",
        "C:/Program Files (x86)/ChangShinSoft/ezTrans XP/Dat",
    )?;

    let client = Client::new(None)?;

    loop {
        let receive_server = client.receive(Some(u32::MAX));

        if let ReceiveMessage::Message(recv_message) = receive_server {
            match ez_trans.default_translate(&recv_message) {
                Ok(translated) => {
                    client.send(translated.as_bytes())?;
                }
                Err(error) => {
                    client.send(format!("Translation error: {}", &error).as_bytes())?;
                }
            }
        } else if let ReceiveMessage::Exit = receive_server {
            break;
        } else if let ReceiveMessage::MessageError(e) = receive_server {
            client.send(format!("Translation error_2: {}", &e).as_bytes())?;
            break;
        } else if let ReceiveMessage::Timeout = receive_server {
            client.send("Translation error_3: timeout".as_bytes())?;
        }
    }

    ez_trans.terminate()?;
    Ok(())
}
