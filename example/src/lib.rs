use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
    thread,
};

use windows::{
    Win32::{
        Foundation::HMODULE,
        System::{LibraryLoader::GetModuleHandleA, SystemServices::DLL_PROCESS_ATTACH},
        UI::WindowsAndMessaging::{MB_OK, MessageBoxA},
    },
    core::{PCSTR, s},
};

const TITLE: PCSTR = s!("This Is Epiphyte");

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_: HMODULE, fwd_reason: u32, _: *mut c_void) -> bool {
    if fwd_reason == DLL_PROCESS_ATTACH {
        thread::spawn(move || unsafe { MessageBoxA(None, s!("Hello, World!"), TITLE, MB_OK) });
    }

    true
}

#[unsafe(no_mangle)]
pub extern "system" fn offset() {
    let process_handle = unsafe { GetModuleHandleA(None).unwrap() };
    let handle_str = format!("handle ptr: {:p}", process_handle.0);

    thread::spawn(move || unsafe {
        MessageBoxA(None, PCSTR::from_raw(handle_str.as_ptr()), TITLE, MB_OK)
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn greet(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let msg = unsafe { CStr::from_ptr(msg).to_str().unwrap_or("") };
    let msg = format!("Hello, {}", msg);

    thread::spawn(move || unsafe {
        MessageBoxA(None, PCSTR::from_raw(msg.as_ptr()), TITLE, MB_OK)
    });
}
