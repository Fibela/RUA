//! Envoltorio delgado sobre VBoxManage. No decide nada: arma líneas de comando
//! fijas y captura stdout/stderr/status. Toda la política vive en `ops`/`config`.
//!
//! Sintaxis fijada para VirtualBox 7.x (confirmado 7.2.14). Las banderas de
//! credenciales de guestcontrol (--username / --password-file) se centralizan
//! aquí para que, si difieren en tu build, se ajusten en un solo sitio.

use std::path::Path;
use std::process::Command;

pub struct Vbox {
    exe: String,
}

#[derive(Debug)]
pub struct CmdOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Vbox {
    pub fn new(exe: impl Into<String>) -> Self {
        Self { exe: exe.into() }
    }

    fn run(&self, args: &[String]) -> Result<CmdOutput, String> {
        let out = Command::new(&self.exe)
            .args(args)
            .output()
            .map_err(|e| format!("no se pudo ejecutar {}: {e}", self.exe))?;
        Ok(CmdOutput {
            status: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    pub fn list_vms(&self) -> Result<CmdOutput, String> {
        self.run(&["list".into(), "vms".into()])
    }

    /// `VBoxManage snapshot <vm> restore <nombre>` — exige la VM apagada.
    pub fn snapshot_restore(&self, vm: &str, snapshot: &str) -> Result<CmdOutput, String> {
        self.run(&[
            "snapshot".into(),
            vm.into(),
            "restore".into(),
            snapshot.into(),
        ])
    }

    pub fn start_headless(&self, vm: &str) -> Result<CmdOutput, String> {
        self.run(&[
            "startvm".into(),
            vm.into(),
            "--type".into(),
            "headless".into(),
        ])
    }

    pub fn poweroff(&self, vm: &str) -> Result<CmdOutput, String> {
        self.run(&["controlvm".into(), vm.into(), "poweroff".into()])
    }

    /// Host -> guest. Entrega el payload; es operación peligrosa (ver `ops`).
    pub fn copy_to(
        &self,
        vm: &str,
        user: &str,
        pass_file: &str,
        source: &Path,
        target_dir: &str,
    ) -> Result<CmdOutput, String> {
        self.run(&[
            "guestcontrol".into(),
            vm.into(),
            "--username".into(),
            user.into(),
            "--password-file".into(),
            pass_file.into(),
            "copyto".into(),
            source.display().to_string(),
            "--target-directory".into(),
            target_dir.into(),
        ])
    }

    /// Guest -> host. Recolección; no es peligrosa.
    pub fn copy_from(
        &self,
        vm: &str,
        user: &str,
        pass_file: &str,
        guest_src: &str,
        host_dest_dir: &str,
    ) -> Result<CmdOutput, String> {
        self.run(&[
            "guestcontrol".into(),
            vm.into(),
            "--username".into(),
            user.into(),
            "--password-file".into(),
            pass_file.into(),
            "copyfrom".into(),
            guest_src.into(),
            "--target-directory".into(),
            host_dest_dir.into(),
        ])
    }

    /// Ejecuta un programa dentro del guest. Primitiva peligrosa.
    ///
    /// En 7.x, tras `--` el primer token es argv[0]. Si se omite, los
    /// argumentos del programa quedan corridos una posición. Por eso se
    /// antepone `exe` como argv[0] antes de los argumentos del usuario.
    pub fn run_guest(
        &self,
        vm: &str,
        user: &str,
        pass_file: &str,
        exe: &str,
        args: &[String],
    ) -> Result<CmdOutput, String> {
        let mut a: Vec<String> = vec![
            "guestcontrol".into(),
            vm.into(),
            "--username".into(),
            user.into(),
            "--password-file".into(),
            pass_file.into(),
            "run".into(),
            "--exe".into(),
            exe.into(),
            "--".into(),
            exe.into(), // argv[0]
        ];
        a.extend_from_slice(args);
        self.run(&a)
    }
}
