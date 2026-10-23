//! rua-sink: colector local de syslog del laboratorio RUA.
//!
//! Escucha TCP en claro, recibe los marcos RFC 6587 del agente Eje-Latam, los
//! parsea (ver `rua_sink`) y los va anexando como JSONL a un archivo, para que
//! el arnés de validación afirme sobre lo recibido sin tocar ASC en la nube.
//!
//! Uso:
//!   rua-sink --listen 10.0.0.1:6514 --out sink-eventos.jsonl [--print]
//!
//! No hay dirección de escucha por omisión: igual que el agente no tiene puerto
//! por defecto, si no se declara `--listen`, el colector no arranca.

use rua_sink::{leer_todos, Evento};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Args {
    listen: String,
    out: String,
    print: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut listen: Option<String> = None;
    let mut out = String::from("sink-eventos.jsonl");
    let mut print = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--listen" => listen = Some(it.next().ok_or("--listen requiere HOST:PUERTO")?),
            "--out" => out = it.next().ok_or("--out requiere una ruta")?,
            "--print" => print = true,
            otro => return Err(format!("argumento desconocido: {otro}")),
        }
    }
    Ok(Args {
        listen: listen.ok_or("falta --listen HOST:PUERTO (no hay valor por omisión)")?,
        out,
        print,
    })
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[rua-sink] {e}");
            std::process::exit(1);
        }
    };

    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&args.out)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[rua-sink] no se pudo abrir '{}': {e}", args.out);
            std::process::exit(1);
        }
    };
    let out = Arc::new(Mutex::new(file));

    let listener = match TcpListener::bind(&args.listen) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[rua-sink] no se pudo escuchar en {}: {e}", args.listen);
            std::process::exit(1);
        }
    };
    eprintln!(
        "[rua-sink] escuchando en {} → {} (una conexión por mensaje)",
        args.listen, args.out
    );

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                let out = Arc::clone(&out);
                let print = args.print;
                std::thread::spawn(move || atender(stream, out, print));
            }
            Err(e) => eprintln!("[rua-sink] conexión fallida: {e}"),
        }
    }
}

fn atender(stream: TcpStream, out: Arc<Mutex<std::fs::File>>, print: bool) {
    // Timeout defensivo: un cliente lento no debe bloquear un hilo para siempre.
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let par = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();

    let (eventos, err) = leer_todos(stream);
    for ev in &eventos {
        anexar(&out, ev, print, &par);
    }
    if let Some(e) = err {
        // Un marco corrupto/rechazado se registra, no tumba el colector.
        eprintln!("[rua-sink] {par}: marco descartado: {e}");
    }
}

fn ahora_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn anexar(out: &Arc<Mutex<std::fs::File>>, ev: &Evento, print: bool, par: &str) {
    // Se envuelve el evento con metadatos de recepción: `recibido_ms` (reloj de
    // pared del host) permite al arnés medir SILENCIO contra `intervaloMs`
    // —la ausencia de latidos es lo que alerta (PA-105)—, no una cadena en el
    // cuerpo. `origen` es el par TCP que lo entregó.
    let registro = serde_json::json!({
        "recibido_ms": ahora_ms(),
        "origen": par,
        "evento": ev,
    });
    match serde_json::to_string(&registro) {
        Ok(linea) => {
            if let Ok(mut f) = out.lock() {
                let _ = writeln!(f, "{linea}");
                let _ = f.flush();
            }
            if print {
                eprintln!(
                    "[rua-sink] {par} {:?} msgid={} gravedad={} host={}",
                    ev.clase, ev.msgid, ev.gravedad, ev.host
                );
            }
        }
        Err(e) => eprintln!("[rua-sink] no se pudo serializar evento: {e}"),
    }
}
