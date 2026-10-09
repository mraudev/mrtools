use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// Root path, e.g. `C:\`.
    path: String,
    label: String,
    file_system: String,
    total: u64,
    free: u64,
    /// `fixed`, `removable`, `network` or `cdrom`.
    kind: &'static str,
}

/// All drives that are ready (empty card readers and optical drives are skipped).
/// Runs on a worker thread – disconnected network drives can take a while to answer.
#[tauri::command]
pub async fn list_drives() -> Result<Vec<Drive>, String> {
    tauri::async_runtime::spawn_blocking(imp::list)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
mod imp {
    use super::Drive;
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    pub fn list() -> Vec<Drive> {
        let mask = unsafe { GetLogicalDrives() };
        (0..26u8)
            .filter(|i| mask & (1 << i) != 0)
            .filter_map(|i| {
                let path = format!("{}:\\", (b'A' + i) as char);
                let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
                let kind = match unsafe { GetDriveTypeW(wide.as_ptr()) } {
                    DRIVE_FIXED => "fixed",
                    DRIVE_REMOVABLE => "removable",
                    DRIVE_REMOTE => "network",
                    DRIVE_CDROM => "cdrom",
                    _ => return None,
                };
                let (mut free, mut total) = (0u64, 0u64);
                // Fails for empty card readers, optical drives without a disc and the like.
                let ok = unsafe {
                    GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, std::ptr::null_mut())
                };
                if ok == 0 {
                    return None;
                }
                let mut label = [0u16; 261];
                let mut fs = [0u16; 261];
                unsafe {
                    GetVolumeInformationW(
                        wide.as_ptr(),
                        label.as_mut_ptr(),
                        label.len() as u32,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        fs.as_mut_ptr(),
                        fs.len() as u32,
                    );
                }
                Some(Drive {
                    path,
                    label: from_wide(&label),
                    file_system: from_wide(&fs),
                    total,
                    free,
                    kind,
                })
            })
            .collect()
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn list() -> Vec<super::Drive> {
        Vec::new()
    }
}
