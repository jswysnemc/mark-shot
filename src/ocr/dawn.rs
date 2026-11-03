// libwebgpu_dawn.so (12 MB) is only needed for GPU. Binary doesn't link to it directly, but
// ORT calls Dawn through stubs, and load() loads the library when GPU is used.
// If missing, the daemon downloads it like models, from the same pyke release, with hash checks.

use std::path::{Path, PathBuf};

pub const LIBRARY: &str = "libwebgpu_dawn.so";

pub fn candidates() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        dirs.push(exe_dir.clone());
        dirs.push(exe_dir.join("../lib/mark-shot"));
    }
    dirs.extend(download_dir());
    if let Some(dist) = option_env!("MARK_SHOT_DAWN_DIST") {
        dirs.push(PathBuf::from(dist));
    }
    dirs.into_iter().map(|dir| dir.join(LIBRARY)).collect()
}

pub fn find() -> Option<PathBuf> {
    candidates().into_iter().find(|path| path.is_file())
}

/// ORT the binary is linked with; a library from another version would not match its calls.
pub const ORT: &str = "1.28.0";

/// `~/.local/share/mark-shot/gpu/onnxruntime-<ORT>`, next to `models`
pub fn download_dir() -> Option<PathBuf> {
    Some(
        dirs::data_dir()?
            .join("mark-shot")
            .join("gpu")
            .join(format!("onnxruntime-{ORT}")),
    )
}

#[cfg(not(feature = "ocr-gpu"))]
pub fn load() -> Result<(), String> {
    Err("this binary is built without the GPU engine".into())
}

#[cfg(not(feature = "ocr-gpu"))]
pub fn ensure() -> Result<(), String> {
    load()
}

#[cfg(all(feature = "ocr-gpu", not(target_arch = "x86_64")))]
pub fn load() -> Result<(), String> {
    Ok(())
}

#[cfg(all(feature = "ocr-gpu", not(target_arch = "x86_64")))]
pub fn ensure() -> Result<(), String> {
    Ok(())
}

#[cfg(all(feature = "ocr-gpu", target_arch = "x86_64"))]
pub use trampolines::load;

/// Finds the library, downloading it first when it is missing, then loads it.
#[cfg(all(feature = "ocr-gpu", target_arch = "x86_64"))]
pub fn ensure() -> Result<(), String> {
    if find().is_none() {
        fetch::download()?;
    }
    load()
}

#[cfg(all(feature = "ocr-gpu", target_arch = "x86_64"))]
mod fetch {
    use log::info;
    use std::fs::{self, File};
    use std::io::{self, BufReader, Read, Write};
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    use super::LIBRARY;
    use crate::ocr::download;
    use crate::ocr::models::Asset;

    pub const URL: &str =
        "https://cdn.pyke.io/0/pyke:ort-rs/ms@1.28.0/x86_64-unknown-linux-gnu+webgpu.tar.lzma2";
    pub const ARCHIVE: Asset = Asset {
        file: "x86_64-unknown-linux-gnu+webgpu.tar.lzma2",
        size: 13_739_496,
        sha256: "68406bc32de516ee8baeaa4c5f2de2bb0031269f19ce8b76d64988c0498b93be",
    };
    pub const LIBRARY_SIZE: u64 = 12_129_088;
    pub const LIBRARY_SHA256: &str =
        "e07cc47ed362fbc24d1aa305a39419ef025332b00973581bdbcbd66e7621a44a";

    pub fn download() -> Result<(), String> {
        let dir =
            super::download_dir().ok_or("no data directory to download the GPU library into")?;
        fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        info!(
            "ocr: downloading {LIBRARY} ({} MB) from {URL}",
            ARCHIVE.size / 1_000_000
        );
        let started = Instant::now();
        let never = AtomicBool::new(false);
        download::fetch(
            &download::agent(),
            URL,
            &ARCHIVE,
            &dir,
            &never,
            &never,
            |_| {},
        )
        .map_err(|e| format!("cannot download {LIBRARY}: {e}"))?;
        let archive = dir.join(ARCHIVE.file);
        let extracted = extract(&archive, &dir);
        let _ = fs::remove_file(&archive);
        extracted?;
        remove_other_versions(&dir);
        info!(
            "ocr: {LIBRARY} is ready in {:.1} s, {}",
            started.elapsed().as_secs_f32(),
            dir.display()
        );
        Ok(())
    }

