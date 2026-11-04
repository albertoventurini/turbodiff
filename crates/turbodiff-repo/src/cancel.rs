//! Stopping a computation whose result is no longer needed (TECH.md §9).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{Error, Result};

/// A flag that long-running operations check between steps. Clones share the
/// flag, so the caller keeps one and hands another to the operation.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// `Err(Cancelled)` once [`Cancel::cancel`] has been called.
    pub(crate) fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_the_flag() {
        let cancel = Cancel::new();
        let handed_out = cancel.clone();
        assert!(handed_out.check().is_ok());
        cancel.cancel();
        assert!(handed_out.is_cancelled());
        assert!(matches!(handed_out.check(), Err(Error::Cancelled)));
    }
}
