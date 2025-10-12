use std::ffi::{CString, c_void};

use dll_syringe::rpc::RemoteRawProcedure as Proc;
use serde::Deserialize;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory,
    Memory::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx, VirtualFreeEx},
    Threading::{OpenProcess, PROCESS_ALL_ACCESS},
};
pub enum RemoteProcContainer {
    Signal(Proc<extern "system" fn()>),
    Text(Proc<extern "system" fn(usize) -> usize>),
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteProcSignature {
    #[default]
    Signal,
    Text,
}

pub struct ScopedRemoteString {
    proc_handle: HANDLE,
    remote: *mut c_void,
}

impl ScopedRemoteString {
    pub fn new(pid: u32, s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let cmsg = CString::new(s)?;
        let cmsg = cmsg.into_bytes_with_nul();
        unsafe {
            let proc_handle = OpenProcess(PROCESS_ALL_ACCESS, false, pid)?;
            if proc_handle.is_invalid() {
                return Err("OpenProcess failed".into());
            }

            let size = cmsg.len() + 1;
            let remote = VirtualAllocEx(
                proc_handle,
                None,
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );

            if remote.is_null() {
                return Err("VirtualAllocEx failed".into());
            }

            if let Err(e) = WriteProcessMemory(
                proc_handle,
                remote,
                cmsg.as_ptr() as *const _,
                cmsg.len(),
                None,
            ) {
                VirtualFreeEx(proc_handle, remote, 0, MEM_RELEASE)?;
                return Err(e.into());
            }

            Ok(Self {
                proc_handle,
                remote,
            })
        }
    }

    pub fn get_addr(&self) -> usize {
        self.remote as usize
    }
}

impl Drop for ScopedRemoteString {
    fn drop(&mut self) {
        unsafe {
            let _ = VirtualFreeEx(self.proc_handle, self.remote, 0, MEM_RELEASE);
        }
    }
}
