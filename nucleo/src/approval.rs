//! Aprobación de operaciones peligrosas, FUERA del canal MCP.
//!
//! Una ejecución peligrosa (un programa no declarado) necesita un token firmado
//! por el aprobador del host, que tiene la llave privada ed25519. El núcleo solo
//! tiene la llave pública, así que el cliente MCP —la IA— no puede forjarlo:
//! aunque lea la config, no puede firmar. La decisión de aprobar nunca entra
//! por el mismo canal que la petición.
//!
//! El token liga el permiso a ESTA petición (vm + programa + argumentos), trae
//! un nonce de un solo uso y una expiración. El binario aprobador (rua-approve)
//! que acuña estos tokens se construye en la fase 5; aquí queda cableada la
//! verificación.
//!
//! Anti-replay: los nonces consumidos se PERSISTEN a disco, así que un token no
//! puede reutilizarse ni siquiera reiniciando el núcleo dentro de su ventana de
//! expiración.
//!
//! Formato del token:  hex(payload_json) "." hex(firma_64)

use crate::error::NucleoError;
use crate::util::{hex_decode, hex_encode, now_ms};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;

/// Lo que el token afirma. Liga el permiso a una petición concreta.
#[derive(Debug, Deserialize)]
struct Payload {
    vm: String,
    program: String,
    args_hash: String,
    nonce: String,
    exp_ms: u64,
}

pub struct Approvals {
    pubkey: Option<VerifyingKey>,
    nonces_path: PathBuf,
    seen_nonces: RefCell<HashSet<String>>,
}

impl Approvals {
    /// Construye desde la clave pública hex (32 bytes) de la config, si la hay,
    /// y la ruta donde se persisten los nonces consumidos.
    pub fn new(pubkey_hex: &Option<String>, nonces_path: PathBuf) -> Result<Self, String> {
        let pubkey = match pubkey_hex {
            None => None,
            Some(hx) => {
                let bytes = hex_decode(hx).ok_or("approval_pubkey_hex no es hex válido")?;
                let arr: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| "approval_pubkey_hex debe ser de 32 bytes (64 hex)")?;
                let vk = VerifyingKey::from_bytes(&arr)
                    .map_err(|e| format!("clave pública inválida: {e}"))?;
                Some(vk)
            }
        };
        let seen = load_nonces(&nonces_path);
        Ok(Approvals {
            pubkey,
            nonces_path,
            seen_nonces: RefCell::new(seen),
        })
    }

    /// Verifica un token contra la petición. Un error significa que la
    /// ejecución peligrosa se rechaza. Si pasa, consume el nonce (en memoria y
    /// en disco) antes de devolver Ok.
    pub fn verify(
        &self,
        vm: &str,
        program: &str,
        prog_args: &[String],
        token: &str,
    ) -> Result<(), NucleoError> {
        let pubkey = self
            .pubkey
            .as_ref()
            .ok_or_else(|| NucleoError::Aprobacion("no hay clave de aprobación configurada".into()))?;

        let (payload_hex, sig_hex) = token
            .split_once('.')
            .ok_or_else(|| NucleoError::Aprobacion("token mal formado".into()))?;
        let payload_bytes =
            hex_decode(payload_hex).ok_or_else(|| NucleoError::Aprobacion("payload no es hex".into()))?;
        let sig_bytes =
            hex_decode(sig_hex).ok_or_else(|| NucleoError::Aprobacion("firma no es hex".into()))?;
        let sig_arr: [u8; 64] = sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| NucleoError::Aprobacion("la firma no mide 64 bytes".into()))?;
        let signature = Signature::from_bytes(&sig_arr);

        // 1) Firma válida bajo la clave pública del host (sobre los bytes
        //    crudos del payload, sin reserializar: sin desajuste de forma).
        pubkey
            .verify(&payload_bytes, &signature)
            .map_err(|_| NucleoError::Aprobacion("firma inválida".into()))?;

        // 2) El payload liga a ESTA petición.
        let payload: Payload = serde_json::from_slice(&payload_bytes)
            .map_err(|e| NucleoError::Aprobacion(format!("payload inválido: {e}")))?;

        if now_ms() > payload.exp_ms {
            return Err(NucleoError::Aprobacion("token expirado".into()));
        }
        if payload.vm != vm || payload.program != program {
            return Err(NucleoError::Aprobacion("el token no corresponde a esta petición".into()));
        }
        if payload.args_hash != args_hash(prog_args) {
            return Err(NucleoError::Aprobacion("los argumentos no coinciden con el token".into()));
        }

        // 3) Anti-replay persistente: el nonce se usa una sola vez. Se verifica
        //    contra el conjunto en memoria (cargado del disco al arrancar) y se
        //    graba antes de aceptar.
        {
            let seen = self.seen_nonces.borrow();
            if seen.contains(&payload.nonce) {
                return Err(NucleoError::Aprobacion("token ya usado (nonce repetido)".into()));
            }
        }
        self.seen_nonces.borrow_mut().insert(payload.nonce.clone());
        if let Err(e) = save_nonces(&self.nonces_path, &self.seen_nonces.borrow()) {
            // Si no podemos persistir el nonce, deshacemos y rechazamos: es
            // preferible negar una detonación a permitir un replay silencioso.
            self.seen_nonces.borrow_mut().remove(&payload.nonce);
            return Err(NucleoError::Aprobacion(format!(
                "no se pudo persistir el nonce (anti-replay): {e}"
            )));
        }

        Ok(())
    }
}

