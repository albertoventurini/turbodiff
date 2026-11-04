//! A least-recently-used cache of file contents, so expanding context
//! doesn't re-read files (TECH.md §4.5).

use std::collections::VecDeque;
use std::sync::Arc;

use gix::ObjectId;

/// The cache keeps at most this many bytes of content.
pub(crate) const CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Contents keyed by object id. A work-tree file's id is the hash of its
/// filtered content, so an edited file never hits a stale entry.
pub(crate) struct ContentCache {
    entries: VecDeque<(ObjectId, Arc<[u8]>)>,
    bytes: usize,
    capacity: usize,
}

impl ContentCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            bytes: 0,
            capacity,
        }
    }

    pub fn get(&mut self, id: &ObjectId) -> Option<Arc<[u8]>> {
        let index = self.entries.iter().position(|(key, _)| key == id)?;
        let entry = self.entries.remove(index)?;
        let data = entry.1.clone();
        self.entries.push_front(entry);
        Some(data)
    }

    /// Adds `data`, evicting the least recently used entries to make room.
    /// Content larger than the whole cache isn't kept.
    pub fn insert(&mut self, id: ObjectId, data: Arc<[u8]>) {
        if self.get(&id).is_some() || data.len() > self.capacity {
            return;
        }
        self.bytes += data.len();
        self.entries.push_front((id, data));
        while self.bytes > self.capacity {
            let (_, evicted) = self.entries.pop_back().expect("bytes come from entries");
            self.bytes -= evicted.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> ObjectId {
        ObjectId::from_bytes_or_panic(&[n; 20])
    }

    fn data(len: usize) -> Arc<[u8]> {
        vec![0; len].into()
    }

    #[test]
    fn evicts_the_least_recently_used_entry() {
        let mut cache = ContentCache::new(10);
        cache.insert(id(1), data(4));
        cache.insert(id(2), data(4));
        assert!(cache.get(&id(1)).is_some());
        cache.insert(id(3), data(4));
        assert!(cache.get(&id(2)).is_none());
        assert!(cache.get(&id(1)).is_some());
        assert!(cache.get(&id(3)).is_some());
    }

    #[test]
    fn content_larger_than_the_cache_is_not_kept() {
        let mut cache = ContentCache::new(10);
        cache.insert(id(1), data(4));
        cache.insert(id(2), data(11));
        assert!(cache.get(&id(2)).is_none());
        assert!(cache.get(&id(1)).is_some());
    }
}
