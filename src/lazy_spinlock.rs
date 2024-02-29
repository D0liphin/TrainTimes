use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicI8, Ordering},
};

use esp_println::println;

const UNINIT: i8 = 0;
const INIT: i8 = 1;
const LOCKED: i8 = 2;

pub struct LazySpinlock<T, F> {
    value: UnsafeCell<MaybeUninit<T>>,
    init: F,
    state: AtomicI8,
}

unsafe impl<T, F> Sync for LazySpinlock<T, F> {}

impl<T, F> LazySpinlock<T, F>
where
    F: FnOnce() -> T + Copy,
{
    pub const fn uninit(init: F) -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            init,
            state: AtomicI8::new(UNINIT),
        }
    }

    /// Initialize if not yet init
    pub fn initialize(&self) {
        println!("initialize()");
        loop {
            println!("loop...");
            if self.state.load(Ordering::Relaxed) == INIT {
                return;
            }

            // This isn't likely to happen anyway
            while self.state.load(Ordering::Relaxed) == LOCKED {
                println!("looping some more");
            }

            let result =
                self.state
                    .compare_exchange(UNINIT, LOCKED, Ordering::Relaxed, Ordering::Relaxed);
            // If we fail to lock, someone else is initializing, so let's
            // just wait for them to do it
            if result.is_err() {
                continue;
            }

            unsafe {
                *self.value.get() = MaybeUninit::new((self.init)());
            }

            self.state.store(INIT, Ordering::Relaxed);
        }
    }

    pub fn is_init(&self) -> bool {
        self.state.load(Ordering::Relaxed) == INIT
    }

    pub fn get(&self) -> &T {
        self.initialize();
        // SAFETY: we run `self.initialize()` first, so it's going to have
        // to initialize!
        unsafe { (*self.value.get()).assume_init_ref() }
    }
}
