//! Aprobación host-side por token firmado (ed25519), fuera del canal MCP.
//!
//! Propiedad que sostiene: el cliente MCP (a futuro, la IA) NO puede
//! autoaprobarse. El aprobador (`rua-approve`, fase 5) tiene la llave privada;
//! el núcleo solo tiene la pública. El cliente pasa un token; el núcleo lo
//! verifica. Aunque el cliente lea la config (la pública), no puede forjarlo.
//!
//! El token liga el permiso a la petición EXACTA: vm + exe + args + hash del
//! payload. Firmar solo "muestra X" dejaría al cliente pedir aprobación para X
//! y luego mandar otros args u otro binario; el hash del payload cierra eso.
//! Nonce de un solo uso + expiración contra el reloj del host = una aprobación,
//! una detonación.
//!
//! PREMISA CRÍTICA: la seguridad se reduce a que el cliente no tiene filesystem
//! ni shell del host, por eso no puede leer la llave privada de `rua-approve`.
//! El día que la IA reciba una herramienta de archivos del host, la privada hay
//! que moverla a un keystore del SO o a una máquina aprobadora aparte.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum ApprovalError {
    /// No hay llave pública configurada (no se puede aprobar nada).
    NoKey,
    /// Token expirado contra el reloj del host.
    Expired,
    /// Nonce ya consumido (replay).
    NonceReused,
    /// Firma inválida: no corresponde a esta petición y esta llave.
    BadSignature,
    /// Token mal formado.
    BadToken(String),
}

impl std::fmt::Display for ApprovalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalError::NoKey => write!(f, "no hay llave de aprobación configurada"),
            ApprovalError::Expired => write!(f, "token expirado"),
            ApprovalError::NonceReused => write!(f, "nonce ya consumido (replay)"),
            ApprovalError::BadSignature => write!(f, "firma inválida para esta petición"),
            ApprovalError::BadToken(s) => write!(f, "token mal formado: {s}"),
        }
    }
}

/// Descriptor canónico de la operación a aprobar. El orden de los campos es
/// parte del contrato de firma: no reordenar.
#[derive(Serialize)]
pub struct OperationDescriptor {
    pub vm: String,
    pub exe: String,
    pub args: Vec<String>,
    /// SHA-256 (hex) del archivo del host que se copiará/ejecutará. "" si no
    /// aplica (p.ej. `run` sobre un binario ya presente en el guest).
    pub payload_sha256: String,
}

#[derive(Serialize)]
struct Claim<'a> {
    descriptor: &'a OperationDescriptor,
    nonce: &'a str,
    expires_at: i64,
}

/// Lo que el cliente pasa en la llamada. NO incluye ninguna decisión: solo la
/// prueba (firma) producida por el aprobador host-side.
pub struct ApprovalToken {
    pub nonce: String,
    pub expires_at: i64,
    pub signature_hex: String,
}

pub struct ApprovalVerifier {
    pubkey: Option<VerifyingKey>,
    nonces_path: PathBuf,
}

