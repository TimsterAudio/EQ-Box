//! Loads `eq_box_apo.dll` the way the Windows audio engine does, runs a
//! tone through it and checks the EQ, including a live settings change.
//!
//! cargo build --release -p eq-box-apo --example apo_harness
//! then run apo_harness.exe from the folder holding eq_box_apo.dll
//! (on Linux: `wine apo_harness.exe`). It writes the engine's settings file,
//! so don't run it on a machine where the engine is in use.

#[cfg(not(windows))]
fn main() {
    eprintln!("The APO harness only runs on Windows (or Wine).");
}

#[cfg(windows)]
fn main() {
    if let Err(e) = win::run() {
        eprintln!("FAIL: {e}");
        std::process::exit(1);
    }
    println!("PASS");
}

#[cfg(windows)]
mod win {
    use eq_core::dsp::EqSettings;
    use eq_core::engine_settings;
    use std::f64::consts::PI;
    use windows::Win32::Media::Audio::Apo::*;
    use windows::Win32::Media::Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
    use windows::Win32::System::Com::{CoTaskMemFree, IClassFactory};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_core::{implement, w, Interface, Ref, Result, BOOL, GUID, HRESULT, PCSTR};

    const CLSID: GUID = GUID::from_u128(0x0febc01a_a301_491f_bbcd_17284a13cb61);
    const RATE: f32 = 48_000.0;
    const CH: u32 = 2;

    #[implement(IAudioMediaType)]
    struct FloatStereo;

    impl IAudioMediaType_Impl for FloatStereo_Impl {
        fn IsCompressedFormat(&self) -> Result<BOOL> {
            Ok(false.into())
        }
        fn IsEqual(&self, _other: Ref<IAudioMediaType>) -> Result<u32> {
            Ok(0)
        }
        fn GetAudioFormat(&self) -> *mut windows::Win32::Media::Audio::WAVEFORMATEX {
            core::ptr::null_mut()
        }
        fn GetUncompressedAudioFormat(&self, f: *mut UNCOMPRESSEDAUDIOFORMAT) -> Result<()> {
            unsafe {
                *f = UNCOMPRESSEDAUDIOFORMAT {
                    guidFormatType: KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
                    dwSamplesPerFrame: CH,
                    dwBytesPerSampleContainer: 4,
                    dwValidBitsPerSample: 32,
                    fFramesPerSecond: RATE,
                    dwChannelMask: 3,
                };
            }
            Ok(())
        }
    }

