//! Windows shell integration: opening files, the properties dialog, copying and deleting with
//! Explorer's own progress and conflict dialogs, and the file clipboard shared with Explorer.

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum FileOp {
    Copy,
    Move,
    /// Into the recycle bin.
    Recycle,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Clipboard {
    paths: Vec<String>,
    /// Cut instead of copied – pasting moves the files.
    cut: bool,
}

/// Opens a file with its default program (folders in Explorer).
#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    imp::shell_execute(None, &path)
}

/// Windows' "Open with" dialog.
#[tauri::command]
pub fn open_with(path: String) -> Result<(), String> {
    std::process::Command::new("rundll32.exe")
        .args(["shell32.dll,OpenAs_RunDLL", &path])
        .spawn()
        .map(drop)
        .map_err(|e| e.to_string())
}

/// Explorer's properties dialog. Runs on the main thread (sync command), which has the
/// message loop the dialog needs.
#[tauri::command]
pub fn properties(path: String) -> Result<(), String> {
    imp::shell_execute(Some("properties"), &path)
}

/// Windows Terminal in `dir`, PowerShell if it is not installed.
#[tauri::command]
pub fn open_terminal(dir: String) -> Result<(), String> {
    use std::process::Command;
    if Command::new("wt.exe").args(["-d", &dir]).spawn().is_ok() {
        return Ok(());
    }
    let mut command = Command::new("powershell.exe");
    command.arg("-NoExit").current_dir(&dir);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x10;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    command.spawn().map(drop).map_err(|e| e.to_string())
}

