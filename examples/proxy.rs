// Named Pipe 프록시 서버 진입점
use eztrans_rs::server::TransProxyServer;

fn main() {
    let mut server = TransProxyServer::new();

    match server.start() {
        Ok(_) => {
            server.run();
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Failed to start proxy server: {}", e);
            std::process::exit(1);
        }
    }
}
