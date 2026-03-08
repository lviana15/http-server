use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use crate::http::parse_request;
use crate::router::Router;
use crate::thread_pool::ThreadPool;

const BAD_REQUEST: &[u8] = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
const TOO_LARGE: &[u8] = b"HTTP/1.1 413 Content Too Large\r\nContent-Length: 0\r\n\r\n";
const MAX_HEADERS_SIZE: usize = 8 * 1024; // 8 KB
const MAX_BODY_SIZE: usize = 1 * 1024 * 1024; // 1 MB

pub struct Server {
    router: Arc<Router>,
    pool: ThreadPool,
}

impl Server {
    pub fn new(
        router: Router,
        threads: usize,
    ) -> Result<Self, crate::thread_pool::PoolCreationError> {
        let pool = ThreadPool::new(threads)?;
        Ok(Self {
            router: Arc::new(router),
            pool,
        })
    }

    pub fn run(&self, url: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(url)?;
        println!("Listening on {}", url);

        for stream in listener.incoming() {
            let stream = stream?;
            let router_clone = Arc::clone(&self.router);
            self.pool
                .execute(move || handle_connection(stream, router_clone));
        }

        Ok(())
    }
}

fn handle_connection(mut stream: TcpStream, router: Arc<Router>) {
    if try_handle(&stream, &router).is_err() {
        let _ = stream.write_all(BAD_REQUEST);
    }
}

fn try_handle(mut stream: &TcpStream, router: &Router) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream);

    let mut raw_headers = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if raw_headers.len() + line.len() > MAX_HEADERS_SIZE {
            stream.write_all(TOO_LARGE)?;
            return Ok(());
        }
        raw_headers.push_str(&line);
    }

    let content_length: usize = raw_headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    if content_length > MAX_BODY_SIZE {
        stream.write_all(TOO_LARGE)?;
        return Ok(());
    }

    let mut body_bytes = vec![0u8; content_length];
    reader.read_exact(&mut body_bytes)?;
    let body_str = String::from_utf8(body_bytes)?;

    let full_request = format!("{}\r\n\r\n{}", raw_headers.trim_end(), body_str);

    let request = parse_request(&full_request)?;

    let response = router.handle(&request);
    stream.write_all(&response.into_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpResponse;
    use std::thread;
    use std::time::Duration;

    // Helper to spawn a server on a specific port
    fn spawn_test_server(port: u16) {
        let mut router = Router::new();
        router.get("/hello", |_req| HttpResponse::ok("world"));
        router.post("/echo", |req| {
            let content = req.body.as_deref().unwrap_or("empty");
            HttpResponse::ok(content)
        });

        let server = Server::new(router, 2).unwrap();
        let url = format!("127.0.0.1:{}", port);

        thread::spawn(move || {
            let _ = server.run(&url);
        });

        // Wait for server to bind
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn test_server_handles_get_request() {
        spawn_test_server(3001);

        let mut stream =
            TcpStream::connect("127.0.0.1:3001").expect("Could not connect to test server");
        stream
            .write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("world"));
    }

    #[test]
    fn test_server_handles_post_with_body() {
        spawn_test_server(3002);

        let mut stream =
            TcpStream::connect("127.0.0.1:3002").expect("Could not connect to test server");
        stream
            .write_all(
                b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\n\r\nhello-there",
            )
            .unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("hello-there"));
    }

    #[test]
    fn test_server_handles_malformed_request_with_400() {
        spawn_test_server(3003);

        let mut stream =
            TcpStream::connect("127.0.0.1:3003").expect("Could not connect to test server");
        stream.write_all(b"INVALID_REQUEST\r\n\r\n").unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
    }

    #[test]
    fn test_server_dos_protection_large_body() {
        spawn_test_server(3004);

        let mut stream =
            TcpStream::connect("127.0.0.1:3004").expect("Could not connect to test server");

        // Try to send a body larger than 1MB
        let large_size = 2 * 1024 * 1024; // 2 MB
        let req = format!(
            "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            large_size
        );
        stream.write_all(req.as_bytes()).unwrap();
        // Server should drop the connection or respond with 413 immediately without waiting for body

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 413 Content Too Large"));
    }
}
