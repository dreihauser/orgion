//! Content-hash "versions" used for optimistic locking (docs/architecture.md §6).

use sha2::{Digest, Sha256};

pub fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_same_hash() {
        assert_eq!(content_hash(b"hello"), content_hash(b"hello"));
    }

    #[test]
    fn different_content_different_hash() {
        assert_ne!(content_hash(b"hello"), content_hash(b"world"));
    }
}
