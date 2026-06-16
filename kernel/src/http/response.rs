//! HTTP/1.1 response builder.
//!
//! Optimized for the common case: small responses with a handful of
//! headers and a borrowed body.  No allocation needed unless the body
//! is dynamically built.
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Copy, Clone, Debug)]
pub struct Status { pub code: u16, pub reason: &'static str }

impl Status {
    pub const OK:                   Self = Self { code: 200, reason: "OK" };
    pub const CREATED:              Self = Self { code: 201, reason: "Created" };
    pub const NO_CONTENT:           Self = Self { code: 204, reason: "No Content" };
    pub const BAD_REQUEST:          Self = Self { code: 400, reason: "Bad Request" };
    pub const NOT_FOUND:            Self = Self { code: 404, reason: "Not Found" };
    pub const METHOD_NOT_ALLOWED:   Self = Self { code: 405, reason: "Method Not Allowed" };
    pub const INTERNAL_ERROR:       Self = Self { code: 500, reason: "Internal Server Error" };
    pub const SERVICE_UNAVAILABLE:  Self = Self { code: 503, reason: "Service Unavailable" };
}

#[derive(Default)]
pub struct Response {
    pub status:  Option<Status>,
    pub headers: Vec<(String, String)>,
    pub body:    Vec<u8>,
}

impl Response {
    pub fn new(status: Status) -> Self {
        Self { status: Some(status), headers: Vec::new(), body: Vec::new() }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((String::from(name), String::from(value)));
        self
    }

    pub fn body(mut self, b: impl Into<Vec<u8>>) -> Self {
        self.body = b.into();
        self
    }

    /// Serialize into wire bytes.  Inserts `Content-Length` automatically.
    pub fn build(&self) -> Vec<u8> {
        let s = self.status.unwrap_or(Status::INTERNAL_ERROR);
        let mut out = Vec::with_capacity(64 + self.body.len());
        let _ = write_into(&mut out, format_args!(
            "HTTP/1.1 {} {}\r\n", s.code, s.reason
        ));
        for (name, value) in &self.headers {
            let _ = write_into(&mut out, format_args!("{}: {}\r\n", name, value));
        }
        let _ = write_into(&mut out, format_args!(
            "Content-Length: {}\r\n\r\n", self.body.len()
        ));
        out.extend_from_slice(&self.body);
        out
    }
}

fn write_into(out: &mut Vec<u8>, args: core::fmt::Arguments) -> core::fmt::Result {
    use core::fmt::Write;
    struct Sink<'a>(&'a mut Vec<u8>);
    impl<'a> Write for Sink<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0.extend_from_slice(s.as_bytes()); Ok(())
        }
    }
    Sink(out).write_fmt(args)
}
