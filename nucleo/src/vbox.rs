//! Wrapper de VBoxManage: las operaciones cerradas del núcleo.
//!
//! Punto clave de seguridad: NUNCA se arma una shell. Se invoca el binario
//! VBoxManage directamente con una lista de argumentos (`Command::args`),
//! así que no hay inyección por la shell. El agente de IA, más arriba, solo
//! puede pedir estas operaciones con nombre; jamás recibe una shell del host.

use crate::config::{Config, VmEntry};
use crate::error::NucleoError;
use std::process::Command;

pub struct Vbox<'a> {
    cfg: &'a Config,
}

impl<'a> Vbox<'a> {
    pub fn new(cfg: &'a Config) -> Self {
        Vbox { cfg }
    }

    /// Ejecuta VBoxManage con argumentos fijos y devuelve su stdout.
    fn run_raw(&self, args: &[String]) -> Result<String, NucleoError> {
        let output = Command::new(&self.cfg.vboxmanage_path)
            .args(args)
            .output()
            .map_err(|e| NucleoError::Io(format!("no se pudo ejecutar VBoxManage: {e}")))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(NucleoError::Vbox(err));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Verifica que la VM esté en la allowlist. Sin esto, el agente podría
    /// pedir operar cualquier VM del host. Con esto, solo las declaradas.
    fn ensure_vm(&self, vm: &str) -> Result<&VmEntry, NucleoError> {
        self.cfg
            .vms
            .iter()
            .find(|v| v.name == vm)
            .ok_or_else(|| NucleoError::VmNoPermitida(vm.to_string()))
    }

    // --- Operaciones ---

    pub fn list_vms(&self) -> Result<String, NucleoError> {
        self.run_raw(&["list".into(), "vms".into()])
    }

    /// Revierte la VM a un snapshot declarado (p. ej. un estado de postura).
    pub fn snapshot_restore(&self, vm: &str, snapshot: &str) -> Result<String, NucleoError> {
        let entry = self.ensure_vm(vm)?;
        if !entry.snapshots.iter().any(|s| s == snapshot) {
            return Err(NucleoError::SnapshotNoPermitido {
                vm: vm.to_string(),
                snapshot: snapshot.to_string(),
            });
        }
        self.run_raw(&[
            "snapshot".into(),
            vm.into(),
            "restore".into(),
            snapshot.into(),
        ])
    }

    pub fn start(&self, vm: &str, headless: bool) -> Result<String, NucleoError> {
        self.ensure_vm(vm)?;
        let kind = if headless { "headless" } else { "gui" };
        self.run_raw(&["startvm".into(), vm.into(), "--type".into(), kind.into()])
    }

    /// Apaga la VM. Por defecto, apagado ACPI (limpio); `force` usa poweroff.
    pub fn stop(&self, vm: &str, force: bool) -> Result<String, NucleoError> {
        self.ensure_vm(vm)?;
        let verb = if force { "poweroff" } else { "acpipowerbutton" };
        self.run_raw(&["controlvm".into(), vm.into(), verb.into()])
    }

    /// Copia un archivo del host a una RUTA destino COMPLETA dentro de la VM.
    /// Se usa la forma explícita `copyto <origen> <destino>` (no
    /// `--target-directory`) porque en VBox 7.2 esa bandera se malinterpreta y
    /// toma el directorio como destino ("already exists and is a directory").
    pub fn copy_in(
        &self,
        vm: &str,
        host_path: &str,
        guest_dest: &str,
        user: &str,
        password_file: &str,
    ) -> Result<String, NucleoError> {
        self.ensure_vm(vm)?;
        self.run_raw(&[
            "guestcontrol".into(),
            vm.into(),
            "copyto".into(),
            "--username".into(),
            user.into(),
            "--passwordfile".into(),
            password_file.into(),
            host_path.into(),
            guest_dest.into(),
        ])
    }

    /// Recolecta un archivo/carpeta de la VM hacia un directorio del host
    /// (logs de Sysmon, pcap, lo que el agente de Eje-Latam haya producido).
    pub fn collect(
        &self,
        vm: &str,
        guest_path: &str,
        host_dir: &str,
        user: &str,
        password_file: &str,
    ) -> Result<String, NucleoError> {
        self.ensure_vm(vm)?;
        self.run_raw(&[
            "guestcontrol".into(),
            vm.into(),
            "copyfrom".into(),
            "--username".into(),
            user.into(),
            "--passwordfile".into(),
            password_file.into(),
            "--target-directory".into(),
            host_dir.into(),
            guest_path.into(),
        ])
    }

    /// Ejecuta un programa dentro de la VM. El candado de ejecución (programa
    /// declarado / peligroso) vive más arriba, en main.rs, antes de llamar aquí.
    pub fn run_guest(
        &self,
        vm: &str,
        program: &str,
        prog_args: &[String],
        user: &str,
        password_file: &str,
    ) -> Result<String, NucleoError> {
        self.ensure_vm(vm)?;
        let mut args: Vec<String> = vec![
            "guestcontrol".into(),
            vm.into(),
            "run".into(),
            "--username".into(),
            user.into(),
            "--passwordfile".into(),
            password_file.into(),
            "--exe".into(),
            program.into(),
            // En VBox 7.2, `run --exe X -- a b` ya pone argv[0]=X por sí mismo;
            // los tokens tras `--` son los ARGUMENTOS (argv[1..]). NO duplicar
            // el programa aquí: hacerlo lo mete como primer argumento (p. ej.
            // `sha256sum` se hashearía a sí mismo en vez del archivo).
            "--".into(),
        ];
        args.extend(prog_args.iter().cloned());
        self.run_raw(&args)
    }
}
