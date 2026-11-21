#![windows_subsystem = "windows"]

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

#[link(name = "user32")]
extern "system" {
    fn EnumWindows(lp_enum_func: Option<unsafe extern "system" fn(isize, isize) -> i32>, l_param: isize) -> i32;
    fn GetWindowTextW(h_wnd: isize, lp_string: *mut u16, n_max_count: i32) -> i32;
    fn ShowWindow(h_wnd: isize, n_cmd_show: i32) -> i32;
    fn SetForegroundWindow(h_wnd: isize) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: isize,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: i32,
    ) -> isize;
}

unsafe extern "system" fn enum_win_proc(hwnd: isize, lparam: isize) -> i32 {
    let mut title = [0u16; 256];
    let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
    if len > 0 {
        let s = String::from_utf16_lossy(&title[..len as usize]);
        if s.starts_with("NitroMan") && !s.contains("HUD") {
            ShowWindow(hwnd, 9); // SW_RESTORE
            SetForegroundWindow(hwnd);
            *(lparam as *mut isize) = hwnd;
            return 0; // stop enumeration
        }
    }
    1 // continue enumeration
}

fn to_wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn main() {
    let mut found_hwnd: isize = 0;
    unsafe {
        EnumWindows(Some(enum_win_proc), &mut found_hwnd as *mut isize as isize);
    }

    if found_hwnd != 0 {
        return;
    }

    let candidates = [
        r"C:\Program Files\NitroMan\NitroMan.exe",
        r"D:\Project\HermesAI\nitroman\target\release\NitroMan.exe",
        r"C:\Program Files (x86)\NitroMan\NitroMan.exe",
    ];

    for path in &candidates {
        if Path::new(path).exists() {
            let op = to_wide("open");
            let target = to_wide(path);
            unsafe {
                ShellExecuteW(0, op.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
            }
            return;
        }
    }
}
