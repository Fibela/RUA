//! rua-orquestador (CLI).
//!
//! Hoy: prueba de conexión con el núcleo (lista las VMs), que es lo único
//! ejecutable sin la golden. La máquina de escenarios vive en la lib
//! (`ejecutar`), lista para cuando exista `golden-eje-linux` y se declaren los
//! planes concretos (rutas del almacén, comando de arranque del agente).
//!
//! Uso: rua-orquestador --nucleo <ruta/rua-nucleo> --config <ruta/rua.config.json>

use rua_orquestador::NucleoStdio;
use rua_orquestador::Nucleo;
use serde_json::json;

fn main() {
    let mut exe = None;
    let mut config = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--nucleo" => exe = it.next(),
            "--config" => config = it.next(),
            otro => {
                eprintln!("[rua-orquestador] argumento desconocido: {otro}");
                std::process::exit(1);
            }
        }
    }
    let (exe, config) = match (exe, config) {
        (Some(e), Some(c)) => (e, c),
        _ => {
            eprintln!("[rua-orquestador] uso: --nucleo <ruta> --config <ruta>");
            std::process::exit(1);
        }
    };

    let mut n = match NucleoStdio::lanzar(&exe, &config) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("[rua-orquestador] {e}");
            std::process::exit(1);
        }
    };
    match n.tool("rua.list_vms", json!({})) {
        Ok(v) => {
            println!("conexión con el núcleo OK. VMs:");
            println!("{}", v.get("stdout").and_then(|x| x.as_str()).unwrap_or("(vacío)"));
        }
        Err(e) => {
            eprintln!("[rua-orquestador] list_vms falló: {e}");
            std::process::exit(1);
        }
    }
}
