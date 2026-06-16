//! Radix-tree HTTP path router with `:param` support.
//!
//! Each node is `[static_children | param_child | wildcard_child | handler]`.
//! Lookup walks the path segment-by-segment; static children are matched
//! exact, then `:param` / `*wildcard` fall-through.
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::parser::Method;
use super::response::Response;

pub type Handler = fn(&super::parser::Request<'_>, &Params<'_>) -> Response;

#[derive(Default)]
pub struct Params<'a> { pairs: Vec<(&'a str, &'a str)> }

impl<'a> Params<'a> {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }
    pub fn insert(&mut self, key: &'a str, value: &'a str) { self.pairs.push((key, value)); }
}

#[derive(Default)]
struct Node {
    seg:        String,
    handlers:   Vec<(Method, Handler)>,
    statics:    Vec<Node>,
    param:      Option<Box<Node>>,    // `:name`
    wildcard:   Option<Box<Node>>,    // `*rest`
    param_name: String,
}

#[derive(Default)]
pub struct RadixRouter { root: Node }

impl RadixRouter {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, method: Method, path: &str, handler: Handler) {
        let mut node = &mut self.root;
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            node = if let Some(rest) = seg.strip_prefix(':') {
                let n = node.param.get_or_insert_with(|| Box::new(Node::default()));
                n.param_name = String::from(rest);
                n.as_mut()
            } else if let Some(_rest) = seg.strip_prefix('*') {
                let n = node.wildcard.get_or_insert_with(|| Box::new(Node::default()));
                n.as_mut()
            } else {
                if let Some(idx) = node.statics.iter().position(|c| c.seg == seg) {
                    &mut node.statics[idx]
                } else {
                    node.statics.push(Node { seg: String::from(seg), ..Node::default() });
                    node.statics.last_mut().unwrap()
                }
            };
        }
        node.handlers.push((method, handler));
    }

    pub fn lookup<'a>(&'a self, method: Method, path: &'a str)
        -> Option<(Handler, Params<'a>)>
    {
        let mut node = &self.root;
        let mut params = Params::default();
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        for (i, seg) in segments.iter().enumerate() {
            if let Some(c) = node.statics.iter().find(|c| c.seg == *seg) {
                node = c;
            } else if let Some(p) = &node.param {
                params.insert(&p.param_name, seg);
                node = p;
            } else if let Some(w) = &node.wildcard {
                let _ = i;
                node = w;
                break;
            } else {
                return None;
            }
        }
        node.handlers.iter()
            .find(|(m, _)| *m == method)
            .map(|(_, h)| (*h, params))
    }
}
