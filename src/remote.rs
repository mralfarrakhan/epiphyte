use std::os::raw::{c_char, c_void};

use dll_syringe::rpc::RemoteRawProcedure as Proc;
use serde::Deserialize;

pub enum Signature {
    Signal(Proc<extern "system" fn()>),
    Blob(Proc<extern "system" fn(*const c_void, u32, *mut c_void, u32) -> u32>),
    Multiplex(Proc<extern "system" fn(u32, *const c_void, u32, *mut c_void, u32) -> u32>),
    Text(Proc<extern "system" fn(*const c_char) -> *mut c_char>),
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureConfig {
    #[default]
    Signal,
    Blob,
    MBlob,
    Text,
}
