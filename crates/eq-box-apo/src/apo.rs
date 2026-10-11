//! The Audio Processing Object itself, and the class factory COM uses to
//! create it.
//!
//! Call order from the audio engine: `Initialize` once, format checks,
//! `LockForProcess` with the final format, then `APOProcess` on the
//! real-time thread for every buffer, and `UnlockForProcess` when the
//! stream stops. Nothing here may panic: a panic would take down the
//! Windows audio engine.

use crate::settings_watch;
use crate::status;
use crate::CLSID_EQ_BOX_APO;
use eq_core::dsp::Equalizer;
use eq_core::shared::Shared;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::{
    CLASS_E_NOAGGREGATION, E_INVALIDARG, E_OUTOFMEMORY, E_POINTER, HANDLE,
};
use windows::Win32::Media::Audio::Apo::*;
use windows::Win32::Media::Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
use windows::Win32::System::Com::{CoTaskMemAlloc, IClassFactory, IClassFactory_Impl};
use windows_core::{implement, Error, IUnknown, Interface, Ref, Result, BOOL, GUID};

// ---------------------------------------------------------------- factory

#[implement(IClassFactory)]
pub struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        riid: *const GUID,
        ppv: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        if ppv.is_null() || riid.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe { *ppv = core::ptr::null_mut() };
        if !outer.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let apo: IUnknown = EqBoxApo::new().into();
        unsafe { apo.query(riid, ppv).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------- engine

#[derive(Clone, Copy, PartialEq)]
struct Format {
    channels: u32,
    rate: f32,
}

struct State {
    initialized: bool,
    locked: bool,
    format: Option<Format>,
    eq: Option<Equalizer>,
}

#[implement(
    IAudioProcessingObject,
    IAudioProcessingObjectRT,
    IAudioProcessingObjectConfiguration,
    IAudioSystemEffects,
    IAudioSystemEffects2
)]
pub struct EqBoxApo {
    state: Mutex<State>,
    shared: Arc<Shared>,
}

