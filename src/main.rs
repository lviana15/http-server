pub mod thread_pool;

use dotenv::dotenv;
use std::env;
use std::net::{TcpListener, TcpStream};

use crate::thread_pool::ThreadPool;

fn handle_connection(stream: TcpStream) {}

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").expect("Missing PORT");
    let url = format!("127.0.0.1:{}", port);

    let pool = ThreadPool::new(4);

    if let Ok(listener) = TcpListener::bind(&url) {
        println!("Successfully bind to {}", url);
        println!("Waiting for connections...");

        for request in listener.incoming() {
            let req = request.unwrap();

            pool.execute(|| handle_connection(req))
        }
    } else {
        println!("Failed to bind to port {}", port);
    }

    Ok(())
}
