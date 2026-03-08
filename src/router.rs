use crate::http::{HttpRequest, HttpResponse};
use std::collections::HashMap;

pub type Handler = Box<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>;

pub struct Router {
    routes: HashMap<(String, String), Handler>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn get<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes
            .insert(("GET".to_string(), path.to_string()), Box::new(handler));
    }

    pub fn post<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes
            .insert(("POST".to_string(), path.to_string()), Box::new(handler));
    }

    pub fn handle(&self, req: &HttpRequest) -> HttpResponse {
        if let Some(handler) = self.routes.get(&(req.method.clone(), req.path.clone())) {
            handler(req)
        } else {
            HttpResponse::not_found()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_req(method: &str, path: &str) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    #[test]
    fn test_router_get_match() {
        let mut router = Router::new();
        router.get("/test", |_req| HttpResponse::ok("get test ok"));

        let req = dummy_req("GET", "/test");
        let res = router.handle(&req);

        assert_eq!(res.status_code, 200);
        assert_eq!(res.body.unwrap(), "get test ok");
    }

    #[test]
    fn test_router_post_match() {
        let mut router = Router::new();
        router.post("/test", |_req| HttpResponse::ok("post test ok"));

        let req = dummy_req("POST", "/test");
        let res = router.handle(&req);

        assert_eq!(res.status_code, 200);
        assert_eq!(res.body.unwrap(), "post test ok");
    }

    #[test]
    fn test_router_unmatched_route_returns_404() {
        let mut router = Router::new();
        router.get("/test", |_req| HttpResponse::ok("ok"));

        let req1 = dummy_req("POST", "/test"); // Wrong method
        let res1 = router.handle(&req1);
        assert_eq!(res1.status_code, 404);

        let req2 = dummy_req("GET", "/test/foo"); // Wrong path
        let res2 = router.handle(&req2);
        assert_eq!(res2.status_code, 404);
    }

    #[test]
    fn test_router_passes_request_to_handler() {
        let mut router = Router::new();
        router.post("/echo", |req| {
            if req.headers.contains_key("X-Echo") {
                HttpResponse::ok(req.body.as_deref().unwrap_or("no body"))
            } else {
                HttpResponse::new(400, "Bad Request")
            }
        });

        let mut req = dummy_req("POST", "/echo");
        req.headers.insert("X-Echo".to_string(), "true".to_string());
        req.body = Some("echo content".to_string());

        let res = router.handle(&req);
        assert_eq!(res.status_code, 200);
        assert_eq!(res.body.unwrap(), "echo content");

        // Missing header should fail gracefully based on logic
        let req_bad = dummy_req("POST", "/echo");
        let res_bad = router.handle(&req_bad);
        assert_eq!(res_bad.status_code, 400);
    }
}
