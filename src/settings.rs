use std::cell::RefCell;
use std::sync::Arc;

thread_local!(static CURRENT_SETTINGS: RefCell<Settings> = RefCell::new(Settings::new()));

#[derive(Clone)]
pub struct Inner {
    sort_maps: bool,
}

/// Configures how insta operates at test time.
#[derive(Clone)]
pub struct Settings {
    inner: Arc<Inner>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            inner: Arc::new(Inner { sort_maps: false }),
        }
    }
}

impl Settings {
    /// Returns the default settings.
    pub fn new() -> Settings {
        Settings::default()
    }

    /// Enables forceful sorting of maps before serialization.
    pub fn set_sort_maps(&mut self, value: bool) {
        Arc::make_mut(&mut self.inner).sort_maps = value;
    }

    /// Returns the current value for map sorting.
    pub fn sort_maps(&self) -> bool {
        self.inner.sort_maps
    }

    /// Runs a function with the current settings bound to the thread.
    pub fn run<F: FnOnce()>(&self, f: F) {
        CURRENT_SETTINGS.with(|x| {
            let old = {
                let mut current = x.borrow_mut();
                let old = current.inner.clone();
                current.inner = self.inner.clone();
                old
            };
            f();
            let mut current = x.borrow_mut();
            current.inner = old;
        })
    }

    /// Runs a function with the current settings.
    pub(crate) fn with<R, F: FnOnce(&Settings) -> R>(f: F) -> R {
        CURRENT_SETTINGS.with(|x| f(&*x.borrow()))
    }
}