impl ApprovalVerifier {
    pub fn new(pubkey_hex: Option<&str>, nonces_path: impl Into<PathBuf>) -> Result<Self, String> {
        let pubkey = match pubkey_hex {
            None => None,
            Some(h) => {
                let bytes = hex::decode(h).map_err(|e| format!("pubkey hex inválida: {e}"))?;
                let arr: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| "pubkey debe ser 32 bytes".to_string())?;
                Some(
                    VerifyingKey::from_bytes(&arr)
                        .map_err(|e| format!("pubkey inválida: {e}"))?,
                )
            }
        };
        Ok(Self {
            pubkey,
            nonces_path: nonces_path.into(),
        })
    }

    /// Bytes canónicos sobre los que se firma/verifica. Mismo orden siempre.
    pub fn canonical_claim(desc: &OperationDescriptor, nonce: &str, expires_at: i64) -> Vec<u8> {
        let claim = Claim {
            descriptor: desc,
            nonce,
            expires_at,
        };
        serde_json::to_vec(&claim).expect("serializar claim")
    }

    fn load_nonces(&self) -> BTreeSet<String> {
        std::fs::read_to_string(&self.nonces_path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn save_nonces(&self, set: &BTreeSet<String>) -> Result<(), String> {
        let text = serde_json::to_string(set).map_err(|e| e.to_string())?;
        std::fs::write(&self.nonces_path, text).map_err(|e| e.to_string())
    }

    /// Verifica el token contra la petición exacta. Si pasa, consume el nonce
    /// (anti-replay) antes de devolver Ok.
    pub fn verify(
        &self,
        desc: &OperationDescriptor,
        token: &ApprovalToken,
        now: i64,
    ) -> Result<(), ApprovalError> {
        let pubkey = self.pubkey.as_ref().ok_or(ApprovalError::NoKey)?;
        if now > token.expires_at {
            return Err(ApprovalError::Expired);
        }
        let mut nonces = self.load_nonces();
        if nonces.contains(&token.nonce) {
            return Err(ApprovalError::NonceReused);
        }
        let msg = Self::canonical_claim(desc, &token.nonce, token.expires_at);
        let sig_bytes =
            hex::decode(&token.signature_hex).map_err(|e| ApprovalError::BadToken(e.to_string()))?;
        let sig_arr: [u8; 64] = sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| ApprovalError::BadToken("firma debe ser 64 bytes".into()))?;
        let sig = Signature::from_bytes(&sig_arr);
        pubkey
            .verify(&msg, &sig)
            .map_err(|_| ApprovalError::BadSignature)?;
        // Consumir el nonce SOLO después de verificar la firma.
        nonces.insert(token.nonce.clone());
        self.save_nonces(&nonces).map_err(ApprovalError::BadToken)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::sync::atomic::{AtomicU64, Ordering};

    static CTR: AtomicU64 = AtomicU64::new(0);

    fn tmp_nonces() -> PathBuf {
        let mut p = std::env::temp_dir();
        let n = CTR.fetch_add(1, Ordering::SeqCst);
        p.push(format!(
            "rua_nonces_{}_{}.json",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_file(&p);
        p
    }

    fn sk(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn desc() -> OperationDescriptor {
        OperationDescriptor {
            vm: "golden-win".into(),
            exe: "C:\\muestra.exe".into(),
            args: vec!["--go".into()],
            payload_sha256: "abcd".into(),
        }
    }

    fn token(sk: &SigningKey, d: &OperationDescriptor, nonce: &str, exp: i64) -> ApprovalToken {
        let msg = ApprovalVerifier::canonical_claim(d, nonce, exp);
        let sig = sk.sign(&msg);
        ApprovalToken {
            nonce: nonce.into(),
            expires_at: exp,
            signature_hex: hex::encode(sig.to_bytes()),
        }
    }

    #[test]
    fn token_valido_pasa() {
        let key = sk(1);
        let v = ApprovalVerifier::new(Some(&hex::encode(key.verifying_key().to_bytes())), tmp_nonces())
            .unwrap();
        let d = desc();
        let t = token(&key, &d, "n-ok", 10_000_000_000);
        assert_eq!(v.verify(&d, &t, 1_000), Ok(()));
    }

    #[test]
    fn vm_distinta_falla() {
        let key = sk(1);
        let v = ApprovalVerifier::new(Some(&hex::encode(key.verifying_key().to_bytes())), tmp_nonces())
            .unwrap();
        let firmado = desc();
        let t = token(&key, &firmado, "n-vm", 10_000_000_000);
        // Verificamos contra un descriptor con otra vm: la firma no cuadra.
        let mut otro = desc();
        otro.vm = "otra-vm".into();
        assert_eq!(v.verify(&otro, &t, 1_000), Err(ApprovalError::BadSignature));
    }

    #[test]
    fn nonce_repetido_falla() {
        let key = sk(1);
        let v = ApprovalVerifier::new(Some(&hex::encode(key.verifying_key().to_bytes())), tmp_nonces())
            .unwrap();
        let d = desc();
        let t = token(&key, &d, "n-rep", 10_000_000_000);
        assert_eq!(v.verify(&d, &t, 1_000), Ok(()));
        // Mismo token otra vez: nonce ya consumido.
        let t2 = token(&key, &d, "n-rep", 10_000_000_000);
        assert_eq!(v.verify(&d, &t2, 1_000), Err(ApprovalError::NonceReused));
    }

    #[test]
    fn llave_impostora_falla() {
        let real = sk(1);
        let impostor = sk(2);
        let v = ApprovalVerifier::new(Some(&hex::encode(real.verifying_key().to_bytes())), tmp_nonces())
            .unwrap();
        let d = desc();
        // Firmado por el impostor, verificado con la pública del real.
        let t = token(&impostor, &d, "n-imp", 10_000_000_000);
        assert_eq!(v.verify(&d, &t, 1_000), Err(ApprovalError::BadSignature));
    }

    #[test]
    fn sin_llave_falla() {
        let v = ApprovalVerifier::new(None, tmp_nonces()).unwrap();
        let key = sk(1);
        let d = desc();
        let t = token(&key, &d, "n-nokey", 10_000_000_000);
        assert_eq!(v.verify(&d, &t, 1_000), Err(ApprovalError::NoKey));
    }

    #[test]
    fn expirado_falla() {
        let key = sk(1);
        let v = ApprovalVerifier::new(Some(&hex::encode(key.verifying_key().to_bytes())), tmp_nonces())
            .unwrap();
        let d = desc();
        let t = token(&key, &d, "n-exp", 500);
        assert_eq!(v.verify(&d, &t, 1_000), Err(ApprovalError::Expired));
    }
}
