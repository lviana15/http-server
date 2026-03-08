use dotenv::dotenv;
use std::env;

use http::http::HttpResponse;
use http::router::Router;
use http::server::Server;

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").expect("Missing PORT");
    let url = format!("127.0.0.1:{}", port);

    let mut router = Router::new();

    router.get("/", |_req| HttpResponse::ok("Welcome to the Home Page!"));

    router.get("/hello", |_req| HttpResponse::ok("Hello, World!"));

    router.post("/echo", |req| {
        if let Some(ref body) = req.body {
            HttpResponse::ok(&format!("You sent: {}", body))
        } else {
            HttpResponse::new(400, "Bad Request")
        }
    });

    let server = Server::new(router, 4).expect("Failed to create thread pool for server");
    server.run(&url)
}
