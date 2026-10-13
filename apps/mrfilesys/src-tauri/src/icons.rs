//! The real Windows icons, as Explorer shows them. Windows hands out icons as indexes into its system
//! image list; each index is turned into a PNG once and then reused (all PDFs, all plain folders, …
//! share one). Plain files are looked up by extension only, without touching the disk.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "lowercase")]
pub enum IconSize {
    /// 32 px – shown at 16 px in the list, sharp on high-DPI screens.
    Small,
    /// 48 px – for the tiles.
    Large,
}

#[derive(Default)]
pub struct Icons {
    /// (system image list index, size) → PNG data URL, `None` if it could not be read.
    cache: Mutex<HashMap<(i32, IconSize), Option<String>>>,
}

/// Icons for `keys`: `ext:.pdf` (by extension), `dir` (a plain folder) or `path:C:\…` (programs,
/// shortcuts, icon files, folders with their own icon, drives).
#[tauri::command]
pub async fn file_icons(app: AppHandle, keys: Vec<String>, size: IconSize) -> Result<HashMap<String, Option<String>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let icons = app.state::<Icons>();
        keys.into_iter()
            .map(|key| {
                let icon = imp::index(&key).and_then(|index| {
                    let cached = icons.cache.lock().unwrap().get(&(index, size)).cloned();
                    cached.unwrap_or_else(|| {
                        let icon = imp::render(index, size);
                        icons.cache.lock().unwrap().insert((index, size), icon.clone());
                        icon
                    })
                });
                (key, icon)
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(windows)]
mod imp {
    use super::IconSize;
    use base64::Engine;
    use std::cell::Cell;
    use windows::{
        core::HSTRING,
        Win32::{
            Graphics::Gdi::{
                DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
                DIB_RGB_COLORS, HBITMAP,
            },
            Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_FLAGS_AND_ATTRIBUTES},
            System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
            UI::{
                Controls::{IImageList, ILD_TRANSPARENT},
                Shell::{
                    SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX, SHGFI_USEFILEATTRIBUTES, SHIL_EXTRALARGE,
                    SHIL_LARGE,
                },
                WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO},
            },
        },
    };

    thread_local! {
        static COM: Cell<bool> = const { Cell::new(false) };
    }

    static IMAGE_LISTS: std::sync::Mutex<bool> = std::sync::Mutex::new(false);

    /// The shell needs COM on the calling thread (worker threads are reused, so once each). The
    /// system image lists are set up once, before the first lookup – set up from several threads at
    /// the same time, lookups fail now and then.
    fn init_com() {
        COM.with(|done| {
            if !done.get() {
                let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
                done.set(true);
            }
        });
        let mut ready = IMAGE_LISTS.lock().unwrap();
        if !*ready {
            *ready = [SHIL_LARGE, SHIL_EXTRALARGE]
                .iter()
                .all(|&list| unsafe { SHGetImageList::<IImageList>(list as i32) }.is_ok());
        }
    }

    fn system_index(name: &str, attributes: Option<FILE_FLAGS_AND_ATTRIBUTES>) -> Option<i32> {
        let mut info = SHFILEINFOW::default();
        let mut flags = SHGFI_SYSICONINDEX;
        if attributes.is_some() {
            flags |= SHGFI_USEFILEATTRIBUTES;
        }
        let ok = unsafe {
            SHGetFileInfoW(
                &HSTRING::from(name),
                attributes.unwrap_or_default(),
                Some(&mut info),
                size_of::<SHFILEINFOW>() as u32,
                flags,
            )
        };
        (ok != 0).then_some(info.iIcon)
    }

    pub fn index(key: &str) -> Option<i32> {
        init_com();
        if let Some(ext) = key.strip_prefix("ext:") {
            system_index(&format!("datei{ext}"), Some(FILE_ATTRIBUTE_NORMAL))
        } else if key == "dir" {
            system_index("ordner", Some(FILE_ATTRIBUTE_DIRECTORY))
        } else {
            system_index(key.strip_prefix("path:")?, None)
        }
    }

    pub fn render(index: i32, size: IconSize) -> Option<String> {
        init_com();
        let list = match size {
            IconSize::Small => SHIL_LARGE,
            IconSize::Large => SHIL_EXTRALARGE,
        };
        unsafe {
            let images: IImageList = SHGetImageList(list as i32).ok()?;
            let icon = images.GetIcon(index, ILD_TRANSPARENT.0).ok()?;
            let png = icon_png(icon);
            let _ = DestroyIcon(icon);
            png
        }
    }

    unsafe fn icon_png(icon: HICON) -> Option<String> {
        let mut info = ICONINFO::default();
        GetIconInfo(icon, &mut info).ok()?;
        let pixels = bitmap_rgba(info.hbmColor, info.hbmMask);
        let _ = DeleteObject(info.hbmColor.into());
        let _ = DeleteObject(info.hbmMask.into());
        let (rgba, width, height) = pixels?;
        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
        }
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png_data)))
    }

    /// Reads a 32-bit icon bitmap as RGBA; uses the mask when the icon has no alpha channel.
    unsafe fn bitmap_rgba(color: HBITMAP, mask: HBITMAP) -> Option<(Vec<u8>, u32, u32)> {
        if color.is_invalid() {
            return None;
        }
        let mut bitmap = BITMAP::default();
        if GetObjectW(color.into(), size_of::<BITMAP>() as i32, Some((&mut bitmap as *mut BITMAP).cast())) == 0 {
            return None;
        }
        let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
        let read = |handle: HBITMAP| {
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut buf = vec![0u8; (width * height * 4) as usize];
            let dc = GetDC(None);
            let lines = GetDIBits(dc, handle, 0, height as u32, Some(buf.as_mut_ptr().cast()), &mut bmi, DIB_RGB_COLORS);
            ReleaseDC(None, dc);
            (lines != 0).then_some(buf)
        };
        let mut bgra = read(color)?;
        if bgra.as_chunks::<4>().0.iter().all(|p| p[3] == 0) {
            // Old-style icon: transparency comes from the AND mask (white = transparent).
            let mask = if mask.is_invalid() { None } else { read(mask) };
            for (i, p) in bgra.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let transparent = mask.as_ref().is_some_and(|m| m[i * 4] != 0);
                p[3] = if transparent { 0 } else { 255 };
            }
        }
        for p in bgra.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
        }
        Some((bgra, width as u32, height as u32))
    }
}

#[cfg(not(windows))]
mod imp {
    use super::IconSize;

    pub fn index(_key: &str) -> Option<i32> {
        None
    }

    pub fn render(_index: i32, _size: IconSize) -> Option<String> {
        None
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn files_of_one_type_share_an_icon() {
        let txt = imp::index("ext:.txt").unwrap();
        assert_eq!(imp::index("ext:.TXT"), Some(txt));
        assert_ne!(imp::index("ext:.zip"), Some(txt));
        let png = imp::render(txt, IconSize::Small).unwrap();
        assert!(png.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn reads_folder_program_and_drive_icons() {
        assert!(imp::index("dir").is_some());
        let windir = std::env::var("SystemRoot").unwrap();
        let notepad = imp::index(&format!("path:{windir}\\notepad.exe")).unwrap();
        assert!(imp::render(notepad, IconSize::Large).is_some());
        assert!(imp::index("path:C:\\").is_some());
        assert_eq!(imp::index("unbekannt"), None);
    }
}
