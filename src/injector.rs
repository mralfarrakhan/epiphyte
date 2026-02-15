use dll_syringe::{
    Syringe,
    process::{OwnedProcess, OwnedProcessModule, Process},
    rpc::RawRpcFunctionPtr,
};
use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::error;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess},
        UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsHungAppWindow},
    },
    core::BOOL,
};

use crate::{
    payload::Metadata,
    remote::{RemoteProcContainer, RemoteProcSignature},
};

pub struct Injector {
    syringe: Syringe,
    injected_payload: OwnedProcessModule,
    respawner: Command,
    payload_path: PathBuf,
    procedure_table: HashMap<String, Metadata>,
    pid: u32,
}

impl Injector {
    pub fn new(
        target_process: OwnedProcess,
        payload_path: impl AsRef<Path>,
        procedure_table: HashMap<String, Metadata>,
    ) -> Result<Self, Box<dyn Error>> {
        let payload_path = payload_path.as_ref().to_path_buf();

        let exec_path = target_process.path()?;
        let respawner = Command::new(exec_path);

        let syringe = Syringe::for_process(target_process);
        let pid = syringe.process().pid()?.get();
        let injected_payload = syringe.inject(payload_path.clone())?.try_to_owned()?;

        Ok(Self {
            syringe,
            injected_payload,
            respawner,
            payload_path,
            procedure_table,
            pid,
        })
    }

    pub unsafe fn get_raw_procedure<F: RawRpcFunctionPtr>(
        &self,
        name: &str,
    ) -> Result<
        Option<dll_syringe::rpc::RemoteRawProcedure<F>>,
        dll_syringe::error::LoadProcedureError,
    > {
        unsafe {
            self.syringe
                .get_raw_procedure::<F>(self.injected_payload.borrowed(), name)
        }
    }

    pub fn is_alive(&self) -> Result<bool, Box<dyn Error>> {
        let is_process_alive =
            self.syringe.process().is_alive() && self.injected_payload.guess_is_loaded();
        let is_gui_hung = is_process_hung(self.pid)?;

        Ok(is_process_alive && !is_gui_hung)
    }

    pub fn renew(&mut self) -> Result<(), Box<dyn Error>> {
        let new_process = self.respawner.spawn()?.id();
        let new_process = OwnedProcess::from_pid(new_process)?;

        self.syringe = Syringe::for_process(new_process);
        self.injected_payload = self
            .syringe
            .inject(self.payload_path.clone())?
            .try_to_owned()?;
        self.pid = self.syringe.process().pid()?.get();

        Ok(())
    }

    pub fn regenerate(&self) -> HashMap<String, RemoteProcContainer> {
        self.procedure_table
            .iter()
            .filter_map(|(s, m)| {
                if s != "DllMain"
                    && m.is_valid()
                    && let Some(sig) = m.signature
                {
                    let procedure = match sig {
                        RemoteProcSignature::Signal => {
                            RemoteProcContainer::Signal(unsafe { self.get_raw_procedure(s).ok()?? })
                        }
                        RemoteProcSignature::Text => {
                            RemoteProcContainer::Text(unsafe { self.get_raw_procedure(s).ok()?? })
                        }
                    };

                    Some((s.clone(), procedure))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn kill(&self) -> windows::core::Result<()> {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, self.pid)?;
            TerminateProcess(handle, 1)
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl Drop for Injector {
    fn drop(&mut self) {
        if let Err(e) = self.syringe.eject(self.injected_payload.borrowed()) {
            error!("payload ejection error: {:?}", e);
        }
    }
}

fn is_process_hung(target_pid: u32) -> windows::core::Result<bool> {
    struct Context {
        target_pid: u32,
        found_hung: bool,
    }

    let mut ctx = Context {
        target_pid,
        found_hung: false,
    };

    unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let ctx = &mut *(lparam.0 as *mut Context);

            let mut window_pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

            if window_pid == ctx.target_pid && IsHungAppWindow(hwnd).as_bool() {
                ctx.found_hung = true;
                return BOOL(0);
            }

            BOOL(1)
        }
    }

    unsafe {
        EnumWindows(Some(enum_window_proc), LPARAM(&mut ctx as *mut _ as isize))?;
    }

    Ok(ctx.found_hung)
}
