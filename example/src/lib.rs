use std::{
    ffi::{CStr, CString, c_char, c_void},
    ptr, thread,
};

use windows::{
    Win32::{
        Foundation::HMODULE,
        System::{
            LibraryLoader::GetModuleHandleA,
            Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc},
            SystemServices::DLL_PROCESS_ATTACH,
        },
        UI::WindowsAndMessaging::{MB_OK, MessageBoxA},
    },
    core::PCSTR,
};

const TITLE: &str = "This is Epiphyte!";

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_: HMODULE, fwd_reason: u32, _: *mut c_void) -> bool {
    if fwd_reason == DLL_PROCESS_ATTACH {
        thread::spawn(move || {
            dbgmsgbox("Hello, World!".into(), None);
        });
    }

    true
}

#[unsafe(no_mangle)]
pub extern "system" fn offset() {
    let process_handle = unsafe { GetModuleHandleA(None).unwrap() };
    let handle_str = format!("handle ptr: {:p}", process_handle.0);

    thread::spawn(move || {
        dbgmsgbox(handle_str, None);
    });
}

/// # Safety
///
/// This function is dangerous, like, really dangerous.
///
/// WARNING: USE VirtualAlloc FOR STRINGS THAT WILL BE RETURNED.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn greet(msg: *const c_char) -> usize {
    unsafe {
        if msg.is_null() {
            return 0;
        }

        let msg = CStr::from_ptr(msg).to_str().unwrap_or("");
        let msg = format!("Hello, {}!", msg);

        let cmsg = CString::new(msg.clone())
            .unwrap_or_default()
            .into_bytes_with_nul();

        let size = cmsg.len();

        let mem = VirtualAlloc(None, size, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
        if mem.is_null() {
            return 0;
        }

        ptr::copy_nonoverlapping(cmsg.as_ptr(), mem as _, size);

        thread::spawn(move || {
            dbgmsgbox(msg, None);
        });

        mem as _
    }
}

fn dbgmsgbox(message: String, title: Option<String>) {
    thread::spawn(move || unsafe {
        MessageBoxA(
            None,
            PCSTR::from_raw(message.as_ptr()),
            PCSTR::from_raw(title.unwrap_or(TITLE.into()).as_ptr()),
            MB_OK,
        );
    });
}
