//! rua-nucleo: servidor MCP determinista sobre VBoxManage.
//!
//! Habla MCP por stdio (JSON-RPC 2.0, un mensaje JSON por línea). Expone un
//! conjunto cerrado de operaciones sobre VMs de VirtualBox. No hay IA aquí:
//! esto solo ejecuta comandos fijos de VBoxManage y registra todo.
//!
//! Candado de ejecución (en `handle_call`, operación `rua.run`): el host —no el
//! cliente— decide qué es peligroso. Un programa DECLARADO en `allowed_programs`
//! de la VM se ejecuta en cualquier nivel (p. ej. el agente de Eje-Latam). Un
//! programa NO declarado es peligroso: en nivel bajo se rechaza; en nivel alto
//! exige `allow_dangerous` + un token firmado por el aprobador del host
//! (ver approval.rs). El cliente NO tiene ningún flag que decida esto.

mod approval;
mod audit;
mod config;
mod error;
mod transporte;
mod util;
mod vbox;

use approval::Approvals;
use audit::Audit;
use config::{Config, RiskLevel};
use error::NucleoError;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use vbox::Vbox;

/// Versión del protocolo MCP que anunciamos en `initialize`.
const PROTOCOL_VERSION: &str = "2025-06-18";

fn main() {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rua-nucleo] error de configuración: {e}");
            std::process::exit(1);
        }
    };

    // Verificador de aprobaciones host-side (clave pública del aprobador +
    // persistencia de nonces consumidos).
    let approvals = match Approvals::new(&cfg.approval_pubkey_hex, cfg.consumed_nonces_path.clone()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[rua-nucleo] error de aprobación: {e}");
            std::process::exit(1);
        }
    };

    let audit = Audit::new(cfg.audit_log.clone());
    audit.record(
        "startup",
        &json!({ "risk_level": format!("{:?}", cfg.risk_level) }),
        "ok",
        "núcleo iniciado",
    );
    eprintln!("[rua-nucleo] listo (nivel: {:?})", cfg.risk_level);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                write_message(&mut out, &rpc_error(Value::Null, -32700, &format!("JSON inválido: {e}")));
                continue;
            }
        };

        // Sin "id" es una notificación: no se responde.
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        match method {
            "initialize" => {
                let result = json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "rua-nucleo", "version": env!("CARGO_PKG_VERSION") }
                });
                reply(&mut out, id, result);
            }
            "notifications/initialized" => { /* notificación, sin respuesta */ }
            "ping" => reply(&mut out, id, json!({})),
            "tools/list" => reply(&mut out, id, json!({ "tools": tool_list() })),
            "tools/call" => match handle_call(&cfg, &approvals, &audit, &req) {
                Ok(text) => reply(&mut out, id, tool_ok(&text)),
                Err(e) => reply(&mut out, id, tool_err(&e.to_string())),
            },
            other => {
                if let Some(id) = id {
                    write_message(&mut out, &rpc_error(id, -32601, &format!("método no soportado: {other}")));
                }
            }
        }
    }
}

/// Despacha una llamada `tools/call` a la operación correspondiente y la
/// registra en el log, con éxito o con error.
fn handle_call(
    cfg: &Config,
    approvals: &Approvals,
    audit: &Audit,
    req: &Value,
) -> Result<String, NucleoError> {
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let vbox = Vbox::new(cfg);

    // Lector de argumento string obligatorio.
    let s = |k: &str| -> Result<String, NucleoError> {
        args.get(k)
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
            .ok_or_else(|| NucleoError::Args(format!("falta el argumento '{k}'")))
    };

    let outcome = (|| -> Result<String, NucleoError> {
        match name.as_str() {
            "rua.list_vms" => vbox.list_vms(),

            "rua.snapshot_restore" => vbox.snapshot_restore(&s("vm")?, &s("snapshot")?),

            "rua.start" => {
                let headless = args.get("headless").and_then(|v| v.as_bool()).unwrap_or(true);
                vbox.start(&s("vm")?, headless)
            }

            "rua.stop" => {
                let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
                vbox.stop(&s("vm")?, force)
            }

            "rua.copy_in" => vbox.copy_in(
                &s("vm")?,
                &s("host_path")?,
                &s("guest_dir")?,
                &s("guest_user")?,
                &s("password_file")?,
            ),

            "rua.collect" => vbox.collect(
                &s("vm")?,
                &s("guest_path")?,
                &s("host_dir")?,
                &s("guest_user")?,
                &s("password_file")?,
            ),

            "rua.copy_in_verified" => {
                let vm = s("vm")?;
                let entry = cfg
                    .vm(&vm)
                    .ok_or_else(|| NucleoError::VmNoPermitida(vm.clone()))?;
                let v = transporte::copy_in_verified(
                    &vbox,
                    entry,
                    &vm,
                    &s("host_path")?,
                    &s("guest_dir")?,
                    &s("guest_user")?,
                    &s("password_file")?,
                )?;
                Ok(v.to_string())
            }

            "rua.run" => {
                let vm = s("vm")?;
                let program = s("program")?;
                let prog_args: Vec<String> = args
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();

                // --- Candado de ejecución (host-side, sin flag del cliente) ---
                // La VM debe estar en la allowlist; el carácter peligroso lo
                // decide si el programa está DECLARADO para esa VM, nunca el
                // cliente. Declarado -> libre. No declarado -> peligroso: bajo
                // rechaza; alto exige allow_dangerous + token firmado.
                let entry = cfg
                    .vm(&vm)
                    .ok_or_else(|| NucleoError::VmNoPermitida(vm.clone()))?;
                let declared = entry.allowed_programs.iter().any(|p| p == &program);
                if !declared {
                    match cfg.risk_level {
                        RiskLevel::Bajo => {
                            return Err(NucleoError::Rechazada(format!(
                                "programa no declarado para '{vm}': en nivel BAJO solo se ejecutan los de allowed_programs"
                            )));
                        }
                        RiskLevel::Alto => {
                            if !cfg.allow_dangerous {
                                return Err(NucleoError::Rechazada(
                                    "programa no declarado y allow_dangerous está desactivado".into(),
                                ));
                            }
                            let token = s("approval_token")?;
                            approvals.verify(&vm, &program, &prog_args, &token)?;
                        }
                    }
                }

                vbox.run_guest(&vm, &program, &prog_args, &s("guest_user")?, &s("password_file")?)
            }

            other => Err(NucleoError::Args(format!("herramienta desconocida: {other}"))),
        }
    })();

    match &outcome {
        Ok(_) => audit.record(&name, &args, "ok", ""),
        Err(e) => audit.record(&name, &args, "error", &e.to_string()),
    }
    outcome
}

