use std::os::raw::c_char;

use dll_syringe::rpc::RemoteRawProcedure as Proc;
use serde::Deserialize;

pub enum RemoteProcContainer {
    Signal(Proc<extern "system" fn()>),
    Text(Proc<extern "system" fn(*const c_char) -> *mut c_char>),
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteProcSignature {
    #[default]
    Signal,
    Text,
}

pub fn write_remote_string(pid: u32, s: &str) -> Result<usize, Box<dyn std::error::Error>> {
    unsafe { Ok(11) }
}
