//! Bitwise Longest-Prefix-Match trie.
use alloc::boxed::Box;

pub struct Trie<V> { root: Node<V> }

struct Node<V> {
    value: Option<V>,
    children: [Option<Box<Node<V>>>; 2],
}

impl<V> Default for Trie<V> {
    fn default() -> Self { Self { root: Node::default() } }
}
impl<V> Default for Node<V> {
    fn default() -> Self { Self { value: None, children: [None, None] } }
}

impl<V: Clone> Trie<V> {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, key: &[u8], prefix_bits: usize, value: V) {
        let mut node = &mut self.root;
        for i in 0..prefix_bits {
            let bit = ((key[i / 8] >> (7 - (i & 7))) & 1) as usize;
            if node.children[bit].is_none() {
                node.children[bit] = Some(Box::new(Node::default()));
            }
            node = node.children[bit].as_mut().unwrap();
        }
        node.value = Some(value);
    }

    pub fn remove(&mut self, key: &[u8], prefix_bits: usize) {
        let mut node = &mut self.root;
        for i in 0..prefix_bits {
            let bit = ((key[i / 8] >> (7 - (i & 7))) & 1) as usize;
            match node.children[bit].as_deref_mut() {
                Some(n) => node = n,
                None    => return,
            }
        }
        node.value = None;
    }

    pub fn lookup(&self, key: &[u8], max_bits: usize) -> Option<V> {
        let mut node = &self.root;
        let mut best: Option<V> = node.value.clone();
        for i in 0..max_bits {
            let bit = ((key[i / 8] >> (7 - (i & 7))) & 1) as usize;
            match node.children[bit].as_deref() {
                Some(n) => {
                    node = n;
                    if node.value.is_some() { best = node.value.clone(); }
                }
                None => break,
            }
        }
        best
    }
}
