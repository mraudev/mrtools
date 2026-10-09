//! Windows APIs that `sysinfo` does not cover. Everything returns empty results on other platforms.

use serde::Serialize;

#[derive(Default, Clone, Copy)]
pub struct NtProc {
    pub threads: u32,
    pub handles: u32,
    /// Private working set – what Task Manager shows as memory.
    pub private_ws: u64,
    /// All threads are suspended (by a debugger, mrprocs, or the system for UWP apps).
    pub suspended: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    name: String,
    path: String,
    size: u32,
    base: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    protocol: &'static str,
    local: String,
    remote: String,
    state: &'static str,
}

pub use imp::*;

#[cfg(windows)]
mod imp {
    use super::{Connection, Module, NtProc};
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use base64::Engine;
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    use windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW;
    use windows_sys::Win32::Security::{
        GetTokenInformation, LookupAccountSidW, TokenElevation, SID_NAME_USE, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W, TH32CS_SNAPMODULE,
        TH32CS_SNAPMODULE32,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetPriorityClass, OpenProcess, OpenProcessToken, SetPriorityClass, TerminateProcess,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_SUSPEND_RESUME, PROCESS_TERMINATE,
    };
    use windows_sys::Win32::UI::Shell::{
        SHGetFileInfoW, ShellExecuteExW, ShellExecuteW, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW, SHFILEINFOW,
        SHGFI_ICON, SHGFI_LARGEICON,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO, SW_SHOWNORMAL};

    #[link(name = "ntdll")]
    extern "system" {
        fn NtQuerySystemInformation(class: i32, buffer: *mut c_void, length: u32, returned: *mut u32) -> i32;
        fn NtSuspendProcess(process: HANDLE) -> i32;
        fn NtResumeProcess(process: HANDLE) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
    }

