//! EQ Box's system-wide engine for Windows.
//!
//! Windows loads this DLL into its audio engine (`audiodg.exe`) for each
//! output it's installed on, and runs every app's sound through it. It's an
//! Audio Processing Object (APO): a COM object the engine creates through
//! `DllGetClassObject`, then calls on its real-time thread with each buffer.
//!
//! It runs the same `Equalizer` as the app. Settings come from the file the
//! EQ Box window writes (see `eq_core::engine_settings`), picked up by a
//! background thread and handed to the audio thread through atomics.
//!
//! On other systems this crate builds to nothing.

#![cfg(windows)]

mod apo;
mod events;
mod settings_watch;
mod status;

use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_POINTER, S_FALSE};
use windows::Win32::System::Com::IClassFactory;
use windows_core::{Interface, GUID, HRESULT};

/// EQ Box's engine CLSID. Never change it: installed systems point at it.
pub const CLSID_EQ_BOX_APO: GUID = GUID::from_u128(0x0febc01a_a301_491f_bbcd_17284a13cb61);

/// COM entry point: hands Windows the factory that makes the engine.
///
/// # Safety
/// Called by COM with valid pointers.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    if *rclsid != CLSID_EQ_BOX_APO {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    static LOGGED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    events::log_limited(
        &LOGGED,
        1,
        events::Level::Info,
        &format!("loaded by {exe} (version {})", env!("CARGO_PKG_VERSION")),
    );
    let factory: IClassFactory = apo::Factory.into();
    factory.query(riid, ppv)
}

/// The settings watcher thread lives as long as the process, so the DLL
/// never asks to be unloaded.
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}
