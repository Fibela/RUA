//! Despacho de operaciones cerradas y el gate de aprobación.
//!
//! Operaciones: list_vms, revert, start, poweroff, collect (no peligrosas) y
//! copy_to, run (peligrosas: entregan/ejecutan payload).
//!
//! REGLA CENTRAL DEL GATE: el campo `confirm` del cliente NO se lee en ningún
//! punto. La aprobación de una operación peligrosa solo puede venir de:
//!   - nivel bajo: imposible por construcción (se rechaza);
//!   - nivel alto: un token ed25519 verificado host-side (ver `approval`).

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::approval::{ApprovalToken, ApprovalVerifier, OperationDescriptor};
use crate::audit::{now_unix, AuditLog};
use crate::config::{RiskLevel, RuaConfig, VmEntry};
use crate::vbox::{CmdOutput, Vbox};

pub struct Ctx {
    pub cfg: RuaConfig,
    pub vbox: Vbox,
    pub audit: AuditLog,
    pub approval: ApprovalVerifier,
}

fn out_json(out: &CmdOutput) -> Value {
    json!({"status": out.status, "stdout": out.stdout, "stderr": out.stderr})
}

fn s(params: &Value, key: &str) -> Result<String, String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .ok_or_else(|| format!("falta parámetro '{key}'"))
}

fn str_array(params: &Value, key: &str) -> Vec<String> {
    params
        .get(key)
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn guest_creds(entry: &VmEntry, vm: &str) -> Result<(String, String), String> {
    let user = entry
        .guest_username
        .clone()
        .ok_or_else(|| format!("falta guest_username para '{vm}'"))?;
    let pass = entry
        .guest_password_file
        .clone()
        .ok_or_else(|| format!("falta guest_password_file para '{vm}'"))?;
    Ok((user, pass))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("leyendo payload {}: {e}", path.display()))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(hex::encode(h.finalize()))
}

fn parse_token(params: &Value) -> Result<ApprovalToken, String> {
    let t = params
        .get("approval_token")
        .ok_or("falta approval_token (requerido en nivel alto)")?;
    Ok(ApprovalToken {
        nonce: t
            .get("nonce")
            .and_then(|x| x.as_str())
            .ok_or("approval_token.nonce faltante")?
            .to_string(),
        expires_at: t
            .get("expires_at")
            .and_then(|x| x.as_i64())
            .ok_or("approval_token.expires_at faltante")?,
        signature_hex: t
            .get("signature_hex")
            .and_then(|x| x.as_str())
            .ok_or("approval_token.signature_hex faltante")?
            .to_string(),
    })
}

/// Gate de operación peligrosa. NO consulta `params["confirm"]`.
fn authorize_dangerous(
    ctx: &Ctx,
    desc: &OperationDescriptor,
    params: &Value,
) -> Result<(), String> {
    match ctx.cfg.risk_level {
        RiskLevel::Bajo => Err(
            "nivel bajo: entregar/ejecutar payload está deshabilitado por construcción".into(),
        ),
        RiskLevel::Alto => {
            if !ctx.cfg.allow_dangerous {
                return Err("allow_dangerous=false: operación peligrosa rechazada".into());
            }
            let token = parse_token(params)?;
            ctx.approval
                .verify(desc, &token, now_unix())
                .map_err(|e| format!("aprobación inválida: {e}"))
        }
    }
}

pub fn dispatch(ctx: &Ctx, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "rua.list_vms" => {
            let out = ctx.vbox.list_vms()?;
            Ok(out_json(&out))
        }

        "rua.revert" => {
            let vm = s(params, "vm")?;
            let snapshot = s(params, "snapshot")?;
            let entry = ctx.cfg.vm(&vm)?;
            if !entry.snapshots.iter().any(|x| x == &snapshot) {
                return Err(format!("snapshot '{snapshot}' no declarado para '{vm}'"));
            }
            ctx.audit.append("intent", &json!({"op":"revert","vm":vm,"snapshot":snapshot}))?;
            let out = ctx.vbox.snapshot_restore(&vm, &snapshot)?;
            ctx.audit.append("result", &json!({"op":"revert","vm":vm,"status":out.status}))?;
            Ok(out_json(&out))
        }

        "rua.start" => {
            let vm = s(params, "vm")?;
            ctx.cfg.vm(&vm)?;
            ctx.audit.append("intent", &json!({"op":"start","vm":vm}))?;
            let out = ctx.vbox.start_headless(&vm)?;
            ctx.audit.append("result", &json!({"op":"start","vm":vm,"status":out.status}))?;
            Ok(out_json(&out))
        }

        "rua.poweroff" => {
            let vm = s(params, "vm")?;
            ctx.cfg.vm(&vm)?;
            ctx.audit.append("intent", &json!({"op":"poweroff","vm":vm}))?;
            let out = ctx.vbox.poweroff(&vm)?;
            ctx.audit.append("result", &json!({"op":"poweroff","vm":vm,"status":out.status}))?;
            Ok(out_json(&out))
        }

        "rua.collect" => {
            let vm = s(params, "vm")?;
            let guest_src = s(params, "guest_src")?;
            let host_dest_dir = s(params, "host_dest_dir")?;
            let entry = ctx.cfg.vm(&vm)?;
            let (user, pass) = guest_creds(entry, &vm)?;
            ctx.audit.append("intent", &json!({"op":"collect","vm":vm,"guest_src":guest_src}))?;
            let out = ctx.vbox.copy_from(&vm, &user, &pass, &guest_src, &host_dest_dir)?;
            ctx.audit.append("result", &json!({"op":"collect","vm":vm,"status":out.status}))?;
            Ok(out_json(&out))
        }

        // ---- Operaciones peligrosas (gate de aprobación) ----
        "rua.copy_to" => {
            let vm = s(params, "vm")?;
            let source = s(params, "source")?;
            let target_dir = s(params, "target_dir")?;
            let entry = ctx.cfg.vm(&vm)?;
            let (user, pass) = guest_creds(entry, &vm)?;
            let payload_sha256 = sha256_file(Path::new(&source))?;
            let desc = OperationDescriptor {
                vm: vm.clone(),
                exe: source.clone(),
                args: vec![],
                payload_sha256,
            };
            authorize_dangerous(ctx, &desc, params)?;
            ctx.audit.append(
                "intent",
                &json!({"op":"copy_to","vm":vm,"source":source,"payload_sha256":desc.payload_sha256}),
            )?;
            let out = ctx.vbox.copy_to(&vm, &user, &pass, Path::new(&source), &target_dir)?;
            ctx.audit.append("result", &json!({"op":"copy_to","vm":vm,"status":out.status}))?;
            Ok(out_json(&out))
        }

        "rua.run" => {
            let vm = s(params, "vm")?;
            let exe = s(params, "exe")?;
            let args = str_array(params, "args");
            let entry = ctx.cfg.vm(&vm)?;
            let (user, pass) = guest_creds(entry, &vm)?;
            let desc = OperationDescriptor {
                vm: vm.clone(),
                exe: exe.clone(),
                args: args.clone(),
                payload_sha256: String::new(),
            };
            authorize_dangerous(ctx, &desc, params)?;
            ctx.audit.append("intent", &json!({"op":"run","vm":vm,"exe":exe,"args":args}))?;
            let out = ctx.vbox.run_guest(&vm, &user, &pass, &exe, &args)?;
            ctx.audit.append("result", &json!({"op":"run","vm":vm,"status":out.status}))?;
            Ok(out_json(&out))
        }

        other => Err(format!("método desconocido: {other}")),
    }
}
