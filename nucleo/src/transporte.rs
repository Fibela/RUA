//! Transporte verificado (regla PA-12): copiar un fichero al guest y DEMOSTRAR
//! que llegó íntegro, no confiar en el código de salida de la copia.
//!
//! Falla que esto previene: el puente WSL↔Windows entregó una vez el binario de
//! la semana anterior y `scp` reportó éxito. Regla innegociable del equipo:
//! tras cada transferencia, hash en destino y comparar con origen.
//!
//! Tri-estado deliberado (y su matiz, pedido por el equipo):
//!   - `Verificado`  — el hash del guest coincide con el del host.
//!   - `NoCoincide`  — se midió en ambos lados y NO coincide (fichero mal).
//!   - `NoMedible`   — no hubo con qué medir: la copia falló, el hasher del
//!                     guest no está o no se pudo ejecutar, o su salida no trae
//!                     un hash legible. "No hay con qué medir" ≠ "no cuadra":
//!                     colapsarlos dejaría al laboratorio ciego sin saberlo.
//!
//! El hasher lo fija el host en la config de la VM (`guest_hasher`), nunca el
//! cliente, así que esta ejecución interna no pasa por el candado de programas.

use crate::config::VmEntry;
use crate::error::NucleoError;
use crate::util::hex_encode;
use crate::vbox::Vbox;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq, Eq)]
pub enum Veredicto {
    Verificado,
    NoCoincide,
    NoMedible,
}

impl Veredicto {
    fn as_str(&self) -> &'static str {
        match self {
            Veredicto::Verificado => "Verificado",
            Veredicto::NoCoincide => "NoCoincide",
            Veredicto::NoMedible => "NoMedible",
        }
    }
}

/// SHA-256 de un fichero del host (lado origen de la comparación).
fn sha256_file(path: &str) -> Result<String, NucleoError> {
    let bytes = std::fs::read(path)
        .map_err(|e| NucleoError::Io(format!("no se pudo leer el origen '{path}': {e}")))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(hex_encode(&h.finalize()))
}

/// Nombre base de una ruta del host (tras el último `/` o `\`).
pub fn basename(host_path: &str) -> &str {
    host_path
        .rsplit(|c| c == '/' || c == '\\')
        .next()
        .unwrap_or(host_path)
}

/// Extrae un SHA-256 (64 hex) de la salida de un hasher, tolerando el formato
/// de `sha256sum` (`<hash>  ruta`) y el de `certutil` (línea de 64 hex, con o
/// sin espacios entre bytes). Devuelve el hash en minúsculas.
pub fn parse_hash(salida: &str) -> Option<String> {
    let es_hex64 = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
    // 1) Algún token separado por espacios que sea 64 hex (cubre sha256sum).
    for tok in salida.split_whitespace() {
        if es_hex64(tok) {
            return Some(tok.to_ascii_lowercase());
        }
    }
    // 2) Alguna línea que, quitados los espacios, sea 64 hex (cubre certutil).
    for linea in salida.lines() {
        let limpia: String = linea.chars().filter(|c| !c.is_whitespace()).collect();
        if es_hex64(&limpia) {
            return Some(limpia.to_ascii_lowercase());
        }
    }
    None
}

/// Decide el veredicto a partir del hash de origen y del hash del guest (si lo
/// hubo). Sin hash del guest → NoMedible (no "no coincide").
pub fn veredicto(host_sha: &str, guest_sha: Option<&str>) -> Veredicto {
    match guest_sha {
        None => Veredicto::NoMedible,
        Some(g) if g.eq_ignore_ascii_case(host_sha) => Veredicto::Verificado,
        Some(_) => Veredicto::NoCoincide,
    }
}

