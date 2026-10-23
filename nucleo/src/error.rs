//! Tipos de error del núcleo.
//!
//! Un solo enum con las formas de fallo que nos importan. Todas se
//! convierten a texto legible que viaja de vuelta al cliente MCP.

use std::fmt;

#[derive(Debug)]
pub enum NucleoError {
    /// La VM no está en la allowlist de la configuración.
    VmNoPermitida(String),
    /// El snapshot no está permitido para esa VM.
    SnapshotNoPermitido { vm: String, snapshot: String },
    /// Una operación peligrosa fue rechazada por el candado de riesgo.
    Rechazada(String),
    /// Falló la verificación de la aprobación host-side.
    Aprobacion(String),
    /// VBoxManage devolvió un error (su stderr).
    Vbox(String),
    /// Error de entrada/salida al invocar VBoxManage.
    Io(String),
    /// Argumentos inválidos o faltantes en la llamada.
    Args(String),
}

impl fmt::Display for NucleoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NucleoError::VmNoPermitida(vm) => {
                write!(f, "VM no permitida: '{vm}' no está en la allowlist de la configuración")
            }
            NucleoError::SnapshotNoPermitido { vm, snapshot } => {
                write!(f, "snapshot no permitido: '{snapshot}' no está declarado para la VM '{vm}'")
            }
            NucleoError::Rechazada(motivo) => write!(f, "operación rechazada: {motivo}"),
            NucleoError::Aprobacion(motivo) => write!(f, "aprobación rechazada: {motivo}"),
            NucleoError::Vbox(detalle) => write!(f, "VBoxManage falló: {detalle}"),
            NucleoError::Io(detalle) => write!(f, "error de E/S: {detalle}"),
            NucleoError::Args(detalle) => write!(f, "argumentos inválidos: {detalle}"),
        }
    }
}

impl std::error::Error for NucleoError {}
