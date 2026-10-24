//! Log de auditoría encadenado.
//!
//! Cada operación se registra como una línea JSON con éxito o error. Cada
//! línea lleva `prev_hash` (el hash de la anterior) y su propio `hash`
//! (SHA-256 sobre el contenido de la entrada). Así, borrar o alterar
//! cualquier línea pasada rompe la cadena y se detecta con `verify_chain`.
//!
//! LÍMITE CONOCIDO: la cadena NO detecta que te trunquen la COLA (borrar las
//! últimas N líneas deja una cadena válida hasta donde quedó). Cuando el log
//! sea evidencia de cliente (fase 4+), hay que anclar el hash de cabeza fuera
//! del archivo (imprimirlo/replicarlo al cerrar cada corrida).
//!
//! El log nunca debe tumbar al servidor: si falla la escritura, se avisa por
//! stderr y la cadena no avanza (para no dejar un hash sin su línea).

use crate::util::{hex_encode, now_ms};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub struct Audit {
    path: PathBuf,
    prev_hash: RefCell<String>,
    seq: RefCell<u64>,
}

impl Audit {
    /// Abre el log y continúa la cadena desde la última línea existente.
    pub fn new(path: PathBuf) -> Self {
        let (prev_hash, seq) = Self::tail(&path);
        Audit {
            path,
            prev_hash: RefCell::new(prev_hash),
            seq: RefCell::new(seq),
        }
    }

    fn tail(path: &PathBuf) -> (String, u64) {
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return (GENESIS.to_string(), 0),
        };
        let mut last: Option<Value> = None;
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                last = Some(v);
            }
        }
        match last {
            Some(v) => {
                let h = v
                    .get("hash")
                    .and_then(|x| x.as_str())
                    .unwrap_or(GENESIS)
                    .to_string();
                let s = v.get("seq").and_then(|x| x.as_u64()).unwrap_or(0);
                (h, s + 1)
            }
            None => (GENESIS.to_string(), 0),
        }
    }

    pub fn record(&self, op: &str, args: &Value, status: &str, detail: &str) {
        let seq = *self.seq.borrow();
        let prev = self.prev_hash.borrow().clone();

        // Entrada SIN el campo `hash`; el hash se calcula sobre esto.
        let mut entry = json!({
            "seq": seq,
            "ts_ms": now_ms(),
            "op": op,
            "args": args,
            "status": status,
            "detail": detail,
            "prev_hash": prev,
        });

        // serde_json (sin preserve_order) ordena las claves → serialización
        // determinista; `verify_chain` reproduce exactamente estos bytes.
        let hash = entry_hash(&entry);
        entry["hash"] = Value::String(hash.clone());

        if let Err(e) = self.append(&entry.to_string()) {
            eprintln!("[auditoría] no se pudo escribir el log: {e}");
            return; // sin línea escrita, la cadena no avanza
        }
        *self.prev_hash.borrow_mut() = hash;
        *self.seq.borrow_mut() = seq + 1;
    }

    fn append(&self, line: &str) -> std::io::Result<()> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(f, "{line}")
    }
}

/// SHA-256 del objeto de entrada SIN el campo `hash`.
fn entry_hash(entry: &Value) -> String {
    let mut clone = entry.clone();
    if let Value::Object(ref mut m) = clone {
        m.remove("hash");
    }
    let mut hasher = Sha256::new();
    hasher.update(clone.to_string().as_bytes());
    hex_encode(&hasher.finalize())
}

/// Recorre el log y verifica el encadenamiento. Devuelve el número de registros
/// válidos, o error en el primer registro roto. Usado por los tests y por un
/// futuro binario `rua-verify`. (No detecta truncado de cola; ver cabecera.)
#[allow(dead_code)]
pub fn verify_chain(path: &Path) -> Result<usize, String> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Ok(0),
    };
    let mut prev = GENESIS.to_string();
    let mut n = 0usize;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        let stored = v
            .get("hash")
            .and_then(|x| x.as_str())
            .ok_or("línea sin hash")?
            .to_string();
        let ph = v
            .get("prev_hash")
            .and_then(|x| x.as_str())
            .ok_or("línea sin prev_hash")?
            .to_string();
        if ph != prev {
            return Err(format!("cadena rota en registro {n}: prev_hash no enlaza"));
        }
        if entry_hash(&v) != stored {
            return Err(format!("hash inválido en registro {n}"));
        }
        prev = stored;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static CTR: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> PathBuf {
        let mut p = std::env::temp_dir();
        let n = CTR.fetch_add(1, Ordering::SeqCst);
        p.push(format!("rua_audit_test_{}_{}.log", std::process::id(), n));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn cadena_valida() {
        let p = tmp();
        let a = Audit::new(p.clone());
        a.record("rua.start", &json!({"vm": "golden"}), "ok", "");
        a.record("rua.collect", &json!({"vm": "golden"}), "ok", "");
        assert_eq!(verify_chain(&p).unwrap(), 2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn detecta_alteracion() {
        let p = tmp();
        let a = Audit::new(p.clone());
        a.record("rua.start", &json!({"vm": "golden"}), "ok", "");
        a.record("rua.run", &json!({"vm": "golden"}), "ok", "");
        // Alterar la primera línea rompe la cadena.
        let content = std::fs::read_to_string(&p).unwrap();
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        lines[0] = lines[0].replace("\"golden\"", "\"otra\"");
        std::fs::write(&p, lines.join("\n") + "\n").unwrap();
        assert!(verify_chain(&p).is_err());
        let _ = std::fs::remove_file(&p);
    }
}
