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
    removable: bool,
}

/// All fixed and removable drives that are ready (network and optical drives are skipped).
#[tauri::command]
pub fn list_drives() -> Vec<Drive> {
    imp::list()
}

#[cfg(windows)]
mod imp {
    use super::Drive;
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;

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
                let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
                if kind != DRIVE_FIXED && kind != DRIVE_REMOVABLE {
                    return None;
                }
                let (mut free, mut total) = (0u64, 0u64);
                // Fails for empty card readers and the like – those are skipped.
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
                    removable: kind == DRIVE_REMOVABLE,
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