impl EqBoxApo {
    fn new() -> Self {
        status::CREATED.fetch_add(1, Relaxed);
        EqBoxApo {
            state: Mutex::new(State {
                initialized: false,
                locked: false,
                format: None,
                eq: None,
            }),
            shared: settings_watch::shared(),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // Nothing panics while holding the lock, but never let a poisoned
        // lock stop the audio.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The format of a media type, if it's one EQ Box can run on: uncompressed
/// 32-bit float, which is what the Windows audio engine uses internally.
fn float_format(t: &IAudioMediaType) -> Result<Format> {
    let mut f = UNCOMPRESSEDAUDIOFORMAT::default();
    unsafe { t.GetUncompressedAudioFormat(&mut f)? };
    let is_float = f.guidFormatType == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
        && f.dwBytesPerSampleContainer == 4
        && f.dwValidBitsPerSample == 32;
    if !is_float || f.dwSamplesPerFrame == 0 || f.fFramesPerSecond <= 0.0 {
        status::refused(
            f.dwValidBitsPerSample,
            f.dwBytesPerSampleContainer,
            f.guidFormatType == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
        );
        return Err(APOERR_FORMAT_NOT_SUPPORTED.into());
    }
    Ok(Format {
        channels: f.dwSamplesPerFrame,
        rate: f.fFramesPerSecond,
    })
}

/// EQ Box works in place, so input and output must be the same format.
fn check_format(
    opposite: Ref<IAudioMediaType>,
    requested: Ref<IAudioMediaType>,
) -> Result<IAudioMediaType> {
    status::FORMAT_QUERIES.fetch_add(1, Relaxed);
    let req = requested.ok()?;
    let f = float_format(req)?;
    if let Some(opp) = opposite.as_ref() {
        if float_format(opp)? != f {
            return Err(APOERR_FORMAT_NOT_SUPPORTED.into());
        }
    }
    Ok(req.clone())
}

fn wide<const N: usize>(s: &str) -> [u16; N] {
    let mut out = [0u16; N];
    for (o, c) in out.iter_mut().zip(s.encode_utf16().take(N - 1)) {
        *o = c;
    }
    out
}

impl IAudioProcessingObject_Impl for EqBoxApo_Impl {
    fn Reset(&self) -> Result<()> {
        if let Some(eq) = self.state().eq.as_mut() {
            eq.reset();
        }
        Ok(())
    }

    fn GetLatency(&self) -> Result<i64> {
        Ok(0)
    }

    fn GetRegistrationProperties(&self) -> Result<*mut APO_REG_PROPERTIES> {
        let props = APO_REG_PROPERTIES {
            clsid: CLSID_EQ_BOX_APO,
            Flags: APO_FLAG(APO_FLAG_DEFAULT.0 | APO_FLAG_INPLACE.0),
            szFriendlyName: wide("EQ Box"),
            szCopyrightInfo: wide("Timster's Audio Works"),
            u32MajorVersion: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0),
            u32MinorVersion: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0),
            u32MinInputConnections: 1,
            u32MaxInputConnections: 1,
            u32MinOutputConnections: 1,
            u32MaxOutputConnections: 1,
            u32MaxInstances: u32::MAX,
            u32NumAPOInterfaces: 1,
            iidAPOInterfaceList: [IAudioProcessingObject::IID],
        };
        // The caller frees this with CoTaskMemFree.
        let p = unsafe { CoTaskMemAlloc(core::mem::size_of::<APO_REG_PROPERTIES>()) }
            as *mut APO_REG_PROPERTIES;
        if p.is_null() {
            return Err(E_OUTOFMEMORY.into());
        }
        unsafe { p.write(props) };
        Ok(p)
    }

    fn Initialize(&self, _cbdatasize: u32, _pbydata: *const u8) -> Result<()> {
        // The data is the endpoint's property stores; EQ Box doesn't need
        // them, its settings come from its own file.
        let mut st = self.state();
        if st.initialized {
            return Err(APOERR_ALREADY_INITIALIZED.into());
        }
        st.initialized = true;
        status::INITIALIZED.fetch_add(1, Relaxed);
        Ok(())
    }

    fn IsInputFormatSupported(
        &self,
        poppositeformat: Ref<IAudioMediaType>,
        prequestedinputformat: Ref<IAudioMediaType>,
    ) -> Result<IAudioMediaType> {
        check_format(poppositeformat, prequestedinputformat)
    }

    fn IsOutputFormatSupported(
        &self,
        poppositeformat: Ref<IAudioMediaType>,
        prequestedoutputformat: Ref<IAudioMediaType>,
    ) -> Result<IAudioMediaType> {
        check_format(poppositeformat, prequestedoutputformat)
    }

    fn GetInputChannelCount(&self) -> Result<u32> {
        match self.state().format {
            Some(f) => Ok(f.channels),
            None => Err(APOERR_NOT_INITIALIZED.into()),
        }
    }
}

impl IAudioProcessingObjectConfiguration_Impl for EqBoxApo_Impl {
    fn LockForProcess(
        &self,
        u32numinputconnections: u32,
        ppinputconnections: *const *const APO_CONNECTION_DESCRIPTOR,
        u32numoutputconnections: u32,
        ppoutputconnections: *const *const APO_CONNECTION_DESCRIPTOR,
    ) -> Result<()> {
        // Counted, so the status report shows if Windows can't start us.
        let result = (|| -> Result<()> {
            if u32numinputconnections != 1 || u32numoutputconnections != 1 {
                return Err(APOERR_NUM_CONNECTIONS_INVALID.into());
            }
            if ppinputconnections.is_null() || ppoutputconnections.is_null() {
                return Err(E_POINTER.into());
            }
            let (input, output) = unsafe {
                let i = *ppinputconnections;
                let o = *ppoutputconnections;
                if i.is_null() || o.is_null() {
                    return Err(E_POINTER.into());
                }
                (&*i, &*o)
            };
            let in_fmt = float_format(input.pFormat.as_ref().ok_or(Error::from(E_INVALIDARG))?)?;
            let out_fmt = float_format(output.pFormat.as_ref().ok_or(Error::from(E_INVALIDARG))?)?;
            if in_fmt != out_fmt {
                return Err(APOERR_INVALID_CONNECTION_FORMAT.into());
            }

            let mut st = self.state();
            if !st.initialized {
                return Err(APOERR_NOT_INITIALIZED.into());
            }
            if st.locked {
                return Err(APOERR_APO_LOCKED.into());
            }
            // Allocating here is fine: this isn't the real-time thread.
            st.eq = Some(Equalizer::new(in_fmt.rate as f64, in_fmt.channels as usize));
            st.format = Some(in_fmt);
            st.locked = true;
            status::LOCKED.fetch_add(1, Relaxed);
            status::RATE.store(in_fmt.rate as u32, Relaxed);
            status::CHANNELS.store(in_fmt.channels, Relaxed);
            Ok(())
        })();
        if result.is_err() {
            status::LOCK_FAILURES.fetch_add(1, Relaxed);
        }
        result
    }

