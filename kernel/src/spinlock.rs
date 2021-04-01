use core::{cell::UnsafeCell, ops::{Deref, DerefMut}};



pub struct Spinlock<T> {
    inner: UnsafeCell<T>,
    locked: UnsafeCell<bool>
}

impl<T> Spinlock<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            locked: UnsafeCell::new(false)
        }
    }

    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        loop {
            // Safety:
            // - The pointer is valid for reads, is properly aligned and is properly initialized.
            // - Concurrent reads are safe, as it is only one byte.
            // The read is volatile to force a read every iteration, to allow other threads to unlock the spinlock.
            while unsafe { self.locked.get().read_volatile() } {}
            // Safety:
            // - Should be safe? This doesn't affect any memory or registers.
            unsafe {
                asm!("cli"); // disable interrupts
            }
            // check condition again
            // Safety: see above
            if unsafe { self.locked.get().read_volatile() } {
                // If we got here, another thread locked the spinlock after the while loop and before cli
                unsafe { asm!("sti"); }
                continue;
            }
            // spinlock is now unlocked and can be locked by the current thread
            unsafe { self.locked.get().write(true); }
            // Safety: see cli above
            unsafe {
                asm!("sti"); // enable interrupts again
            }

            break SpinlockGuard {
                parent: self
            }
        }
    }
}

pub struct SpinlockGuard<'a, T> {
    parent: &'a Spinlock<T>
}

impl<'a, T> Deref for SpinlockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // This should never fail, as a SpinlockGuard should be unique
        unsafe { self.parent.inner.get().as_ref().unwrap() }
    }
}

impl<'a, T> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.parent.inner.get().as_mut().unwrap() }
    }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { self.parent.locked.get().write(false); }
    }
}