//! rua-arnes: evalúa una traza del sink contra el catálogo de fase 2 (o un
//! escenario suelto) y emite un veredicto tri-estado por escenario más el
//! veredicto agregado de la corrida.
//!
//! Uso:
//!   rua-arnes --traza sink-eventos.jsonl [--escenario <nombre>] [--ahora-ms N]
//!
//! `--ahora-ms` fija el momento de evaluación (para medir silencio). Si se
//! omite, se usa el reloj del host.
//!
//! Salida: una línea por escenario evaluado y una final `CORRIDA: <veredicto>`.
//! Código de salida: 0 aprobada; 2 no concluyente; 3 fallo del agente;
//! 4 falso OK del lab. Así un CI puede distinguir "no sé" de "mal".

use rua_arnes::*;

struct Args {
    traza: String,
    escenario: Option<String>,
    ahora_ms: Option<u128>,
}

fn parse_args() -> Result<Args, String> {
    let mut traza = None;
    let mut escenario = None;
    let mut ahora_ms = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--traza" => traza = Some(it.next().ok_or("--traza requiere una ruta")?),
            "--escenario" => escenario = Some(it.next().ok_or("--escenario requiere un nombre")?),
            "--ahora-ms" => {
                ahora_ms = Some(
                    it.next()
                        .ok_or("--ahora-ms requiere un número")?
                        .parse()
                        .map_err(|_| "--ahora-ms no es numérico")?,
                )
            }
            otro => return Err(format!("argumento desconocido: {otro}")),
        }
    }
    Ok(Args {
        traza: traza.ok_or("falta --traza <ruta-jsonl>")?,
        escenario,
        ahora_ms,
    })
}

fn ahora_ms_reloj() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[rua-arnes] {e}");
            std::process::exit(1);
        }
    };

    let texto = match std::fs::read_to_string(&args.traza) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[rua-arnes] no se pudo leer '{}': {e}", args.traza);
            std::process::exit(1);
        }
    };
    let tr = match parse_jsonl(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[rua-arnes] traza inválida: {e}");
            std::process::exit(1);
        }
    };
    let ahora = args.ahora_ms.unwrap_or_else(ahora_ms_reloj);

    let catalogo = catalogo_fase2();
    let seleccion: Vec<&Escenario> = match &args.escenario {
        Some(n) => catalogo.iter().filter(|e| e.nombre == n).collect(),
        None => catalogo.iter().collect(),
    };
    if seleccion.is_empty() {
        eprintln!("[rua-arnes] escenario no encontrado en el catálogo");
        std::process::exit(1);
    }

    let mut veredictos = Vec::new();
    for e in &seleccion {
        let r = evaluar(&e.firma, &tr, ahora);
        let v = veredicto(e.clase, &r);
        println!(
            "{:<28} {:<5} {:<14} {}",
            e.nombre,
            match e.clase {
                Clase::Verde => "verde",
                Clase::Rojo => "rojo",
            },
            match v {
                Veredicto::Correcto => "CORRECTO",
                Veredicto::FalsoOkDelLab => "FALSO-OK-LAB",
                Veredicto::FalloDelAgente => "FALLO-AGENTE",
                Veredicto::NoConcluyente => "NO-MEDIBLE",
            },
            match &r {
                Resultado::Cumplida => String::new(),
                Resultado::NoCumplida(m) | Resultado::NoMedible(m) => format!("({m})"),
            }
        );
        veredictos.push(v);
    }

    let corrida = agregar(&veredictos);
    println!(
        "CORRIDA: {}",
        match corrida {
            Corrida::Aprobada => "APROBADA",
            Corrida::FalsoOkDelLab => "FALSO-OK-DEL-LAB",
            Corrida::FalloDelAgente => "FALLO-DEL-AGENTE",
            Corrida::NoConcluyente => "NO-CONCLUYENTE",
        }
    );
    std::process::exit(match corrida {
        Corrida::Aprobada => 0,
        Corrida::NoConcluyente => 2,
        Corrida::FalloDelAgente => 3,
        Corrida::FalsoOkDelLab => 4,
    });
}
