//! Transporte JSON-RPC 2.0 por stdio (newline-delimited), que es la base del
//! transporte stdio de MCP. El handshake formal de MCP (initialize / tools/list
//! / tools/call) es una capa delgada que se añade encima cuando conectemos el
//! cliente IA; por ahora exponemos los métodos `rua.*` directamente para poder
//! probar el núcleo end-to-end sin esa ceremonia.

use crate::ops::{self, Ctx};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn error_response(id: Value, code: i64, message: &str) -> String {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}}).to_string()
}

fn handle_line(ctx: &Ctx, line: &str) -> String {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return error_response(Value::Null, -32700, &format!("parse error: {e}")),
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
    match ops::dispatch(ctx, method, &params) {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}).to_string(),
        Err(e) => error_response(id, -32000, &e),
    }
}

pub fn serve(ctx: Ctx) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let resp = handle_line(&ctx, &line);
        writeln!(out, "{resp}").map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}
