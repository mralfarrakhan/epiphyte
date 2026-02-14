use dll_syringe::{
    Syringe,
    error::InjectError,
    process::{OwnedProcess, OwnedProcessModule},
    rpc::RawRpcFunctionPtr,
};
use std::path::Path;
use tracing::error;

pub struct Injector {
    syringe: Syringe,
    injected_payload: OwnedProcessModule,
}

impl Injector {
    pub fn new(
        target_process: OwnedProcess,
        payload_path: impl AsRef<Path>,
    ) -> Result<Self, InjectError> {
        let syringe = Syringe::for_process(target_process);
        let injected_payload = syringe.inject(payload_path)?.try_to_owned()?;

        Ok(Self {
            syringe,
            injected_payload,
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
}

impl Drop for Injector {
    fn drop(&mut self) {
        if let Err(e) = self.syringe.eject(self.injected_payload.borrowed()) {
            error!("payload ejection error: {:?}", e);
        }
    }
}