/// Copia `host_path` al `guest_dir` del guest y verifica por hash en destino.
/// Devuelve un objeto JSON con el veredicto y los hashes. Solo devuelve `Err`
/// cuando no se puede ni empezar (origen ilegible); todo lo demás es un
/// veredicto medido (incluido NoMedible).
#[allow(clippy::too_many_arguments)]
pub fn copy_in_verified(
    vbox: &Vbox,
    entry: &VmEntry,
    vm: &str,
    host_path: &str,
    guest_dir: &str,
    user: &str,
    password_file: &str,
) -> Result<Value, NucleoError> {
    let host_sha = sha256_file(host_path)?;

    let armar = |ver: Veredicto, guest_sha: Option<String>, detalle: String| -> Value {
        let dest = format!("{}{}{}", guest_dir, entry.guest_sep, basename(host_path));
        json!({
            "op": "copy_in_verified",
            "veredicto": ver.as_str(),
            "host_sha256": host_sha,
            "guest_sha256": guest_sha,
            "guest_dest": dest,
            "detalle": detalle,
        })
    };

    // 1) Copiar. Si falla, NoMedible: el fichero quizá no llegó.
    // Ruta destino COMPLETA en el guest (dir + sep + nombre base).
    let dest = format!("{}{}{}", guest_dir, entry.guest_sep, basename(host_path));

    if let Err(e) = vbox.copy_in(vm, host_path, &dest, user, password_file) {
        return Ok(armar(Veredicto::NoMedible, None, format!("copyto falló: {e}")));
    }

    // 2) Hashear en el guest sobre la ruta destino.
    if entry.guest_hasher.is_empty() {
        return Ok(armar(Veredicto::NoMedible, None, "guest_hasher vacío en la config".into()));
    }
    let programa = &entry.guest_hasher[0];
    let args: Vec<String> = entry.guest_hasher[1..]
        .iter()
        .map(|a| a.replace("{path}", &dest))
        .collect();

    let salida = match vbox.run_guest(vm, programa, &args, user, password_file) {
        Ok(s) => s,
        Err(e) => {
            // Hasher ausente o no ejecutable → NoMedible, NO NoCoincide.
            return Ok(armar(
                Veredicto::NoMedible,
                None,
                format!("el hasher del guest no se pudo ejecutar: {e}"),
            ));
        }
    };

    // 3) Parsear y comparar.
    match parse_hash(&salida) {
        None => Ok(armar(
            Veredicto::NoMedible,
            None,
            "la salida del hasher no contiene un SHA-256 legible".into(),
        )),
        Some(g) => {
            let ver = veredicto(&host_sha, Some(&g));
            let detalle = match ver {
                Veredicto::Verificado => "hash de destino coincide con el de origen".into(),
                Veredicto::NoCoincide => "hash de destino DISTINTO al de origen".into(),
                Veredicto::NoMedible => unreachable!(),
            };
            Ok(armar(ver, Some(g), detalle))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basename_unix_y_windows() {
        assert_eq!(basename("/home/eje/agente"), "agente");
        assert_eq!(basename("C:\\Users\\Alexx\\agente.exe"), "agente.exe");
        assert_eq!(basename("sinseparador"), "sinseparador");
    }

    #[test]
    fn parse_hash_sha256sum() {
        let h = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let salida = format!("{h}  /ruta/al/fichero");
        assert_eq!(parse_hash(&salida).unwrap(), h);
    }

    #[test]
    fn parse_hash_certutil_con_espacios() {
        // certutil puede imprimir el hash con espacios entre bytes.
        let salida = "SHA256 hash of file.bin:\nab cd ef 01 23 45 67 89 ab cd ef 01 23 45 67 89 ab cd ef 01 23 45 67 89 ab cd ef 01 23 45 67 89\nCertUtil: -hashfile command completed successfully.";
        let h = parse_hash(salida).unwrap();
        assert_eq!(h, "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
    }

    #[test]
    fn parse_hash_sin_hash() {
        assert!(parse_hash("sha256sum: no existe el fichero").is_none());
    }

    #[test]
    fn veredicto_tres_estados() {
        let host = "aa".repeat(32);
        assert_eq!(veredicto(&host, Some(&host)), Veredicto::Verificado);
        assert_eq!(veredicto(&host, Some(&"bb".repeat(32))), Veredicto::NoCoincide);
        assert_eq!(veredicto(&host, None), Veredicto::NoMedible);
    }

    #[test]
    fn veredicto_ignora_mayusculas() {
        let host = "abcd".repeat(16); // 64 chars
        let guest = host.to_ascii_uppercase();
        assert_eq!(veredicto(&host, Some(&guest)), Veredicto::Verificado);
    }
}