    fn UnlockForProcess(&self) -> Result<()> {
        let mut st = self.state();
        if !st.locked {
            return Err(APOERR_ALREADY_UNLOCKED.into());
        }
        st.locked = false;
        status::LOCKED.fetch_sub(1, Relaxed);
        Ok(())
    }
}

impl IAudioProcessingObjectRT_Impl for EqBoxApo_Impl {
    fn APOProcess(
        &self,
        u32numinputconnections: u32,
        ppinputconnections: *const *const APO_CONNECTION_PROPERTY,
        u32numoutputconnections: u32,
        ppoutputconnections: *mut *mut APO_CONNECTION_PROPERTY,
    ) {
        // Real-time thread: no allocation, no blocking, no panics.
        if u32numinputconnections < 1
            || u32numoutputconnections < 1
            || ppinputconnections.is_null()
            || ppoutputconnections.is_null()
        {
            return;
        }
        let (input, output) = unsafe {
            let i = *ppinputconnections;
            let o = *ppoutputconnections;
            if i.is_null() || o.is_null() {
                return;
            }
            (&*i, &mut *o)
        };
        output.u32ValidFrameCount = input.u32ValidFrameCount;
        output.u32BufferFlags = input.u32BufferFlags;
        if input.u32BufferFlags != BUFFER_VALID || input.pBuffer == 0 || output.pBuffer == 0 {
            status::SILENT_BUFFERS.fetch_add(1, Relaxed);
            return;
        }
        status::BUFFERS.fetch_add(1, Relaxed);

        // Only ever contended if Windows breaks its own call rules; then
        // the audio passes through untouched rather than waiting.
        let Ok(mut st) = self.state.try_lock() else {
            return;
        };
        let Some(eq) = st.eq.as_mut() else { return };
        let n = input.u32ValidFrameCount as usize * eq.channels();
        unsafe {
            if input.pBuffer != output.pBuffer {
                core::ptr::copy(input.pBuffer as *const f32, output.pBuffer as *mut f32, n);
            }
            let buf = core::slice::from_raw_parts_mut(output.pBuffer as *mut f32, n);
            let settings = self.shared.load();
            eq.process(buf, &settings);
        }
    }

    fn CalcInputFrames(&self, u32outputframecount: u32) -> u32 {
        u32outputframecount
    }

    fn CalcOutputFrames(&self, u32inputframecount: u32) -> u32 {
        u32inputframecount
    }
}

impl IAudioSystemEffects_Impl for EqBoxApo_Impl {}

impl IAudioSystemEffects2_Impl for EqBoxApo_Impl {
    fn GetEffectsList(
        &self,
        ppeffectsids: *mut *mut GUID,
        pceffects: *mut u32,
        _event: HANDLE,
    ) -> Result<()> {
        // EQ Box doesn't expose individual effects to Windows' settings UI.
        if ppeffectsids.is_null() || pceffects.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *ppeffectsids = core::ptr::null_mut();
            *pceffects = 0;
        }
        Ok(())
    }
}
