//! Configuración del núcleo.
//!
//! Se lee de un archivo JSON (por defecto `rua.config.json`, o la ruta en
//! la variable de entorno `RUA_CONFIG`). La allowlist de VMs es parte del
//! control de seguridad: el núcleo NO puede operar ninguna VM que no esté
//! declarada aquí. La lista de programas permitidos por VM es parte del
//! candado de ejecución: en nivel bajo solo se ejecutan programas declarados;
//! los no declarados son "peligrosos" y requieren nivel alto + token.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Ruta al ejecutable VBoxManage.
    #[serde(default = "default_vboxmanage")]
    pub vboxmanage_path: String,

    /// Nivel de riesgo del laboratorio. En "bajo", solo se ejecutan los
    /// programas declarados en `allowed_programs`; cualquier otro se rechaza.
    #[serde(default)]
    pub risk_level: RiskLevel,

    /// Permitir ejecuciones peligrosas (programa no declarado). Solo tiene
    /// efecto en nivel "alto", y aun así cada una necesita un token.
    #[serde(default)]
    pub allow_dangerous: bool,

    /// Clave pública ed25519 (32 bytes en hex) del aprobador del host, para
    /// verificar tokens de ejecución peligrosa. La clave PRIVADA no vive aquí:
    /// la tiene el aprobador aparte. Sin esta clave, no hay nivel alto posible.
    #[serde(default)]
    pub approval_pubkey_hex: Option<String>,

    /// Allowlist de VMs sobre las que el núcleo puede operar.
    #[serde(default)]
    pub vms: Vec<VmEntry>,

    /// Archivo del log de auditoría (encadenado, una línea JSON por evento).
    #[serde(default = "default_audit_log")]
    pub audit_log: PathBuf,

    /// Archivo donde se persisten los nonces de tokens ya consumidos
    /// (anti-replay que sobrevive a reinicios del núcleo).
    #[serde(default = "default_nonces")]
    pub consumed_nonces_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VmEntry {
    /// Nombre de la VM tal como la conoce VirtualBox.
    pub name: String,
    /// Snapshots permitidos para esta VM (p. ej. los estados de postura).
    #[serde(default)]
    pub snapshots: Vec<String>,
    /// Programas que SÍ se pueden ejecutar en esta VM sin token, en cualquier
    /// nivel (p. ej. el agente de Eje-Latam). Un programa fuera de esta lista
    /// es "peligroso": prohibido en nivel bajo, con token en nivel alto.
    #[serde(default)]
    pub allowed_programs: Vec<String>,
    /// Hasher INTERNO del transporte verificado, ejecutado dentro del guest
    /// sobre el fichero recién copiado. `{path}` se sustituye por la ruta
    /// destino en el guest. Lo fija el host (no el cliente), así que no pasa
    /// por el candado de ejecución. Por defecto, Linux: `sha256sum {path}`.
    #[serde(default = "default_hasher")]
    pub guest_hasher: Vec<String>,
    /// Separador de rutas del guest, para componer destino = dir + sep + base.
    /// Linux: "/". Windows: "\\".
    #[serde(default = "default_guest_sep")]
    pub guest_sep: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Nivel bajo: validar Eje-Latam, probar ASC. Sin código hostil.
    #[default]
    Bajo,
    /// Nivel alto: ransomware y malware real. Exige red con internet
    /// simulado y token de aprobación para detonar.
    Alto,
}

impl Config {
    /// Carga la configuración desde `RUA_CONFIG` o `rua.config.json`.
    pub fn load() -> Result<Self, String> {
        let path = std::env::var("RUA_CONFIG").unwrap_or_else(|_| "rua.config.json".to_string());
        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("no se pudo leer la config '{path}': {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("config inválida en '{path}': {e}"))
    }

    /// Busca una VM en la allowlist.
    pub fn vm(&self, name: &str) -> Option<&VmEntry> {
        self.vms.iter().find(|v| v.name == name)
    }
}

fn default_vboxmanage() -> String {
    // Ruta típica de instalación en Windows (confirmada: VirtualBox 7.2.14).
    "C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe".to_string()
}

fn default_audit_log() -> PathBuf {
    PathBuf::from("rua-audit.log")
}

fn default_nonces() -> PathBuf {
    PathBuf::from("rua-nonces.json")
}

fn default_hasher() -> Vec<String> {
    vec!["sha256sum".to_string(), "{path}".to_string()]
}

fn default_guest_sep() -> String {
    "/".to_string()
}
