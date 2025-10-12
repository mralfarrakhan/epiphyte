use std::cmp;
use std::ffi::{CString, c_void};
use std::mem::MaybeUninit;

use dll_syringe::rpc::RemoteRawProcedure as Proc;
use serde::Deserialize;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQueryEx};
use windows::Win32::System::Threading::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory,
    Memory::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx, VirtualFreeEx},
    Threading::{OpenProcess, PROCESS_ALL_ACCESS},
};

const CHUNK: usize = 1024;

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
    address: *mut c_void,
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
            let address = VirtualAllocEx(
                proc_handle,
                None,
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );

            if address.is_null() {
                CloseHandle(proc_handle)?;
                return Err("VirtualAllocEx failed".into());
            }

            if let Err(e) = WriteProcessMemory(
                proc_handle,
                address,
                cmsg.as_ptr() as *const _,
                cmsg.len(),
                None,
            ) {
                VirtualFreeEx(proc_handle, address, 0, MEM_RELEASE)?;
                CloseHandle(proc_handle)?;
                return Err(e.into());
            }

            Ok(Self {
                proc_handle,
                address,
            })
        }
    }

    pub fn from_remote(pid: u32, address: usize) -> Result<Self, Box<dyn std::error::Error>> {
        if address == 0 {
            return Err("Null address".into());
        }
        unsafe {
            let proc_handle = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, false, pid)?;
            if proc_handle.is_invalid() {
                return Err("OpenProcess failed".into());
            }

            Ok(Self {
                proc_handle,
                address: address as _,
            })
        }
    }

    pub fn read_remote(&self) -> Result<String, Box<dyn std::error::Error>> {
        unsafe {
            let mut mbi = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
            let mbi_size = std::mem::size_of::<MEMORY_BASIC_INFORMATION>();
            let res = VirtualQueryEx(
                self.proc_handle,
                Some(self.address as _),
                mbi.as_mut_ptr() as _,
                mbi_size,
            );
            if res == 0 {
                return Err("VirtualQueryEx failed".into());
            }

            let mbi = mbi.assume_init();
            let max_read = mbi.RegionSize;

            let mut out = Vec::new();
            let mut offset = 0usize;

            while offset < max_read {
                let to_read = cmp::min(CHUNK, max_read - offset);
                let mut buf = vec![0u8; to_read];
                let mut bytes_read = 0usize;
                if let Err(e) = ReadProcessMemory(
                    self.proc_handle,
                    self.address.add(offset) as _,
                    buf.as_mut_ptr() as _,
                    to_read,
                    Some(&mut bytes_read),
                ) {
                    return Err(e.into());
                }

                buf.truncate(bytes_read);
                if let Some(pos) = buf.iter().position(|&b| b == 0) {
                    out.extend_from_slice(&buf[..pos]);
                    let out = String::from_utf8(out)?;
                    return Ok(out);
                } else {
                    out.extend_from_slice(&buf);
                    offset += bytes_read;
                    if bytes_read == 0 {
                        break;
                    }
                }
            }
        }

        Err("Null terminator not found in region".into())
    }

    pub fn get_addr(&self) -> usize {
        self.address as usize
    }
}

impl Drop for ScopedRemoteString {
    fn drop(&mut self) {
        unsafe {
            let _ = VirtualFreeEx(self.proc_handle, self.address, 0, MEM_RELEASE);
            let _ = CloseHandle(self.proc_handle);
        }
    }
}
