//! Windows API: established TCP connections with owning process, window titles and the
//! user name Remotedesktop remembers per server.

/// An established outgoing or incoming TCP connection.
#[derive(Clone, Debug, PartialEq)]
pub struct Tcp {
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub pid: u32,
}

pub use imp::*;

#[cfg(windows)]
mod imp {
    use super::Tcp;
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_CONNECTIONS,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain([0]).collect()
    }

    fn port(raw: u32) -> u16 {
        u16::from_be(raw as u16)
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

    /// All established TCP connections (IPv4 and IPv6). Needs no admin rights.
    pub fn established() -> Vec<Tcp> {
        const ESTABLISHED: u32 = 5;
        let tcp = |af: u16| {
            move |buf: *mut c_void, size: &mut u32| unsafe {
                GetExtendedTcpTable(buf, size, 0, af as u32, TCP_TABLE_OWNER_PID_CONNECTIONS, 0)
            }
        };
        let v4 = table::<MIB_TCPROW_OWNER_PID>(tcp(AF_INET))
            .into_iter()
            .filter(|r| r.dwState == ESTABLISHED)
            .map(|r| Tcp {
                local_port: port(r.dwLocalPort),
                remote_address: Ipv4Addr::from(r.dwRemoteAddr.to_le_bytes()).to_string(),
                remote_port: port(r.dwRemotePort),
                pid: r.dwOwningPid,
            });
        let v6 = table::<MIB_TCP6ROW_OWNER_PID>(tcp(AF_INET6))
            .into_iter()
            .filter(|r| r.dwState == ESTABLISHED)
            .map(|r| {
                let ip = Ipv6Addr::from(r.ucRemoteAddr);
                Tcp {
                    local_port: port(r.dwLocalPort),
                    // IPv4 over an IPv6 socket shows up as ::ffff:a.b.c.d.
                    remote_address: ip.to_ipv4_mapped().map_or(ip.to_string(), |v4| v4.to_string()),
                    remote_port: port(r.dwRemotePort),
                    pid: r.dwOwningPid,
                }
            });
        v4.chain(v6).collect()
    }

    unsafe extern "system" fn collect_title(hwnd: HWND, lparam: LPARAM) -> i32 {
        let titles = &mut *(lparam as *mut HashMap<u32, Vec<String>>);
        if IsWindowVisible(hwnd) != 0 {
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if len > 0 {
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, &mut pid);
                titles.entry(pid).or_default().push(String::from_utf16_lossy(&buf[..len as usize]));
            }
        }
        1
    }

    /// Titles of the visible top-level windows, by process id.
    pub fn window_titles() -> HashMap<u32, Vec<String>> {
        let mut titles: HashMap<u32, Vec<String>> = HashMap::new();
        // SAFETY: the callback only runs during EnumWindows, while `titles` is alive.
        unsafe { EnumWindows(Some(collect_title), &mut titles as *mut _ as LPARAM) };
        titles
    }

    /// The user name Remotedesktop last used for `host` (its "UsernameHint"), if any.
    pub fn rdp_username_hint(host: &str) -> Option<String> {
        if host.is_empty() || host.contains('\\') {
            return None;
        }
        let key = wide(&format!("Software\\Microsoft\\Terminal Server Client\\Servers\\{host}"));
        let value = wide("UsernameHint");
        let mut buf = [0u16; 256];
        let mut size = (buf.len() * 2) as u32;
        // SAFETY: the buffer and its size in bytes are passed together.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if status != 0 {
            return None;
        }
        let len = (size as usize / 2).saturating_sub(1);
        let name = String::from_utf16_lossy(&buf[..len]);
        (!name.is_empty()).then_some(name)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Tcp;
    use std::collections::HashMap;

    pub fn established() -> Vec<Tcp> {
        Vec::new()
    }

    pub fn window_titles() -> HashMap<u32, Vec<String>> {
        HashMap::new()
    }

    pub fn rdp_username_hint(_host: &str) -> Option<String> {
        None
    }
}