    type GetClassObject =
        unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut core::ffi::c_void) -> HRESULT;

    fn check(cond: bool, what: &str) -> std::result::Result<(), String> {
        if cond {
            println!("ok   {what}");
            Ok(())
        } else {
            Err(what.to_string())
        }
    }

    /// Gain in dB of a sine at `freq` through the APO, in 10 ms buffers.
    fn measure(rt: &IAudioProcessingObjectRT, freq: f64) -> f64 {
        let frames = 480;
        let mut phase = 0.0f64;
        let (mut sum_in, mut sum_out, mut n) = (0.0f64, 0.0f64, 0usize);
        for block in 0..100 {
            let mut buf: Vec<f32> = (0..frames)
                .flat_map(|_| {
                    phase += 2.0 * PI * freq / RATE as f64;
                    let v = (phase.sin() * 0.1) as f32;
                    [v; CH as usize]
                })
                .collect();
            let input: Vec<f32> = buf.clone();
            let mut conn = APO_CONNECTION_PROPERTY {
                pBuffer: buf.as_mut_ptr() as usize,
                u32ValidFrameCount: frames,
                u32BufferFlags: BUFFER_VALID,
                u32Signature: 0,
            };
            let in_ptr: *const APO_CONNECTION_PROPERTY = &conn;
            let mut out_ptr: *mut APO_CONNECTION_PROPERTY = &mut conn;
            unsafe { rt.APOProcess(1, &in_ptr, 1, &mut out_ptr) };
            if block >= 50 {
                sum_in += input.iter().map(|x| (*x as f64).powi(2)).sum::<f64>();
                sum_out += buf.iter().map(|x| (*x as f64).powi(2)).sum::<f64>();
                n += 1;
            }
        }
        let _ = n;
        10.0 * (sum_out / sum_in).log10()
    }

    pub fn run() -> std::result::Result<(), String> {
        // Settings first: +12 dB at band 6 (1 kHz bell). With `--from-app`,
        // EQ Box itself must already have written that (band 6 at +12 dB),
        // which checks the whole chain from the panel to the sound.
        let path = engine_settings::default_path();
        let from_app = std::env::args().any(|a| a == "--from-app");
        if !from_app {
            let mut s = EqSettings::default();
            s.bands[5].gain_db = 12.0;
            engine_settings::write(&path, &s)
                .map_err(|e| format!("write {}: {e}", path.display()))?;
        }
        println!("settings file {}", path.display());

        unsafe {
            let dll =
                LoadLibraryW(w!("eq_box_apo.dll")).map_err(|e| format!("LoadLibrary: {e}"))?;
            let proc = GetProcAddress(dll, PCSTR(c"DllGetClassObject".as_ptr() as _))
                .ok_or("no DllGetClassObject")?;
            let get: GetClassObject = core::mem::transmute(proc);
            let mut factory: Option<IClassFactory> = None;
            get(&CLSID, &IClassFactory::IID, &mut factory as *mut _ as _)
                .ok()
                .map_err(|e| format!("DllGetClassObject: {e}"))?;
            let factory = factory.ok_or("no factory")?;
            let apo: IAudioProcessingObject = factory
                .CreateInstance(None)
                .map_err(|e| format!("CreateInstance: {e}"))?;
            check(true, "created the engine through COM")?;

            let props = apo.GetRegistrationProperties().map_err(|e| e.to_string())?;
            let name = String::from_utf16_lossy(&(*props).szFriendlyName)
                .trim_end_matches('\0')
                .to_string();
            let flags = (*props).Flags.0;
            CoTaskMemFree(Some(props as _));
            check(
                name == "EQ Box" && flags & APO_FLAG_INPLACE.0 != 0,
                "registration properties",
            )?;

            apo.Initialize(&[])
                .map_err(|e| format!("Initialize: {e}"))?;
            check(apo.Initialize(&[]).is_err(), "second Initialize is refused")?;

            let fmt: IAudioMediaType = FloatStereo.into();
            let ok = apo
                .IsInputFormatSupported(None, &fmt)
                .map_err(|e| format!("IsInputFormatSupported: {e}"))?;
            check(ok.as_raw() == fmt.as_raw(), "accepts 32-bit float")?;

            let cfg: IAudioProcessingObjectConfiguration = apo.cast().map_err(|e| e.to_string())?;
            let desc = APO_CONNECTION_DESCRIPTOR {
                Type: APO_CONNECTION_BUFFER_TYPE_EXTERNAL,
                pBuffer: 0,
                u32MaxFrameCount: 480,
                pFormat: core::mem::ManuallyDrop::new(Some(fmt.clone())),
                u32Signature: 0,
            };
            let d: *const APO_CONNECTION_DESCRIPTOR = &desc;
            cfg.LockForProcess(&[d], &[d])
                .map_err(|e| format!("LockForProcess: {e}"))?;
            check(
                apo.GetInputChannelCount().map_err(|e| e.to_string())? == CH,
                "locked for 2 channels",
            )?;

            let rt: IAudioProcessingObjectRT = apo.cast().map_err(|e| e.to_string())?;
            let g = measure(&rt, 1000.0);
            println!("     1 kHz through the engine: {g:+.2} dB (expect +12)");
            check((g - 12.0).abs() < 0.2, "EQ from the settings file")?;
            let g5 = measure(&rt, 5000.0);
            println!("     5 kHz: {g5:+.2} dB (expect about +0.6)");
            check((g5 - 0.6).abs() < 0.2, "the bell's shape")?;

            if from_app {
                return Ok(());
            }

            // Live change: flat, picked up by the watcher.
            engine_settings::write(&path, &EqSettings::default()).map_err(|e| e.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(400));
            let g = measure(&rt, 1000.0);
            println!("     1 kHz after going flat: {g:+.2} dB (expect 0)");
            check(g.abs() < 0.1, "picks up settings changes live")?;

            // Silent buffers pass straight through.
            let mut buf = vec![0.5f32; 960];
            let mut conn = APO_CONNECTION_PROPERTY {
                pBuffer: buf.as_mut_ptr() as usize,
                u32ValidFrameCount: 480,
                u32BufferFlags: BUFFER_SILENT,
                u32Signature: 0,
            };
            let in_ptr: *const APO_CONNECTION_PROPERTY = &conn;
            let mut out_ptr: *mut APO_CONNECTION_PROPERTY = &mut conn;
            rt.APOProcess(1, &in_ptr, 1, &mut out_ptr);
            check(
                conn.u32BufferFlags == BUFFER_SILENT && buf[0] == 0.5,
                "leaves silent buffers alone",
            )?;

            // The engine reports what it's doing once a second.
            std::thread::sleep(std::time::Duration::from_millis(1300));
            let status = path.with_file_name("engine-status.txt");
            let text = std::fs::read_to_string(&status).unwrap_or_default();
            for line in text.lines() {
                println!("     | {line}");
            }
            check(
                text.contains("streams running: 1") && text.contains("48000 Hz, 2 ch"),
                "writes its status report",
            )?;

            cfg.UnlockForProcess()
                .map_err(|e| format!("UnlockForProcess: {e}"))?;
            check(cfg.UnlockForProcess().is_err(), "second Unlock is refused")?;
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(status);
        }
        Ok(())
    }
}
