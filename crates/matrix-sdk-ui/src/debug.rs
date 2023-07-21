use std::{
    future::Future,
    ops::{Deref, DerefMut},
    panic::Location,
    sync::Arc,
};

use tokio::time::{timeout, Duration};
use tracing::{error, warn};

#[derive(Debug, Default)]
pub(crate) struct DebugMutex<T> {
    inner: tokio::sync::Mutex<T>,
    last_locked_from: Arc<std::sync::Mutex<Option<&'static Location<'static>>>>,
}

impl<T> DebugMutex<T> {
    pub fn new(value: T) -> Self {
        Self { inner: tokio::sync::Mutex::new(value), last_locked_from: Default::default() }
    }

    #[track_caller]
    pub fn lock(&self) -> impl Future<Output = DebugMutexGuard<'_, T>> {
        let caller = Location::caller();
        async move {
            let guard = match timeout(Duration::from_millis(50), self.inner.lock()).await {
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
            DebugMutexGuard { inner: guard, last_locked_from: self.last_locked_from.clone() }
        }
    }
}

#[derive(Debug)]
pub struct DebugMutexGuard<'a, T> {
    inner: tokio::sync::MutexGuard<'a, T>,
    last_locked_from: Arc<std::sync::Mutex<Option<&'static Location<'static>>>>,
}

impl<T> Deref for DebugMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<T> DerefMut for DebugMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl<T> Drop for DebugMutexGuard<'_, T> {
    fn drop(&mut self) {
        *self.last_locked_from.lock().unwrap() = None;
    }
}