    const SYSTEM_PROCESS_INFORMATION: i32 = 5;
    const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;
    const THREAD_STATE_WAITING: u32 = 5;
    const WAIT_REASON_SUSPENDED: u32 = 5;

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *const u16,
    }

    /// `SYSTEM_PROCESS_INFORMATION` from ntdll, followed by `number_of_threads` thread records.
    #[repr(C)]
    struct SysProcess {
        next_entry_offset: u32,
        number_of_threads: u32,
        working_set_private_size: i64,
        hard_fault_count: u32,
        number_of_threads_high_watermark: u32,
        cycle_time: u64,
        create_time: i64,
        user_time: i64,
        kernel_time: i64,
        image_name: UnicodeString,
        base_priority: i32,
        unique_process_id: usize,
        inherited_from_unique_process_id: usize,
        handle_count: u32,
        session_id: u32,
        unique_process_key: usize,
        peak_virtual_size: usize,
        virtual_size: usize,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_page_count: usize,
        io_counters: [u64; 6],
    }

    /// `SYSTEM_THREAD_INFORMATION`.
    #[repr(C)]
    struct SysThread {
        kernel_time: i64,
        user_time: i64,
        create_time: i64,
        wait_time: u32,
        start_address: usize,
        client_id: [usize; 2],
        priority: i32,
        base_priority: i32,
        context_switches: u32,
        thread_state: u32,
        wait_reason: u32,
    }

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain([0]).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    fn last_error() -> String {
        std::io::Error::last_os_error().to_string()
    }

    fn nt_error(status: i32) -> String {
        std::io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32).to_string()
    }

    /// Process handle that is closed on drop.
    struct Process(HANDLE);

    impl Process {
        fn open(pid: u32, access: u32) -> Result<Self, String> {
            let handle = unsafe { OpenProcess(access, 0, pid) };
            if handle.is_null() {
                Err(last_error())
            } else {
                Ok(Self(handle))
            }
        }
    }

    impl Drop for Process {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    /// Thread and handle counts, private working set and suspension of all processes in one call.
    pub fn system_processes() -> HashMap<u32, NtProc> {
        let mut map = HashMap::new();
        // u64 elements keep the buffer 8-byte aligned for the structs.
        let mut buf: Vec<u64> = vec![0; 256 * 1024];
        loop {
            let mut needed = 0u32;
            let status = unsafe {
                NtQuerySystemInformation(
                    SYSTEM_PROCESS_INFORMATION,
                    buf.as_mut_ptr().cast(),
                    (buf.len() * 8) as u32,
                    &mut needed,
                )
            };
            if status == STATUS_INFO_LENGTH_MISMATCH {
                buf = vec![0; needed as usize / 8 + 16 * 1024];
                continue;
            }
            if status < 0 {
                return map;
            }
            break;
        }

        let base = buf.as_ptr() as *const u8;
        let mut offset = 0usize;
        loop {
            // SAFETY: ntdll filled the buffer with a chain of records that stay inside it.
            unsafe {
                let p = &*(base.add(offset) as *const SysProcess);
                let threads = std::slice::from_raw_parts(
                    base.add(offset + std::mem::size_of::<SysProcess>()) as *const SysThread,
                    p.number_of_threads as usize,
                );
                let suspended = !threads.is_empty()
                    && threads
                        .iter()
                        .all(|t| t.thread_state == THREAD_STATE_WAITING && t.wait_reason == WAIT_REASON_SUSPENDED);
                map.insert(
                    p.unique_process_id as u32,
                    NtProc {
                        threads: p.number_of_threads,
                        handles: p.handle_count,
                        private_ws: p.working_set_private_size.max(0) as u64,
                        suspended,
                    },
                );
                if p.next_entry_offset == 0 {
                    break;
                }
                offset += p.next_entry_offset as usize;
            }
        }
        map
    }

    /// "DOMAIN\user" for a SID like "S-1-5-18".
    pub fn account_name(sid: &str) -> Option<String> {
        unsafe {
            let mut psid = std::ptr::null_mut();
            if ConvertStringSidToSidW(wide(sid).as_ptr(), &mut psid) == 0 {
                return None;
            }
            let mut name = [0u16; 256];
            let mut domain = [0u16; 256];
            let (mut name_len, mut domain_len) = (name.len() as u32, domain.len() as u32);
            let mut kind: SID_NAME_USE = 0;
            let ok = LookupAccountSidW(
                std::ptr::null(),
                psid,
                name.as_mut_ptr(),
                &mut name_len,
                domain.as_mut_ptr(),
                &mut domain_len,
                &mut kind,
            );
            LocalFree(psid);
            if ok == 0 {
                return None;
            }
            let (name, domain) = (from_wide(&name), from_wide(&domain));
            Some(if domain.is_empty() { name } else { format!("{domain}\\{name}") })
        }
    }

    pub fn terminate(pid: u32) -> Result<(), String> {
        let process = Process::open(pid, PROCESS_TERMINATE)?;
        if unsafe { TerminateProcess(process.0, 1) } == 0 {
            return Err(last_error());
        }
        Ok(())
    }

    pub fn suspend(pid: u32, suspend: bool) -> Result<(), String> {
        let process = Process::open(pid, PROCESS_SUSPEND_RESUME)?;
        let status = unsafe {
            if suspend {
                NtSuspendProcess(process.0)
            } else {
                NtResumeProcess(process.0)
            }
        };
        if status < 0 {
            return Err(nt_error(status));
        }
        Ok(())
    }

    /// Priority class (`IDLE_PRIORITY_CLASS` …), 0 if it cannot be read.
    pub fn priority(pid: u32) -> u32 {
        Process::open(pid, PROCESS_QUERY_LIMITED_INFORMATION)
            .map(|p| unsafe { GetPriorityClass(p.0) })
            .unwrap_or(0)
    }

    pub fn set_priority(pid: u32, class: u32) -> Result<(), String> {
        let process = Process::open(pid, PROCESS_SET_INFORMATION)?;
        if unsafe { SetPriorityClass(process.0, class) } == 0 {
            return Err(last_error());
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

    /// Whether `pid` runs with admin rights; `None` if access is denied.
    pub fn elevated(pid: u32) -> Option<bool> {
        let process = Process::open(pid, PROCESS_QUERY_LIMITED_INFORMATION).ok()?;
        token_elevated(process.0)
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

    /// Opens the Explorer properties dialog. Must run on a thread with a message loop (the main thread).
    pub fn show_properties(path: &str) -> Result<(), String> {
        let verb = wide("properties");
        let file = wide(path);
        let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_INVOKEIDLIST;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.nShow = SW_SHOWNORMAL;
        if unsafe { ShellExecuteExW(&mut info) } == 0 {
            return Err(last_error());
        }
        Ok(())
    }

    pub fn modules(pid: u32) -> Result<Vec<Module>, String> {
        let snapshot = loop {
            let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) };
            if handle != INVALID_HANDLE_VALUE {
                break handle;
            }
            let error = std::io::Error::last_os_error();
            // ERROR_BAD_LENGTH: the module list changed while reading – try again.
            if error.raw_os_error() != Some(24) {
                return Err(error.to_string());
            }
        };
        let mut out = Vec::new();
        let mut entry: MODULEENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;
        let mut ok = unsafe { Module32FirstW(snapshot, &mut entry) };
        while ok != 0 {
            out.push(Module {
                name: from_wide(&entry.szModule),
                path: from_wide(&entry.szExePath),
                size: entry.modBaseSize,
                base: format!("0x{:X}", entry.modBaseAddr as usize),
            });
            ok = unsafe { Module32NextW(snapshot, &mut entry) };
        }
        unsafe { CloseHandle(snapshot) };
        out.sort_by_key(|m| m.name.to_lowercase());
        Ok(out)
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

    fn v4(addr: u32, port_raw: u32) -> String {
        format!("{}:{}", Ipv4Addr::from(addr.to_le_bytes()), port(port_raw))
    }

    fn v6(addr: [u8; 16], port_raw: u32) -> String {
        format!("[{}]:{}", Ipv6Addr::from(addr), port(port_raw))
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

    pub fn connections(pid: u32) -> Vec<Connection> {
        let mut out = Vec::new();
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
        for r in table::<MIB_TCPROW_OWNER_PID>(tcp(AF_INET)).into_iter().filter(|r| r.dwOwningPid == pid) {
            out.push(Connection {
                protocol: "TCP",
                local: v4(r.dwLocalAddr, r.dwLocalPort),
                remote: if r.dwState == 2 { String::new() } else { v4(r.dwRemoteAddr, r.dwRemotePort) },
                state: tcp_state(r.dwState),
            });
        }
        for r in table::<MIB_TCP6ROW_OWNER_PID>(tcp(AF_INET6)).into_iter().filter(|r| r.dwOwningPid == pid) {
            out.push(Connection {
                protocol: "TCPv6",
                local: v6(r.ucLocalAddr, r.dwLocalPort),
                remote: if r.dwState == 2 { String::new() } else { v6(r.ucRemoteAddr, r.dwRemotePort) },
                state: tcp_state(r.dwState),
            });
        }
        for r in table::<MIB_UDPROW_OWNER_PID>(udp(AF_INET)).into_iter().filter(|r| r.dwOwningPid == pid) {
            out.push(Connection { protocol: "UDP", local: v4(r.dwLocalAddr, r.dwLocalPort), remote: String::new(), state: "" });
        }
        for r in table::<MIB_UDP6ROW_OWNER_PID>(udp(AF_INET6)).into_iter().filter(|r| r.dwOwningPid == pid) {
            out.push(Connection { protocol: "UDPv6", local: v6(r.ucLocalAddr, r.dwLocalPort), remote: String::new(), state: "" });
        }
        out
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
    unsafe fn bitmap_rgba(color: windows_sys::Win32::Graphics::Gdi::HBITMAP, mask: windows_sys::Win32::Graphics::Gdi::HBITMAP) -> Option<(Vec<u8>, u32)> {
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
    use super::{Connection, Module, NtProc};
    use std::collections::HashMap;

    const UNSUPPORTED: &str = "Nur unter Windows verfügbar";

    pub fn system_processes() -> HashMap<u32, NtProc> {
        HashMap::new()
    }
    pub fn account_name(_sid: &str) -> Option<String> {
        None
    }
    pub fn terminate(_pid: u32) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn suspend(_pid: u32, _suspend: bool) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn priority(_pid: u32) -> u32 {
        0
    }
    pub fn set_priority(_pid: u32, _class: u32) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn elevated(_pid: u32) -> Option<bool> {
        None
    }
    pub fn self_elevated() -> bool {
        false
    }
    pub fn run_as_admin() -> Result<bool, String> {
        Err(UNSUPPORTED.into())
    }
    pub fn show_properties(_path: &str) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn modules(_pid: u32) -> Result<Vec<Module>, String> {
        Err(UNSUPPORTED.into())
    }
    pub fn connections(_pid: u32) -> Vec<Connection> {
        Vec::new()
    }
    pub fn icon(_path: &str) -> Option<String> {
        None
    }
}


#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn finds_own_process() {
        let procs = system_processes();
        let me = procs.get(&std::process::id()).expect("own process listed");
        assert!(me.threads >= 1);
        assert!(me.handles > 0);
        assert!(me.private_ws > 0);
        assert!(!me.suspended);
    }

    #[test]
    fn lists_modules_and_ports() {
        let pid = std::process::id();
        let modules = modules(pid).unwrap();
        assert!(modules.iter().any(|m| m.name.eq_ignore_ascii_case("ntdll.dll")));

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let local = format!("127.0.0.1:{port}");
        assert!(connections(pid).iter().any(|c| c.protocol == "TCP" && c.local == local && c.state == "Lauscht"));
    }

    #[test]
    fn suspends_resumes_and_ends() {
        let mut child = std::process::Command::new("ping").args(["-n", "30", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let pid = child.id();
        let suspended = || system_processes().get(&pid).map(|p| p.suspended);

        suspend(pid, true).unwrap();
        assert_eq!(suspended(), Some(true));
        suspend(pid, false).unwrap();
        assert_eq!(suspended(), Some(false));
        // The child inherits our priority class – on CI runners that is below normal.
        let target = if priority(pid) == 0x4000 { 0x20 } else { 0x4000 };
        set_priority(pid, target).unwrap();
        assert_eq!(priority(pid), target);

        terminate(pid).unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(1));
    }

    #[test]
    fn reads_icon_and_account() {
        let icon = icon(r"C:\Windows\explorer.exe").expect("explorer has an icon");
        assert!(icon.starts_with("data:image/png;base64,"));
        // "NT AUTHORITY\SYSTEM", localized on non-English Windows.
        assert!(account_name("S-1-5-18").is_some_and(|n| n.ends_with("\\SYSTEM")));
    }
}