fn load_nonces(path: &PathBuf) -> HashSet<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

fn save_nonces(path: &PathBuf, set: &HashSet<String>) -> Result<(), String> {
    let v: Vec<&String> = set.iter().collect();
    let text = serde_json::to_string(&v).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Hash canónico de los argumentos del programa, para ligar el token a ellos.
pub fn args_hash(prog_args: &[String]) -> String {
    let v = serde_json::json!(prog_args);
    let mut h = Sha256::new();
    h.update(v.to_string().as_bytes());
    hex_encode(&h.finalize())
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
        p.push(format!("rua_nonces_test_{}_{}.json", std::process::id(), n));
        let _ = std::fs::remove_file(&p);
        p
    }

    fn mint(
        sk: &SigningKey,
        vm: &str,
        program: &str,
        prog_args: &[String],
        nonce: &str,
        exp_ms: u64,
    ) -> String {
        let payload = serde_json::json!({
            "vm": vm,
            "program": program,
            "args_hash": args_hash(prog_args),
            "nonce": nonce,
            "exp_ms": exp_ms,
        });
        let pbytes = payload.to_string().into_bytes();
        let sig = sk.sign(&pbytes);
        format!("{}.{}", hex_encode(&pbytes), hex_encode(&sig.to_bytes()))
    }

    #[test]
    fn token_valido_pasa() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let ap = Approvals::new(&Some(hex_encode(sk.verifying_key().as_bytes())), tmp_nonces()).unwrap();
        let args = vec!["muestra.exe".to_string()];
        let tok = mint(&sk, "golden", "det.exe", &args, "n1", now_ms() + 60_000);
        assert!(ap.verify("golden", "det.exe", &args, &tok).is_ok());
    }

    #[test]
    fn vm_distinta_falla() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let ap = Approvals::new(&Some(hex_encode(sk.verifying_key().as_bytes())), tmp_nonces()).unwrap();
        let args = vec!["muestra.exe".to_string()];
        let tok = mint(&sk, "golden", "det.exe", &args, "n2", now_ms() + 60_000);
        assert!(ap.verify("otra-vm", "det.exe", &args, &tok).is_err());
    }

    #[test]
    fn nonce_repetido_falla() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let ap = Approvals::new(&Some(hex_encode(sk.verifying_key().as_bytes())), tmp_nonces()).unwrap();
        let args = vec!["muestra.exe".to_string()];
        let tok = mint(&sk, "golden", "det.exe", &args, "n3", now_ms() + 60_000);
        assert!(ap.verify("golden", "det.exe", &args, &tok).is_ok());
        let tok2 = mint(&sk, "golden", "det.exe", &args, "n3", now_ms() + 60_000);
        assert!(ap.verify("golden", "det.exe", &args, &tok2).is_err());
    }

    #[test]
    fn nonce_persiste_tras_reinicio() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let path = tmp_nonces();
        let pk = hex_encode(sk.verifying_key().as_bytes());
        let args = vec!["muestra.exe".to_string()];
        let tok = mint(&sk, "golden", "det.exe", &args, "npersist", now_ms() + 600_000);
        // Primer "proceso": consume el nonce.
        {
            let ap = Approvals::new(&Some(pk.clone()), path.clone()).unwrap();
            assert!(ap.verify("golden", "det.exe", &args, &tok).is_ok());
        }
        // Segundo "proceso": recarga del disco y debe rechazar el replay.
        {
            let ap = Approvals::new(&Some(pk), path.clone()).unwrap();
            let tok2 = mint(&sk, "golden", "det.exe", &args, "npersist", now_ms() + 600_000);
            assert!(ap.verify("golden", "det.exe", &args, &tok2).is_err());
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn otra_llave_falla() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let impostor = SigningKey::from_bytes(&[9u8; 32]);
        let ap = Approvals::new(&Some(hex_encode(sk.verifying_key().as_bytes())), tmp_nonces()).unwrap();
        let args = vec!["muestra.exe".to_string()];
        let tok = mint(&impostor, "golden", "det.exe", &args, "n4", now_ms() + 60_000);
        assert!(ap.verify("golden", "det.exe", &args, &tok).is_err());
    }

    #[test]
    fn sin_llave_rechaza() {
        let ap = Approvals::new(&None, tmp_nonces()).unwrap();
        let args = vec!["muestra.exe".to_string()];
        assert!(ap.verify("golden", "det.exe", &args, "lo.que.sea").is_err());
    }

    #[test]
    fn expirado_falla() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let ap = Approvals::new(&Some(hex_encode(sk.verifying_key().as_bytes())), tmp_nonces()).unwrap();
        let args = vec!["muestra.exe".to_string()];
        let tok = mint(&sk, "golden", "det.exe", &args, "nexp", now_ms().saturating_sub(1));
        assert!(ap.verify("golden", "det.exe", &args, &tok).is_err());
    }
}