/// Catálogo de herramientas que anuncia el servidor en `tools/list`.
fn tool_list() -> Vec<Value> {
    let vm = || json!({ "type": "string", "description": "Nombre de la VM (debe estar en la allowlist)" });
    let guest_user = || json!({ "type": "string", "description": "Usuario dentro de la VM" });
    let password_file =
        || json!({ "type": "string", "description": "Ruta a un archivo con la contraseña (no la contraseña en claro)" });

    vec![
        json!({
            "name": "rua.list_vms",
            "description": "Lista las VMs registradas en VirtualBox.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rua.snapshot_restore",
            "description": "Revierte una VM a un snapshot declarado (p. ej. un estado de postura).",
            "inputSchema": {
                "type": "object",
                "properties": { "vm": vm(), "snapshot": { "type": "string" } },
                "required": ["vm", "snapshot"]
            }
        }),
        json!({
            "name": "rua.start",
            "description": "Arranca una VM (headless por defecto).",
            "inputSchema": {
                "type": "object",
                "properties": { "vm": vm(), "headless": { "type": "boolean", "default": true } },
                "required": ["vm"]
            }
        }),
        json!({
            "name": "rua.stop",
            "description": "Apaga una VM. ACPI por defecto; force=true usa poweroff.",
            "inputSchema": {
                "type": "object",
                "properties": { "vm": vm(), "force": { "type": "boolean", "default": false } },
                "required": ["vm"]
            }
        }),
        json!({
            "name": "rua.copy_in",
            "description": "Copia un archivo del host a un directorio dentro de la VM.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm": vm(),
                    "host_path": { "type": "string" },
                    "guest_dir": { "type": "string" },
                    "guest_user": guest_user(),
                    "password_file": password_file()
                },
                "required": ["vm", "host_path", "guest_dir", "guest_user", "password_file"]
            }
        }),
        json!({
            "name": "rua.collect",
            "description": "Recolecta un archivo/carpeta de la VM hacia un directorio del host.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm": vm(),
                    "guest_path": { "type": "string" },
                    "host_dir": { "type": "string" },
                    "guest_user": guest_user(),
                    "password_file": password_file()
                },
                "required": ["vm", "guest_path", "host_dir", "guest_user", "password_file"]
            }
        }),
        json!({
            "name": "rua.copy_in_verified",
            "description": "Copia un archivo al guest y DEMUESTRA que llegó íntegro (regla PA-12): hashea en destino y compara con el SHA-256 del origen. Devuelve un veredicto tri-estado: Verificado / NoCoincide / NoMedible (no hubo con qué medir). Nunca instalar sin un Verificado.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm": vm(),
                    "host_path": { "type": "string" },
                    "guest_dir": { "type": "string" },
                    "guest_user": guest_user(),
                    "password_file": password_file()
                },
                "required": ["vm", "host_path", "guest_dir", "guest_user", "password_file"]
            }
        }),
        json!({
            "name": "rua.run",
            "description": "Ejecuta un programa dentro de la VM. Si el programa NO está en allowed_programs de la VM, es peligroso: prohibido en nivel bajo; en nivel alto exige allow_dangerous + un approval_token firmado por el aprobador del host. El cliente no decide si es peligroso.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm": vm(),
                    "program": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } },
                    "guest_user": guest_user(),
                    "password_file": password_file(),
                    "approval_token": { "type": "string", "description": "Token host-side; solo se usa si el programa no está declarado y el nivel es alto. El cliente no puede generarlo." }
                },
                "required": ["vm", "program", "guest_user", "password_file"]
            }
        }),
    ]
}

// --- Helpers JSON-RPC ---

fn write_message<W: Write>(out: &mut W, msg: &Value) {
    let _ = writeln!(out, "{msg}");
    let _ = out.flush();
}

fn reply<W: Write>(out: &mut W, id: Option<Value>, result: Value) {
    if let Some(id) = id {
        write_message(out, &json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tool_ok(text: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": text } ] })
}

fn tool_err(msg: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": msg } ], "isError": true })
}
