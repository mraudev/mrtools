//! Windows API: the TCP and UDP tables with owning process, ending processes, elevation and exe icons.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Socket {
    /// "TCP" or "UDP".
    pub protocol: &'static str,
    pub ipv6: bool,
    pub local_address: String,
    pub local_port: u16,
    /// Empty for listening sockets and UDP.
    pub remote_address: String,
    pub remote_port: u16,
    /// German TCP state ("Lauscht", "Verbunden", …), empty for UDP.
    pub state: &'static str,
    pub pid: u32,
}

pub use imp::*;

#[cfg(windows)]
mod imp {
    use super::Socket;
    use std::ffi::c_void;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use base64::Engine;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, TerminateProcess, PROCESS_TERMINATE,
    };
    use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, ShellExecuteW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO, SW_SHOWNORMAL};

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain([0]).collect()
    }

    fn last_error() -> String {
        std::io::Error::last_os_error().to_string()
    }

    fn tcp_state(state: u32) -> &'static str {
        match state {
            1 => "Geschlossen",
            2 => "Lauscht",
            3 => "SYN gesendet",
            4 => "SYN empfangen",
            5 => "Verbunden",
            6 => "FIN-Wait 1",
            7 => "FIN-Wait 2",
            8 => "Close-Wait",
            9 => "Closing",
            10 => "Last-ACK",
            11 => "Time-Wait",
            12 => "Wird gelöscht",
            _ => "",
        }
    }

    fn port(raw: u32) -> u16 {
        u16::from_be(raw as u16)
    }

    fn v4(addr: u32) -> String {
        Ipv4Addr::from(addr.to_le_bytes()).to_string()
    }

    fn v6(addr: [u8; 16]) -> String {
        Ipv6Addr::from(addr).to_string()
    }

    /// Reads an IP helper table: a u32 row count followed by the rows.
    fn table<T: Copy>(fetch: impl Fn(*mut c_void, &mut u32) -> u32) -> Vec<T> {
        let mut size = 0u32;
        fetch(std::ptr::null_mut(), &mut size);
        for _ in 0..4 {
            // Rows contain only u32 and byte arrays, so u32 alignment is enough.
            let mut buf: Vec<u32> = vec![0; size as usize / 4 + 64];
            size = (buf.len() * 4) as u32;
            if fetch(buf.as_mut_ptr().cast(), &mut size) == 0 {
                let count = buf[0] as usize;
                // SAFETY: the API wrote `count` rows directly after the count.
                return unsafe { std::slice::from_raw_parts(buf.as_ptr().add(1) as *const T, count).to_vec() };
            }
        }
        Vec::new()
    }

    /// All TCP and UDP sockets of all processes. Needs no admin rights.
    pub fn sockets() -> Vec<Socket> {
        let tcp = |af: u16| {
            move |buf: *mut c_void, size: &mut u32| unsafe {
                GetExtendedTcpTable(buf, size, 0, af as u32, TCP_TABLE_OWNER_PID_ALL, 0)
            }
        };
        let udp = |af: u16| {
            move |buf: *mut c_void, size: &mut u32| unsafe {
                GetExtendedUdpTable(buf, size, 0, af as u32, UDP_TABLE_OWNER_PID, 0)
            }
        };
        let mut out = Vec::new();
        for r in table::<MIB_TCPROW_OWNER_PID>(tcp(AF_INET)) {
            let listening = r.dwState == 2;
            out.push(Socket {
                protocol: "TCP",
                ipv6: false,
                local_address: v4(r.dwLocalAddr),
                local_port: port(r.dwLocalPort),
                remote_address: if listening { String::new() } else { v4(r.dwRemoteAddr) },
                remote_port: if listening { 0 } else { port(r.dwRemotePort) },
                state: tcp_state(r.dwState),
                pid: r.dwOwningPid,
            });
        }
        for r in table::<MIB_TCP6ROW_OWNER_PID>(tcp(AF_INET6)) {
            let listening = r.dwState == 2;
            out.push(Socket {
                protocol: "TCP",
                ipv6: true,
                local_address: v6(r.ucLocalAddr),
                local_port: port(r.dwLocalPort),
                remote_address: if listening { String::new() } else { v6(r.ucRemoteAddr) },
                remote_port: if listening { 0 } else { port(r.dwRemotePort) },
                state: tcp_state(r.dwState),
                pid: r.dwOwningPid,
            });
        }
        for r in table::<MIB_UDPROW_OWNER_PID>(udp(AF_INET)) {
            out.push(Socket {
                protocol: "UDP",
                ipv6: false,
                local_address: v4(r.dwLocalAddr),
                local_port: port(r.dwLocalPort),
                remote_address: String::new(),
                remote_port: 0,
                state: "",
                pid: r.dwOwningPid,
            });
        }
        for r in table::<MIB_UDP6ROW_OWNER_PID>(udp(AF_INET6)) {
            out.push(Socket {
                protocol: "UDP",
                ipv6: true,
                local_address: v6(r.ucLocalAddr),
                local_port: port(r.dwLocalPort),
                remote_address: String::new(),
                remote_port: 0,
                state: "",
                pid: r.dwOwningPid,
            });
        }
        out
    }

    pub fn terminate(pid: u32) -> Result<(), String> {
        unsafe {
            let process = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if process.is_null() {
                return Err(last_error());
            }
            let ok = TerminateProcess(process, 1);
            CloseHandle(process);
            if ok == 0 {
                return Err(last_error());
            }
        }
        Ok(())
    }

    fn token_elevated(process: HANDLE) -> Option<bool> {
        unsafe {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
                return None;
            }
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut len = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut len,
            );
            CloseHandle(token);
            (ok != 0).then_some(elevation.TokenIsElevated != 0)
        }
    }

    pub fn self_elevated() -> bool {
        token_elevated(unsafe { GetCurrentProcess() }).unwrap_or(false)
    }

    /// Starts this exe again with the UAC prompt. Returns false if the user declined.
    pub fn run_as_admin() -> Result<bool, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                wide("runas").as_ptr(),
                wide(&exe.to_string_lossy()).as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        Ok(result as isize > 32)
    }

    /// The exe's 32 px icon as a PNG data URL.
    pub fn icon(path: &str) -> Option<String> {
        if path.is_empty() || !Path::new(path).is_file() {
            return None;
        }
        unsafe {
            let mut info: SHFILEINFOW = std::mem::zeroed();
            let ok = SHGetFileInfoW(
                wide(path).as_ptr(),
                0,
                &mut info,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if ok == 0 || info.hIcon.is_null() {
                return None;
            }
            let mut icon_info: ICONINFO = std::mem::zeroed();
            let got = GetIconInfo(info.hIcon, &mut icon_info) != 0;
            DestroyIcon(info.hIcon);
            if !got {
                return None;
            }
            let pixels = bitmap_rgba(icon_info.hbmColor, icon_info.hbmMask);
            DeleteObject(icon_info.hbmColor);
            DeleteObject(icon_info.hbmMask);
            let (rgba, size) = pixels?;

            let mut png_data = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut png_data, size, size);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header().ok()?;
                writer.write_image_data(&rgba).ok()?;
            }
            Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png_data)))
        }
    }

    /// Reads a square 32-bit icon bitmap as RGBA; uses the mask when the icon has no alpha channel.
    unsafe fn bitmap_rgba(
        color: windows_sys::Win32::Graphics::Gdi::HBITMAP,
        mask: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    ) -> Option<(Vec<u8>, u32)> {
        if color.is_null() {
            return None;
        }
        const SIZE: i32 = 32;
        let read = |bitmap| {
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SIZE,
                biHeight: -SIZE, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut buf = vec![0u8; (SIZE * SIZE * 4) as usize];
            let dc = GetDC(std::ptr::null_mut());
            let lines = GetDIBits(dc, bitmap, 0, SIZE as u32, buf.as_mut_ptr().cast(), &mut bmi, DIB_RGB_COLORS);
            ReleaseDC(std::ptr::null_mut(), dc);
            (lines != 0).then_some(buf)
        };
        let mut bgra = read(color)?;
        if bgra.as_chunks::<4>().0.iter().all(|p| p[3] == 0) {
            // Old-style icon: transparency comes from the AND mask (white = transparent).
            let mask = if mask.is_null() { None } else { read(mask) };
            for (i, p) in bgra.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let transparent = mask.as_ref().is_some_and(|m| m[i * 4] != 0);
                p[3] = if transparent { 0 } else { 255 };
            }
        }
        for p in bgra.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
        }
        Some((bgra, SIZE as u32))
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Socket;

    pub fn sockets() -> Vec<Socket> {
        Vec::new()
    }
    pub fn terminate(_pid: u32) -> Result<(), String> {
        Err("Nur unter Windows verfügbar".into())
    }
    pub fn self_elevated() -> bool {
        false
    }
    pub fn run_as_admin() -> Result<bool, String> {
        Ok(false)
    }
    pub fn icon(_path: &str) -> Option<String> {
        None
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn finds_own_listening_socket() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let me = std::process::id();
        let socket = sockets()
            .into_iter()
            .find(|s| s.pid == me && s.local_port == port)
            .expect("own socket listed");
        assert_eq!((socket.protocol, socket.state, socket.local_address.as_str()), ("TCP", "Lauscht", "127.0.0.1"));
    }

    #[test]
    fn ends_a_process() {
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        terminate(child.id()).unwrap();
        assert!(!child.wait().unwrap().success());
    }

    #[test]
    fn reads_an_icon() {
        let exe = std::env::var("SystemRoot").unwrap() + "\\notepad.exe";
        assert!(icon(&exe).is_some_and(|i| i.starts_with("data:image/png;base64,")));
    }
}
