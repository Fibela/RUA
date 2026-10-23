//! Log de auditoría encadenado (tamper-evident).
//!
//! Cada línea es un objeto JSON: {prev_hash, ts_unix, kind, detail, hash}.
//! `hash = SHA-256(prev_hash \n ts \n kind \n detail_json)`, y `prev_hash` es
//! el `hash` de la línea anterior (GENESIS en la primera). Alterar o borrar una
//! línea intermedia rompe la cadena y `verify_chain` lo detecta.
//!
//! LÍMITE CONOCIDO: la cadena NO detecta que te trunquen la cola (borrar las
//! últimas N líneas deja una cadena válida hasta donde quedó). Cuando el log
//! sea evidencia de cliente (fase 4+), hay que anclar el hash de cabeza fuera
//! del archivo.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub struct AuditLog {
    path: PathBuf,
}

pub fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn compute_hash(prev: &str, ts: i64, kind: &str, detail_json: &str) -> String {
    let mut h = Sha256::new();
    h.update(prev.as_bytes());
    h.update(b"\n");
    h.update(ts.to_string().as_bytes());
    h.update(b"\n");
    h.update(kind.as_bytes());
    h.update(b"\n");
    h.update(detail_json.as_bytes());
    hex::encode(h.finalize())
}

impl AuditLog {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn last_hash(&self) -> Result<String, String> {
        if !self.path.exists() {
            return Ok("GENESIS".to_string());
        }
        let f = std::fs::File::open(&self.path).map_err(|e| e.to_string())?;
        let mut last = "GENESIS".to_string();
        for line in BufReader::new(f).lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            if let Some(h) = v.get("hash").and_then(|h| h.as_str()) {
                last = h.to_string();
            }
        }
        Ok(last)
    }

    /// Escribe un registro y devuelve su hash. `kind` suele ser "intent" o
    /// "result". `detail` es un objeto JSON libre con los datos de la operación.
    pub fn append(&self, kind: &str, detail: &Value) -> Result<String, String> {
        let prev = self.last_hash()?;
        let ts = now_unix();
        let detail_json = serde_json::to_string(detail).map_err(|e| e.to_string())?;
        let hash = compute_hash(&prev, ts, kind, &detail_json);
        let line = json!({
            "prev_hash": prev,
            "ts_unix": ts,
            "kind": kind,
            "detail": detail,
            "hash": hash,
        });
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        writeln!(f, "{}", serde_json::to_string(&line).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(hash)
    }
}

/// Recorre el log y verifica el encadenamiento. Devuelve el número de registros
/// válidos, o error en el primer registro roto. (Usado por los tests y, más
/// adelante, por un binario `rua-verify`.)
#[allow(dead_code)]
pub fn verify_chain(path: &Path) -> Result<usize, String> {
    if !path.exists() {
        return Ok(0);
    }
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut prev = "GENESIS".to_string();
    let mut n = 0usize;
    for line in BufReader::new(f).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        let ph = v.get("prev_hash").and_then(|x| x.as_str()).ok_or("línea sin prev_hash")?;
        let ts = v.get("ts_unix").and_then(|x| x.as_i64()).ok_or("línea sin ts_unix")?;
        let kind = v.get("kind").and_then(|x| x.as_str()).ok_or("línea sin kind")?;
        let detail = v.get("detail").ok_or("línea sin detail")?;
        let stored = v.get("hash").and_then(|x| x.as_str()).ok_or("línea sin hash")?;
        if ph != prev {
            return Err(format!("cadena rota en registro {n}: prev_hash no coincide"));
        }
        let detail_json = serde_json::to_string(detail).map_err(|e| e.to_string())?;
        if compute_hash(ph, ts, kind, &detail_json) != stored {
            return Err(format!("hash inválido en registro {n}"));
        }
        prev = stored.to_string();
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tmp(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("rua_audit_{}_{}.log", tag, now_unix()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn cadena_valida() {
        let p = tmp("ok");
        let log = AuditLog::new(p.clone());
        log.append("intent", &json!({"op":"run","vm":"golden"})).unwrap();
        log.append("result", &json!({"op":"run","status":0})).unwrap();
        assert_eq!(verify_chain(&p).unwrap(), 2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn detecta_alteracion() {
        let p = tmp("tamper");
        let log = AuditLog::new(p.clone());
        log.append("intent", &json!({"op":"revert"})).unwrap();
        log.append("result", &json!({"op":"revert"})).unwrap();
        // Alteramos la primera línea.
        let content = std::fs::read_to_string(&p).unwrap();
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        lines[0] = lines[0].replace("revert", "arrancar");
        std::fs::write(&p, lines.join("\n") + "\n").unwrap();
        assert!(verify_chain(&p).is_err());
        let _ = std::fs::remove_file(&p);
    }
}
