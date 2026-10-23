//! Configuración del núcleo. Todo lo que es *política* (nivel de riesgo,
//! allowlist de VMs, snapshots declarados, llave pública del aprobador) vive
//! aquí, del lado del host. El cliente MCP NO puede cambiar nada de esto:
//! solo manda peticiones de operación.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Nivel bajo: validar Eje-Latam, probar ASC. Red interna aislada.
    /// Las operaciones peligrosas (entregar/ejecutar payload) se rechazan por
    /// construcción.
    Bajo,
    /// Nivel alto: ransomware / malware real. Exige token de aprobación
    /// firmado fuera de banda para toda operación peligrosa.
    Alto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VmEntry {
    /// Snapshots que el núcleo acepta para `rua.revert`. Un snapshot no
    /// declarado se rechaza aunque exista en VirtualBox.
    #[serde(default)]
    pub snapshots: Vec<String>,
    /// Usuario del guest para guestcontrol (copy_to / run / collect).
    #[serde(default)]
    pub guest_username: Option<String>,
    /// Ruta en el host a un archivo con la contraseña del guest
    /// (VBoxManage ... --password-file).
    #[serde(default)]
    pub guest_password_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuaConfig {
    /// Ruta completa a VBoxManage(.exe). En Windows suele ser
    /// C:\Program Files\Oracle\VirtualBox\VBoxManage.exe
    pub vboxmanage_path: String,
    pub risk_level: RiskLevel,
    /// Candado de política adicional: aun en nivel alto, si está en false,
    /// las operaciones peligrosas se rechazan. Nunca se lee del cliente.
    #[serde(default)]
    pub allow_dangerous: bool,
    /// Allowlist: SOLO estas VMs son operables.
    pub vms: BTreeMap<String, VmEntry>,
    /// Ruta del log de auditoría encadenado.
    pub audit_log_path: PathBuf,
    /// Llave pública ed25519 (hex, 32 bytes) del aprobador host-side.
    /// null mientras no estemos en nivel alto (fase 5).
    #[serde(default)]
    pub approval_pubkey_hex: Option<String>,
    /// Persistencia de nonces consumidos (anti-replay de tokens).
    #[serde(default = "default_nonces_path")]
    pub consumed_nonces_path: PathBuf,
}

fn default_nonces_path() -> PathBuf {
    PathBuf::from("rua.nonces.json")
}

impl RuaConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("leyendo config {}: {e}", path.display()))?;
        let cfg: RuaConfig =
            serde_json::from_str(&text).map_err(|e| format!("parseando config: {e}"))?;
        Ok(cfg)
    }

    pub fn vm(&self, name: &str) -> Result<&VmEntry, String> {
        self.vms
            .get(name)
            .ok_or_else(|| format!("VM '{name}' no está en la allowlist"))
    }
}
