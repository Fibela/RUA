//! RUA — núcleo determinista (fase 1).
//!
//! Wrapper de VBoxManage expuesto como servidor JSON-RPC/MCP por stdio, con
//! operaciones cerradas, allowlist de VMs, snapshots declarados, log de
//! auditoría encadenado y gate de aprobación host-side (token ed25519).
//!
//! Uso:  rua-nucleo [ruta-config]   (por defecto: rua.config.json)

mod approval;
mod audit;
mod config;
mod mcp;
mod ops;
mod vbox;

use config::RuaConfig;
use std::path::Path;

fn main() {
    if let Err(e) = run() {
        eprintln!("rua-nucleo: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cfg_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "rua.config.json".to_string());
    let cfg = RuaConfig::load(Path::new(&cfg_path))?;

    let vbox = vbox::Vbox::new(cfg.vboxmanage_path.clone());
    let audit = audit::AuditLog::new(cfg.audit_log_path.clone());
    let approval = approval::ApprovalVerifier::new(
        cfg.approval_pubkey_hex.as_deref(),
        cfg.consumed_nonces_path.clone(),
    )?;

    let ctx = ops::Ctx {
        cfg,
        vbox,
        audit,
        approval,
    };

    eprintln!(
        "rua-nucleo v0.1.0 — JSON-RPC/MCP por stdio. Nivel de riesgo: {:?}",
        ctx.cfg.risk_level
    );
    mcp::serve(ctx)
}
