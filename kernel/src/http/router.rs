//! Tiny exact-match router.
//!
//! `Router` holds `(Method, &'static str path) → Handler` mappings.
//! Stage 4 ships exact-match only; path-param matching arrives in
//! stage 5 with a real radix tree.
use alloc::vec::Vec;

use super::parser::{Method, Request};
use super::response::{Response, Status};

pub type Handler = fn(&Request<'_>) -> Response;

pub struct Router {
    routes: Vec<Route>,
    fallback: Handler,
}

struct Route {
    method: Method,
    path:   &'static str,
    handler: Handler,
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new(), fallback: not_found }
    }

    pub fn add(mut self, method: Method, path: &'static str, h: Handler) -> Self {
        self.routes.push(Route { method, path, handler: h });
        self
    }

    pub fn fallback(mut self, h: Handler) -> Self { self.fallback = h; self }

    pub fn dispatch(&self, req: &Request<'_>) -> Response {
        // Single-pass: track whether *any* route shares the path while
        // looking for the exact (method, path) match.  Avoids the
        // O(2N) "first scan + second scan for 405" pattern.
        let mut same_path = false;
        for r in &self.routes {
            if r.path == req.path {
                if r.method == req.method { return (r.handler)(req); }
                same_path = true;
            }
        }
        if same_path {
            return Response::new(Status::METHOD_NOT_ALLOWED);
        }
        (self.fallback)(req)
    }
}

fn not_found(_req: &Request<'_>) -> Response {
    Response::new(Status::NOT_FOUND).body(b"not found".to_vec())
}
