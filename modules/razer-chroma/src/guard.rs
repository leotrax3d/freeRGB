// SPDX-License-Identifier: GPL-2.0-or-later
//
// Port of OpenRGB Controllers/RazerController/RazerDeviceGuard.cpp: the
// machine-wide "Global\RazerLinkReadWriteGuardMutex" that Razer software
// holds around device writes. Copyright (C) OpenRGB contributors.
// Modified for freeRGB: RAII guard in Rust; an unavailable mutex (e.g. no
// permission for the Global namespace) degrades to no cross-vendor guard,
// as upstream does when CreateMutex fails.

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows_sys::Win32::Security::{
        InitializeSecurityDescriptor, SetSecurityDescriptorDacl, SECURITY_ATTRIBUTES,
        SECURITY_DESCRIPTOR,
    };
    use windows_sys::Win32::System::Threading::{
        CreateMutexA, ReleaseMutex, WaitForSingleObject, INFINITE,
    };

    pub struct DeviceGuard(HANDLE);

    impl Default for DeviceGuard {
        fn default() -> Self {
            Self::new()
        }
    }

    // SAFETY: a mutex handle may be used from any thread of this process.
    #[allow(unsafe_code)]
    unsafe impl Send for DeviceGuard {}

    impl DeviceGuard {
        #[allow(unsafe_code)]
        pub fn new() -> Self {
            // SAFETY: plain Win32 calls on stack-owned, initialised structures;
            // a null DACL grants access to Razer's own services, as upstream.
            unsafe {
                let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
                InitializeSecurityDescriptor(&mut sd as *mut _ as *mut _, 1);
                SetSecurityDescriptorDacl(&mut sd as *mut _ as *mut _, 1, std::ptr::null(), 0);
                let sa = SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: &mut sd as *mut _ as *mut _,
                    bInheritHandle: 0,
                };
                Self(CreateMutexA(
                    &sa,
                    0,
                    c"Global\\RazerLinkReadWriteGuardMutex".as_ptr() as _,
                ))
            }
        }

        #[allow(unsafe_code)]
        pub fn lock(&self) -> Lock<'_> {
            if self.0.is_null() {
                return Lock(None);
            }
            loop {
                // SAFETY: valid handle owned by self.
                let result = unsafe { WaitForSingleObject(self.0, INFINITE) };
                if result == WAIT_OBJECT_0 || result == WAIT_ABANDONED {
                    // An abandoned mutex is now owned by us (upstream releases
                    // and retries; owning it is equivalent and cannot spin).
                    return Lock(Some(self));
                }
            }
        }
    }

    pub struct Lock<'a>(Option<&'a DeviceGuard>);

    impl Drop for Lock<'_> {
        #[allow(unsafe_code)]
        fn drop(&mut self) {
            if let Some(guard) = self.0 {
                // SAFETY: this thread owns the mutex (WaitForSingleObject).
                unsafe { ReleaseMutex(guard.0) };
            }
        }
    }

    impl Drop for DeviceGuard {
        #[allow(unsafe_code)]
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: handle created in new() and not used afterwards.
                unsafe { CloseHandle(self.0) };
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    /// Upstream's guard is Windows-only as well.
    #[derive(Default)]
    pub struct DeviceGuard;
    pub struct Lock;
    impl DeviceGuard {
        pub fn new() -> Self {
            Self
        }
        pub fn lock(&self) -> Lock {
            Lock
        }
    }
}

pub use imp::DeviceGuard;
