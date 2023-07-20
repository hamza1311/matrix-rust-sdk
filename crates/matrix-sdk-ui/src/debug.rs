use std::{future::Future, panic::Location};

use tokio::time::{timeout, Duration};
use tracing::{error, warn};

#[derive(Debug, Default)]
pub(crate) struct DebugMutex<T> {
    inner: tokio::sync::Mutex<T>,
    last_locked_from: std::sync::Mutex<Option<&'static Location<'static>>>,
}

impl<T> DebugMutex<T> {
    pub fn new(value: T) -> Self {
        Self { inner: tokio::sync::Mutex::new(value), last_locked_from: Default::default() }
    }

    #[track_caller]
    pub fn lock(&self) -> impl Future<Output = tokio::sync::MutexGuard<'_, T>> {
        let caller = Location::caller();
        async move {
            let guard = match timeout(Duration::from_secs(1), self.inner.lock()).await {
                Ok(g) => g,
                Err(_) => {
                    if let Some(location) = &*self.last_locked_from.lock().unwrap() {
                        warn!("locking timed out. locked by: {location}");
                    } else {
                        error!("locking timed out. no caller info.");
                    }
                    self.inner.lock().await
                }
            };

            *self.last_locked_from.lock().unwrap() = Some(caller);
            guard
        }
    }
}