    /// Takes only the library out of the (already verified) archive.
    pub fn extract(archive: &Path, dir: &Path) -> Result<(), String> {
        let broken = |e: io::Error| format!("cannot unpack {}: {e}", archive.display());
        let file = File::open(archive).map_err(broken)?;
        let mut tar = lzma_rust2::Lzma2Reader::new(BufReader::new(file), 1 << 26, None);
        loop {
            let mut header = [0u8; 512];
            tar.read_exact(&mut header).map_err(broken)?;
            if header.iter().all(|&b| b == 0) {
                return Err(format!("{LIBRARY} is not in {}", archive.display()));
            }
            let name = field(&header[..100]);
            let size = u64::from_str_radix(field(&header[124..136]).trim(), 8)
                .map_err(|_| format!("damaged entry in {}", archive.display()))?;
            if header[156] != b'0' || name.rsplit('/').next() != Some(LIBRARY) {
                io::copy(
                    &mut (&mut tar).take(size.next_multiple_of(512)),
                    &mut io::sink(),
                )
                .map_err(broken)?;
                continue;
            }
            if size != LIBRARY_SIZE {
                return Err(format!("{LIBRARY} in the archive has an unexpected size"));
            }
            let part = dir.join(format!("{LIBRARY}.part"));
            let written = write_checked(&mut (&mut tar).take(size), &part);
            if let Err(e) = written {
                let _ = fs::remove_file(&part);
                return Err(e);
            }
            return fs::rename(&part, dir.join(LIBRARY)).map_err(|e| e.to_string());
        }
    }

    fn write_checked(from: &mut impl Read, part: &Path) -> Result<(), String> {
        let mut out =
            File::create(part).map_err(|e| format!("cannot write {}: {e}", part.display()))?;
        let mut hash = hmac_sha256::Hash::new();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = from.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
        if hex(&hash.finalize()) != LIBRARY_SHA256 {
            return Err(format!("{LIBRARY} failed the checksum"));
        }
        out.sync_all().map_err(|e| e.to_string())
    }

    pub fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn field(bytes: &[u8]) -> &str {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..end]).unwrap_or_default()
    }

    fn remove_other_versions(current: &Path) {
        let Some(parent) = current.parent() else {
            return;
        };
        for entry in fs::read_dir(parent)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
        {
            if entry.path() != current {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn pinned_download_is_the_library_this_binary_was_linked_with() {
            let Some(dist) = option_env!("MARK_SHOT_DAWN_DIST") else {
                return;
            };
            let dist = Path::new(dist);
            assert_eq!(
                dist.file_name().and_then(|name| name.to_str()),
                Some(ARCHIVE.sha256),
                "ort was updated: take URL, size and sha256 from ort-sys build/download/dist.tsv"
            );
            assert!(URL.contains(&format!("ms@{}", super::super::ORT)));
            let bytes = fs::read(dist.join(LIBRARY)).unwrap();
            assert_eq!(bytes.len() as u64, LIBRARY_SIZE);
            assert_eq!(hex(&hmac_sha256::Hash::hash(&bytes)), LIBRARY_SHA256);
        }
    }
}

#[cfg(all(feature = "ocr-gpu", target_arch = "x86_64"))]
mod trampolines {
    use log::{error, info};
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt;
    use std::sync::Mutex;

    use nix::libc;

    include!(concat!(env!("OUT_DIR"), "/dawn_names.rs"));
    std::arch::global_asm!(include_str!(concat!(env!("OUT_DIR"), "/dawn.s")));

    unsafe extern "C" {
        static mut mark_shot_dawn_slots: [usize; NAMES.len()];
    }

    #[unsafe(no_mangle)]
    extern "C" fn mark_shot_dawn_missing() -> ! {
        error!("ocr: WebGPU was used before {} was loaded", super::LIBRARY);
        std::process::abort()
    }

    pub fn load() -> Result<(), String> {
        static LOADED: Mutex<bool> = Mutex::new(false);
        let mut loaded = LOADED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !*loaded {
            open()?;
            *loaded = true;
        }
        Ok(())
    }

    fn open() -> Result<(), String> {
        let path =
            super::find().ok_or_else(|| format!("{} is not downloaded yet", super::LIBRARY))?;
        let name = CString::new(path.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        if handle.is_null() {
            let reason = unsafe { libc::dlerror() };
            let reason = if reason.is_null() {
                "unknown error".into()
            } else {
                unsafe { CStr::from_ptr(reason) }.to_string_lossy()
            };
            return Err(format!("cannot load {}: {reason}", path.display()));
        }
        let mut found = Vec::with_capacity(NAMES.len());
        for symbol in NAMES {
            let address = unsafe { libc::dlsym(handle, symbol.as_ptr()) };
            if address.is_null() {
                return Err(format!(
                    "{} has no {}, wrong version",
                    path.display(),
                    symbol.to_string_lossy()
                ));
            }
            found.push(address as usize);
        }
        let slots = (&raw mut mark_shot_dawn_slots).cast::<usize>();
        for (i, address) in found.into_iter().enumerate() {
            unsafe { slots.add(i).write(address) };
        }
        info!("ocr: loaded {}", path.display());
        Ok(())
    }
}
