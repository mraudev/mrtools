//! Windows API: established TCP connections with owning process, windows, the registry,
//! the credential store (for Remotedesktop passwords) and starting programs via the shell.

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

    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_NOT_FOUND, HWND, LPARAM};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_CONNECTIONS,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_DOMAIN_PASSWORD,
    };
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_DWORD,
        RRF_RT_REG_SZ,
    };
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow,
        ShowWindow, SW_RESTORE, SW_SHOWNORMAL,
    };

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain([0]).collect()
    }

    fn last_error() -> String {
        std::io::Error::last_os_error().to_string()
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

    /// Visible top-level windows with a title, by process id.
    fn windows() -> HashMap<u32, Vec<(HWND, String)>> {
        unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> i32 {
            let found = &mut *(lparam as *mut HashMap<u32, Vec<(HWND, String)>>);
            if IsWindowVisible(hwnd) != 0 {
                let mut buf = [0u16; 512];
                let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
                if len > 0 {
                    let mut pid = 0u32;
                    GetWindowThreadProcessId(hwnd, &mut pid);
                    found.entry(pid).or_default().push((hwnd, String::from_utf16_lossy(&buf[..len as usize])));
                }
            }
            1
        }
        let mut found: HashMap<u32, Vec<(HWND, String)>> = HashMap::new();
        // SAFETY: the callback only runs during EnumWindows, while `found` is alive.
        unsafe { EnumWindows(Some(collect), &mut found as *mut _ as LPARAM) };
        found
    }

    /// Titles of the visible top-level windows, by process id.
    pub fn window_titles() -> HashMap<u32, Vec<String>> {
        windows()
            .into_iter()
            .map(|(pid, list)| (pid, list.into_iter().map(|(_, title)| title).collect()))
            .collect()
    }

    /// Brings the main window of a process to the front (restored if minimized).
    pub fn focus(pid: u32) -> Result<(), String> {
        let windows = windows();
        let (hwnd, _) = windows
            .get(&pid)
            .and_then(|list| list.first())
            .ok_or("Kein Fenster gefunden – die Sitzung läuft vielleicht in einer Konsole ohne eigenes Fenster.")?;
        // SAFETY: the handle comes from EnumWindows just now; stale handles only make the calls fail.
        unsafe {
            if IsIconic(*hwnd) != 0 {
                ShowWindow(*hwnd, SW_RESTORE);
            }
            SetForegroundWindow(*hwnd);
        }
        Ok(())
    }

    pub fn terminate(pid: u32) -> Result<(), String> {
        // SAFETY: plain handle use, closed on every path.
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

    /// A string value under HKEY_CURRENT_USER, if present.
    pub fn reg_string(key: &str, value: &str) -> Option<String> {
        let key = wide(key);
        let value = wide(value);
        let mut buf = [0u16; 1024];
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
        Some(String::from_utf16_lossy(&buf[..len]))
    }

    /// A DWORD value under HKEY_CURRENT_USER, if present.
    pub fn reg_dword(key: &str, value: &str) -> Option<u32> {
        let key = wide(key);
        let value = wide(value);
        let mut data = 0u32;
        let mut size = 4u32;
        // SAFETY: a u32 and its size are passed together.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut data as *mut u32).cast(),
                &mut size,
            )
        };
        (status == 0).then_some(data)
    }

    /// Names of the subkeys of a key under HKEY_CURRENT_USER (empty if it does not exist).
    pub fn reg_subkeys(key: &str) -> Vec<String> {
        let key = wide(key);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: the handle is closed below.
        if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_READ, &mut handle) } != 0 {
            return Vec::new();
        }
        let mut names = Vec::new();
        for index in 0.. {
            let mut buf = [0u16; 256];
            let mut len = buf.len() as u32;
            // SAFETY: the buffer and its length in characters are passed together.
            let status = unsafe {
                RegEnumKeyExW(
                    handle,
                    index,
                    buf.as_mut_ptr(),
                    &mut len,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if status != 0 {
                break;
            }
            names.push(String::from_utf16_lossy(&buf[..len as usize]));
        }
        // SAFETY: opened above.
        unsafe { RegCloseKey(handle) };
        names
    }

    /// The user name Remotedesktop last used for `host` (its "UsernameHint"), if any.
    pub fn rdp_username_hint(host: &str) -> Option<String> {
        if host.is_empty() || host.contains('\\') {
            return None;
        }
        reg_string(&format!("Software\\Microsoft\\Terminal Server Client\\Servers\\{host}"), "UsernameHint")
            .filter(|name| !name.is_empty())
    }

    /// The credential Remotedesktop reads for `host`.
    fn rdp_target(host: &str) -> Vec<u16> {
        wide(&format!("TERMSRV/{host}"))
    }

    pub fn has_rdp_password(host: &str) -> bool {
        let target = rdp_target(host);
        let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: CredRead allocates the result, freed with CredFree. The password of a domain
        // credential is not returned to normal processes, only its existence matters here.
        unsafe {
            if CredReadW(target.as_ptr(), CRED_TYPE_DOMAIN_PASSWORD, 0, &mut credential) == 0 {
                return false;
            }
            CredFree(credential.cast());
        }
        true
    }

    pub fn set_rdp_password(host: &str, user: &str, password: &str) -> Result<(), String> {
        let mut target = rdp_target(host);
        let mut user = wide(user);
        let blob: Vec<u8> = password.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_DOMAIN_PASSWORD,
            TargetName: target.as_mut_ptr(),
            Comment: std::ptr::null_mut(),
            LastWritten: Default::default(),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: user.as_mut_ptr(),
        };
        // SAFETY: every pointer refers to a buffer that lives until after the call.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            return Err(last_error());
        }
        Ok(())
    }

    pub fn delete_rdp_password(host: &str) -> Result<(), String> {
        let target = rdp_target(host);
        // SAFETY: plain call with a null-terminated string.
        if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_DOMAIN_PASSWORD, 0) } == 0 {
            // SAFETY: reads the thread's last error right after the failed call.
            if unsafe { GetLastError() } != ERROR_NOT_FOUND {
                return Err(last_error());
            }
        }
        Ok(())
    }

    /// Starts a program through the shell, so a UAC prompt appears if it needs admin rights.
    pub fn shell_open(program: &str, parameters: &str) -> Result<(), String> {
        let verb = wide("open");
        let program = wide(program);
        let parameters = wide(parameters);
        // SAFETY: all strings are null-terminated and live until after the call.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                program.as_ptr(),
                parameters.as_ptr(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        } as isize;
        // Values up to 32 are errors; 5 is "access denied", e.g. a declined UAC prompt.
        match result {
            r if r > 32 => Ok(()),
            5 => Err("Abgebrochen oder keine Berechtigung.".into()),
            r => Err(format!("Start fehlgeschlagen (Fehler {r}).")),
        }
    }
}