/// Copies, moves or deletes like Explorer (progress, conflict questions, undo).
/// `target` is the destination folder for copy and move.
#[tauri::command]
pub async fn file_op(window: tauri::WebviewWindow, op: FileOp, paths: Vec<String>, target: Option<String>) -> Result<(), String> {
    #[cfg(windows)]
    let owner = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    #[cfg(not(windows))]
    let owner = {
        let _ = window;
        0
    };
    tauri::async_runtime::spawn_blocking(move || imp::file_op(owner, op, &paths, target.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

/// Puts files on the Windows clipboard, so they can be pasted in Explorer as well.
#[tauri::command]
pub fn clipboard_set(paths: Vec<String>, cut: bool) -> Result<(), String> {
    imp::clipboard_set(&paths, cut)
}

/// Files on the Windows clipboard (empty if there are none).
#[tauri::command]
pub fn clipboard_get() -> Clipboard {
    imp::clipboard_get().unwrap_or_default()
}

#[tauri::command]
pub fn clipboard_clear() {
    imp::clipboard_clear();
}

/// Hands the files over to a native Windows drag (the left mouse button must be down), so they can be
/// dropped in Explorer, on the desktop or in other apps. The drop target decides between copy and move,
/// as when dragging in Explorer. Runs on the main thread (sync command), which OLE drag and drop needs.
#[tauri::command]
pub fn drag_out(paths: Vec<String>) -> Result<(), String> {
    #[cfg(windows)]
    return drag::drag_out(&paths).map_err(|e| e.message());
    #[cfg(not(windows))]
    {
        let _ = paths;
        Err("Nur unter Windows verfügbar".into())
    }
}

/// Explorer's way of deleting permanently, for the speed comparison in `delete.rs`.
#[cfg(all(test, windows))]
pub fn bench_delete(path: &str) {
    use windows_sys::Win32::UI::Shell::{SHFileOperationW, FOF_NOCONFIRMATION, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
    let from = double_null(&[path.to_owned()]);
    let mut info: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
    info.wFunc = FO_DELETE;
    info.pFrom = from.as_ptr();
    info.fFlags = (FOF_NOCONFIRMATION | FOF_SILENT) as _;
    unsafe { SHFileOperationW(&mut info) };
}

#[cfg(windows)]
mod drag {
    use windows::{
        core::{Result, HSTRING},
        Win32::{
            System::{
                Com::IDataObject,
                Ole::{IDropSource, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE},
            },
            UI::Shell::{
                BHID_DataObject, Common::ITEMIDLIST, ILFree, SHCreateShellItemArrayFromIDLists, SHDoDragDrop,
                SHParseDisplayName,
            },
        },
    };

    pub fn drag_out(paths: &[String]) -> Result<()> {
        let mut pidls: Vec<*const ITEMIDLIST> = Vec::new();
        let result = (|| unsafe {
            for path in paths {
                let mut pidl = std::ptr::null_mut();
                SHParseDisplayName(&HSTRING::from(path.as_str()), None, &mut pidl, 0, None)?;
                pidls.push(pidl);
            }
            // The shell's own data object: targets get the formats and drag image they know from Explorer.
            let data: IDataObject = SHCreateShellItemArrayFromIDLists(&pidls)?.BindToHandler(None, &BHID_DataObject)?;
            // Never deletes anything itself: for a move the target moves the files (optimized move).
            SHDoDragDrop(None, &data, None::<&IDropSource>, DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK)
        })();
        for pidl in pidls {
            unsafe { ILFree(Some(pidl)) };
        }
        result.map(drop)
    }
}

/// `a\0b\0\0` – the list format of SHFileOperation and CF_HDROP.
#[cfg_attr(not(windows), allow(dead_code))]
fn double_null(paths: &[String]) -> Vec<u16> {
    let mut out: Vec<u16> = paths.iter().flat_map(|p| p.encode_utf16().chain([0])).collect();
    out.push(0);
    out
}

#[cfg(windows)]
mod imp {
    use super::{double_null, Clipboard, FileOp};
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{
        Foundation::{GlobalFree, HWND},
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
                RegisterClipboardFormatW, SetClipboardData,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT},
            Ole::CF_HDROP,
        },
        UI::{
            Shell::{
                DragQueryFileW, SHFileOperationW, ShellExecuteExW, DROPFILES, FOF_ALLOWUNDO, FOF_NOCONFIRMATION,
                FOF_RENAMEONCOLLISION, FOF_WANTNUKEWARNING, FO_COPY, FO_DELETE, FO_MOVE, SEE_MASK_INVOKEIDLIST,
                SHELLEXECUTEINFOW, SHFILEOPSTRUCTW,
            },
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };

    const DROPEFFECT_COPY: u32 = 1;
    const DROPEFFECT_MOVE: u32 = 2;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    fn last_error() -> String {
        std::io::Error::last_os_error().to_string()
    }

    pub fn shell_execute(verb: Option<&str>, path: &str) -> Result<(), String> {
        let verb = verb.map(wide);
        let file = wide(path);
        let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_INVOKEIDLIST;
        info.lpVerb = verb.as_ref().map_or(null(), |v| v.as_ptr());
        info.lpFile = file.as_ptr();
        info.nShow = SW_SHOWNORMAL;
        if unsafe { ShellExecuteExW(&mut info) } == 0 {
            return Err(last_error());
        }
        Ok(())
    }

    pub fn file_op(owner: isize, op: FileOp, paths: &[String], target: Option<&str>) -> Result<(), String> {
        let from = double_null(paths);
        let to = target.map(|t| double_null(&[t.to_owned()]));
        let (func, flags) = match op {
            // Pasting into the same folder makes "x - Kopie" instead of asking.
            FileOp::Copy => (FO_COPY, FOF_ALLOWUNDO | FOF_RENAMEONCOLLISION),
            FileOp::Move => (FO_MOVE, FOF_ALLOWUNDO),
            FileOp::Recycle => (FO_DELETE, FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_WANTNUKEWARNING),
        };
        let mut info: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
        info.hwnd = owner as HWND;
        info.wFunc = func;
        info.pFrom = from.as_ptr();
        info.pTo = to.as_ref().map_or(null(), |t| t.as_ptr());
        info.fFlags = flags as _;
        match unsafe { SHFileOperationW(&mut info) } {
            0 => Ok(()),
            // Cancelled by the user – not an error.
            0x4C7 => Ok(()),
            code => Err(format!("Vorgang fehlgeschlagen (Code {code:#x})")),
        }
    }

    struct OpenClipboardGuard;

    impl OpenClipboardGuard {
        fn open() -> Result<Self, String> {
            if unsafe { OpenClipboard(null_mut()) } == 0 {
                return Err(format!("Zwischenablage nicht verfügbar: {}", last_error()));
            }
            Ok(Self)
        }
    }

    impl Drop for OpenClipboardGuard {
        fn drop(&mut self) {
            unsafe { CloseClipboard() };
        }
    }

    fn drop_effect_format() -> u32 {
        unsafe { RegisterClipboardFormatW(wide("Preferred DropEffect").as_ptr()) }
    }

    /// Copies `bytes` into movable global memory, as the clipboard wants it.
    unsafe fn global(bytes: &[u8]) -> Result<*mut core::ffi::c_void, String> {
        let handle = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len());
        if handle.is_null() {
            return Err(last_error());
        }
        let ptr = GlobalLock(handle) as *mut u8;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        GlobalUnlock(handle);
        Ok(handle)
    }

    pub fn clipboard_set(paths: &[String], cut: bool) -> Result<(), String> {
        let header = DROPFILES {
            pFiles: size_of::<DROPFILES>() as u32,
            pt: unsafe { std::mem::zeroed() },
            fNC: 0,
            fWide: 1,
        };
        let mut drop_files = unsafe {
            std::slice::from_raw_parts(&header as *const DROPFILES as *const u8, size_of::<DROPFILES>()).to_vec()
        };
        drop_files.extend(double_null(paths).iter().flat_map(|c| c.to_le_bytes()));
        let effect = if cut { DROPEFFECT_MOVE } else { DROPEFFECT_COPY };

        let _clipboard = OpenClipboardGuard::open()?;
        unsafe {
            EmptyClipboard();
            for (format, bytes) in [(CF_HDROP as u32, drop_files), (drop_effect_format(), effect.to_le_bytes().to_vec())] {
                let handle = global(&bytes)?;
                // On success the clipboard owns the memory.
                if SetClipboardData(format, handle).is_null() {
                    GlobalFree(handle);
                    return Err(last_error());
                }
            }
        }
        Ok(())
    }

    pub fn clipboard_get() -> Option<Clipboard> {
        if unsafe { IsClipboardFormatAvailable(CF_HDROP as u32) } == 0 {
            return None;
        }
        let _clipboard = OpenClipboardGuard::open().ok()?;
        unsafe {
            let drop = GetClipboardData(CF_HDROP as u32);
            if drop.is_null() {
                return None;
            }
            let count = DragQueryFileW(drop, u32::MAX, null_mut(), 0);
            let paths = (0..count)
                .map(|i| {
                    let len = DragQueryFileW(drop, i, null_mut(), 0);
                    let mut buf = vec![0u16; len as usize + 1];
                    DragQueryFileW(drop, i, buf.as_mut_ptr(), buf.len() as u32);
                    String::from_utf16_lossy(&buf[..len as usize])
                })
                .collect();

            let effect = GetClipboardData(drop_effect_format());
            let cut = !effect.is_null() && {
                let ptr = GlobalLock(effect) as *const u32;
                let value = if ptr.is_null() { 0 } else { ptr.read_unaligned() };
                GlobalUnlock(effect);
                value & DROPEFFECT_MOVE != 0
            };
            Some(Clipboard { paths, cut })
        }
    }

    pub fn clipboard_clear() {
        if let Ok(_clipboard) = OpenClipboardGuard::open() {
            unsafe { EmptyClipboard() };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Clipboard, FileOp};

    const UNSUPPORTED: &str = "Nur unter Windows verfügbar";

    pub fn shell_execute(_verb: Option<&str>, _path: &str) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    pub fn file_op(_owner: isize, _op: FileOp, _paths: &[String], _target: Option<&str>) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    pub fn clipboard_set(_paths: &[String], _cut: bool) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    pub fn clipboard_get() -> Option<Clipboard> {
        None
    }

    pub fn clipboard_clear() {}
}

#[cfg(test)]
mod tests {
    use super::double_null;

    #[test]
    fn double_null_terminates() {
        let list = double_null(&["a".into(), "bc".into()]);
        assert_eq!(list, [97, 0, 98, 99, 0, 0]);
    }
}
