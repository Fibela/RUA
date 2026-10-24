//! Orquestador del laboratorio RUA (fase 2).
//!
//! Ejecuta un escenario contra la golden a través del **núcleo** (nunca toca
//! `VBoxManage` directo; habla sus ops cerradas por JSON-RPC/MCP) y evalúa la
//! traza que recibió el `sink` con el **arnés**. Hornea las reglas de secuencia
//! que acordamos con el equipo de Eje-Latam, en el orden en que importan:
//!
//! 1. **Escuchar antes de provocar.** La transición de condición se emite una
//!    sola vez; el `sink` debe estar escuchando antes de arrancar el agente.
//!    El runner lo trata como precondición (recibe la ruta del JSONL del sink,
//!    que ya debe estar corriendo).
//! 2. **Todas las escrituras en el almacén ANTES de arrancar el agente.** Las
//!    revocaciones se cargan dos veces al arrancar; escribir a mitad vuelve el
//!    resultado no determinista. Por construcción: `escrituras_almacen` es una
//!    fase separada y anterior a `arrancar_agente`.
//! 3. **Nunca instalar sin `Verificado`.** Toda entrega al almacén va por
//!    `copy_in_verified`; si el veredicto no es `Verificado`, el escenario se
//!    ABORTA y el agente NO se arranca (PA-12).
//! 4. **Evidencia antes de revertir.** `evidencia.alm → evidencia.anc →
//!    centinela.dat` se recolectan ANTES de cualquier revert; revertir es el
//!    ataque de repetición y destruye justo lo que se mide.

use rua_arnes::{evaluar, veredicto, Clase, Firma, Veredicto};
use serde_json::{json, Value};

/// Transporte hacia el núcleo. Una sola operación: invocar una herramienta MCP
/// `rua.*` con sus argumentos y devolver el objeto de resultado ya parseado
/// (para `copy_in_verified`, el JSON del veredicto; para el resto,
/// `{status, stdout, stderr}`).
pub trait Nucleo {
    fn tool(&mut self, nombre: &str, args: Value) -> Result<Value, String>;
}

/// Una operación del núcleo dentro de un plan de escenario.
#[derive(Debug, Clone)]
pub enum Op {
    /// `rua.snapshot_restore` — solo en escenarios que toleran revert.
    Revert { snapshot: String },
    /// `rua.start` la VM (headless).
    StartVm,
    /// `rua.stop` la VM.
    StopVm { force: bool },
    /// `rua.run` un programa en el guest (para provocar: rm, mv, systemctl…).
    Run { program: String, args: Vec<String> },
    /// `rua.copy_in_verified` — entrega al guest CON verificación de hash.
    /// Si no da `Verificado` (integridad de transporte: origen == destino), el
    /// escenario se aborta.
    ///
    /// `esperado_sha256` añade verificación de **identidad**: el binario en
    /// destino no solo llegó íntegro, es EXACTAMENTE el de procedencia
    /// registrada (p. ej. `eje-agente` del commit de la golden). Si el destino
    /// no coincide con el esperado, no se instala — es el requisito del equipo
    /// ("confirmar dba88063… en destino; si no coincide, no se instala") y es
    /// PA-12 cerrado de punta a punta: transporte Y origen.
    CopyInVerificado {
        host_path: String,
        guest_dir: String,
        esperado_sha256: Option<String>,
    },
    /// Espera pasiva (ms) para dejar emitir al agente.
    Esperar { ms: u64 },
}

/// Plan declarativo de un escenario. Las fases están separadas a propósito para
/// que el orden disciplinado sea estructural, no una convención que se pueda
/// romper.
#[derive(Debug, Clone)]
pub struct Plan {
    pub nombre: String,
    pub vm: String,
    /// `true` para escenarios que no miden tiempo (toleran volver a limpio).
    /// `false` para los que exigen VM persistente (marca de agua, registro).
    pub tolera_revert: bool,
    /// Snapshot limpio al que revertir al empezar (solo si `tolera_revert`).
    pub snapshot_base: Option<String>,
    /// Usuario del guest para las ops de `guestcontrol`.
    pub guest_user: String,
    pub password_file: String,
    /// FASE 1 — todas las escrituras en el almacén (van ANTES de arrancar).
    pub escrituras_almacen: Vec<Op>,
    /// FASE 2 — arrancar el agente (típicamente `Run{systemctl start …}`).
    pub arrancar_agente: Op,
    /// Espera tras arrancar, para que emita.
    pub espera_emisiones_ms: u64,
    /// FASE 3 — evidencia a recolectar ANTES de revertir: (guest_path, host_dir).
    /// El orden de la lista debe ser alm → anc → centinela.
    pub evidencia: Vec<(String, String)>,
    /// Firma y clase para el veredicto del arnés.
    pub firma: Firma,
    pub clase: Clase,
}

/// Resultado de ejecutar un escenario.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultadoEscenario {
    pub nombre: String,
    pub veredicto: Veredicto,
    pub detalle: String,
    /// Si el escenario se abortó antes de medir (p. ej. transporte no
    /// `Verificado`), aquí va el motivo; el veredicto es `NoConcluyente`.
    pub abortado: Option<String>,
}

fn arg_vm(vm: &str) -> Value {
    json!({ "vm": vm })
}

/// ¿Es una semilla de despliegue (`*.sem`)? El nombre base termina en `.sem`.
/// Material que firma inventarios/configuraciones: no se transporta jamás.
pub fn es_semilla(host_path: &str) -> bool {
    host_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(host_path)
        .to_ascii_lowercase()
        .ends_with(".sem")
}

/// Ejecuta un `Op` de guestcontrol que necesita credenciales.
fn run_guest(
    n: &mut dyn Nucleo,
    vm: &str,
    user: &str,
    pwd: &str,
    program: &str,
    args: &[String],
) -> Result<Value, String> {
    n.tool(
        "rua.run",
        json!({ "vm": vm, "program": program, "args": args, "guest_user": user, "password_file": pwd }),
    )
}

/// Ejecuta un escenario completo con la secuencia disciplinada. `cargar_traza`
/// devuelve el JSONL que el sink haya acumulado (el sink debe estar escuchando
/// desde antes). `ahora_ms` es el momento de evaluación para el arnés.
pub fn ejecutar(
    n: &mut dyn Nucleo,
    plan: &Plan,
    cargar_traza: &mut dyn FnMut() -> String,
    ahora_ms: u128,
) -> ResultadoEscenario {
    let abort = |motivo: String| ResultadoEscenario {
        nombre: plan.nombre.clone(),
        veredicto: Veredicto::NoConcluyente,
        detalle: motivo.clone(),
        abortado: Some(motivo),
    };

    // 0. Revert al limpio SOLO si el escenario lo tolera.
    if plan.tolera_revert {
        if let Some(snap) = &plan.snapshot_base {
            if let Err(e) = n.tool(
                "rua.snapshot_restore",
                json!({ "vm": plan.vm, "snapshot": snap }),
            ) {
                return abort(format!("revert inicial falló: {e}"));
            }
        }
    }

    // 1. Arrancar la VM (el servicio del agente debe estar detenido en el
    //    estado de partida de la golden).
    if let Err(e) = n.tool("rua.start", arg_vm(&plan.vm)) {
        return abort(format!("start de la VM falló: {e}"));
    }

    // 2. FASE escrituras: todas antes de arrancar el agente.
    for op in &plan.escrituras_almacen {
        match op {
            Op::CopyInVerificado {
                host_path,
                guest_dir,
                esperado_sha256,
            } => {
                // La semilla (`*.sem`) NUNCA entra a una golden: puede firmar
                // inventarios y configuraciones que un sensor creerá. Vive solo
                // donde corre la ceremonia; RUA se niega a transportarla.
                if es_semilla(host_path) {
                    return abort(format!(
                        "RUA se niega a copiar una semilla a un guest: '{host_path}'. La semilla solo vive donde corre la ceremonia, nunca viaja por copy_in."
                    ));
                }
                let v = match n.tool(
                    "rua.copy_in_verified",
                    json!({ "vm": plan.vm, "host_path": host_path, "guest_dir": guest_dir,
                            "guest_user": plan.guest_user, "password_file": plan.password_file }),
                ) {
                    Ok(v) => v,
                    Err(e) => return abort(format!("copy_in_verified falló: {e}")),
                };
                let ver = v.get("veredicto").and_then(|x| x.as_str()).unwrap_or("");
                if ver != "Verificado" {
                    // Nunca instalar sin Verificado: abortar SIN arrancar.
                    return abort(format!(
                        "entrega no verificada ({ver}): se aborta sin arrancar el agente (PA-12, transporte)"
                    ));
                }
                // Verificación de IDENTIDAD: el binario en destino debe ser
                // EXACTAMENTE el de procedencia registrada. `guest_sha256` ya
                // es == host (Verificado), así que basta compararlo con el
                // esperado. Si no coincide, no se instala.
                if let Some(esperado) = esperado_sha256 {
                    let destino = v.get("guest_sha256").and_then(|x| x.as_str()).unwrap_or("");
                    if !destino.eq_ignore_ascii_case(esperado) {
                        return abort(format!(
                            "procedencia: el binario en destino ({destino}) no es el registrado ({esperado}): NO se instala (PA-12, origen)"
                        ));
                    }
                }
            }
            Op::Run { program, args } => {
                if let Err(e) = run_guest(
                    n,
                    &plan.vm,
                    &plan.guest_user,
                    &plan.password_file,
                    program,
                    args,
                ) {
                    return abort(format!("escritura en almacén ('{program}') falló: {e}"));
                }
            }
            Op::Esperar { ms } => {
                std::thread::sleep(std::time::Duration::from_millis(*ms));
            }
            otro => return abort(format!("op no válida en fase de escrituras: {otro:?}")),
        }
    }

    // 3. FASE arrancar el agente.
    match &plan.arrancar_agente {
        Op::Run { program, args } => {
            if let Err(e) = run_guest(
                n,
                &plan.vm,
                &plan.guest_user,
                &plan.password_file,
                program,
                args,
            ) {
                return abort(format!("arrancar el agente falló: {e}"));
            }
        }
        otro => return abort(format!("arrancar_agente debe ser Run, es {otro:?}")),
    }

    // 4. Dejar emitir.
    if plan.espera_emisiones_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(
            plan.espera_emisiones_ms.min(5000),
        ));
    }

    // 5. FASE evidencia — ANTES de cualquier revert.
    for (guest_path, host_dir) in &plan.evidencia {
        if let Err(e) = n.tool(
            "rua.collect",
            json!({ "vm": plan.vm, "guest_path": guest_path, "host_dir": host_dir,
                    "guest_user": plan.guest_user, "password_file": plan.password_file }),
        ) {
            return abort(format!(
                "collect de evidencia '{guest_path}' falló (antes de revertir): {e}"
            ));
        }
    }

    // 6. Apagar.
    let _ = n.tool("rua.stop", json!({ "vm": plan.vm, "force": false }));

    // 7. Evaluar la traza del sink con el arnés.
    let texto = cargar_traza();
    let tr = match rua_arnes::parse_jsonl(&texto) {
        Ok(t) => t,
        Err(e) => return abort(format!("traza del sink inválida: {e}")),
    };
    let r = evaluar(&plan.firma, &tr, ahora_ms);
    let v = veredicto(plan.clase, &r);

    // 8. Limpieza: revertir SOLO si tolera (y ya se recolectó la evidencia).
    if plan.tolera_revert {
        if let Some(snap) = &plan.snapshot_base {
            let _ = n.tool(
                "rua.snapshot_restore",
                json!({ "vm": plan.vm, "snapshot": snap }),
            );
        }
    }

    let detalle = match &r {
        rua_arnes::Resultado::Cumplida => "firma cumplida".to_string(),
        rua_arnes::Resultado::NoCumplida(m) | rua_arnes::Resultado::NoMedible(m) => m.clone(),
    };
    ResultadoEscenario {
        nombre: plan.nombre.clone(),
        veredicto: v,
        detalle,
        abortado: None,
    }
}

// ---------------------------------------------------------------------------
// Cliente real: habla JSON-RPC/MCP por stdio con el binario `rua-nucleo`.
// ---------------------------------------------------------------------------

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub struct NucleoStdio {
    hijo: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    id: i64,
}

impl NucleoStdio {
    /// Lanza el núcleo con `RUA_CONFIG=config` y hace el handshake MCP.
    pub fn lanzar(exe: &str, config: &str) -> Result<Self, String> {
        let mut hijo = Command::new(exe)
            .env("RUA_CONFIG", config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("no se pudo lanzar el núcleo: {e}"))?;
        let stdin = hijo.stdin.take().ok_or("sin stdin")?;
        let stdout = BufReader::new(hijo.stdout.take().ok_or("sin stdout")?);
        let mut s = Self {
            hijo,
            stdin,
            stdout,
            id: 0,
        };
        s.rpc("initialize", json!({}))?;
        Ok(s)
    }

    fn rpc(&mut self, metodo: &str, params: Value) -> Result<Value, String> {
        self.id += 1;
        let req = json!({ "jsonrpc": "2.0", "id": self.id, "method": metodo, "params": params });
        writeln!(self.stdin, "{req}").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        loop {
            let mut linea = String::new();
            let n = self
                .stdout
                .read_line(&mut linea)
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("el núcleo cerró stdout".into());
            }
            if linea.trim().is_empty() {
                continue;
            }
            let v: Value = match serde_json::from_str(&linea) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("id").and_then(|x| x.as_i64()) == Some(self.id) {
                if let Some(err) = v.get("error") {
                    return Err(err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("error")
                        .to_string());
                }
                return Ok(v.get("result").cloned().unwrap_or(json!({})));
            }
        }
    }
}

impl Nucleo for NucleoStdio {
    fn tool(&mut self, nombre: &str, args: Value) -> Result<Value, String> {
        let result = self.rpc("tools/call", json!({ "name": nombre, "arguments": args }))?;
        if result
            .get("isError")
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
        {
            let t = result
                .pointer("/content/0/text")
                .and_then(|x| x.as_str())
                .unwrap_or("error");
            return Err(t.to_string());
        }
        let text = result
            .pointer("/content/0/text")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        // copy_in_verified devuelve JSON (con "veredicto"); el resto, texto.
        match serde_json::from_str::<Value>(text) {
            Ok(v) if v.is_object() => Ok(v),
            _ => Ok(json!({ "stdout": text })),
        }
    }
}

impl Drop for NucleoStdio {
    fn drop(&mut self) {
        let _ = self.hijo.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Mock que registra el orden de las llamadas y devuelve respuestas
    /// programables, para verificar la SECUENCIA (la propiedad que importa).
    struct NucleoMock {
        log: Rc<RefCell<Vec<String>>>,
        copy_verificado: bool,
    }
    impl Nucleo for NucleoMock {
        fn tool(&mut self, nombre: &str, args: Value) -> Result<Value, String> {
            // Etiqueta legible de la llamada, para el log de orden.
            let etiqueta = match nombre {
                "rua.run" => format!(
                    "run:{}",
                    args.get("program").and_then(|x| x.as_str()).unwrap_or("")
                ),
                "rua.collect" => format!(
                    "collect:{}",
                    args.get("guest_path")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                ),
                "rua.copy_in_verified" => "copy_in_verified".to_string(),
                otro => otro.to_string(),
            };
            self.log.borrow_mut().push(etiqueta);
            if nombre == "rua.copy_in_verified" {
                let ver = if self.copy_verificado {
                    "Verificado"
                } else {
                    "NoCoincide"
                };
                // guest_sha256 fijo para probar el gate de identidad.
                return Ok(
                    json!({ "veredicto": ver, "guest_sha256": "deadbeef", "host_sha256": "deadbeef" }),
                );
            }
            Ok(json!({ "status": 0, "stdout": "", "stderr": "" }))
        }
    }

    fn plan_base() -> Plan {
        Plan {
            nombre: "rojo-clave-borrada".into(),
            vm: "golden-eje-linux".into(),
            tolera_revert: true,
            snapshot_base: Some("base-identidad-anclada".into()),
            guest_user: "vboxuser".into(),
            password_file: "/x".into(),
            escrituras_almacen: vec![Op::Run {
                program: "/bin/rm".into(),
                args: vec!["/var/lib/eje/clave".into()],
            }],
            arrancar_agente: Op::Run {
                program: "/usr/bin/systemctl".into(),
                args: vec!["start".into(), "eje-agente".into()],
            },
            espera_emisiones_ms: 0,
            evidencia: vec![
                ("/var/lib/eje/evidencia.alm".into(), "/host/ev".into()),
                ("/var/lib/eje/evidencia.anc".into(), "/host/ev".into()),
                ("/var/lib/eje/centinela.dat".into(), "/host/ev".into()),
            ],
            firma: Firma::Perturbacion {
                condicion: "operativaSuprimida",
                gravedad: 3,
            },
            clase: Clase::Rojo,
        }
    }

    fn traza_queja() -> String {
        r#"{"recibido_ms":1000,"origen":"x","evento":{"gravedad":3,"msgid":"condicion","clase":"condicion","campos":{"condicion":"operativaSuprimida","estado":"activa"},"condiciones":null}}"#.to_string()
    }

    #[test]
    fn secuencia_escrituras_antes_de_arrancar_agente() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log: log.clone(),
            copy_verificado: true,
        };
        let plan = plan_base();
        let mut cargar = || traza_queja();
        let _ = ejecutar(&mut n, &plan, &mut cargar, 2000);
        let l = log.borrow();
        let i_escritura = l.iter().position(|x| x == "run:/bin/rm").unwrap();
        let i_arranque = l
            .iter()
            .position(|x| x == "run:/usr/bin/systemctl")
            .unwrap();
        assert!(
            i_escritura < i_arranque,
            "la escritura al almacén debe ir antes de arrancar el agente: {l:?}"
        );
    }

    #[test]
    fn evidencia_antes_de_revertir_de_limpieza() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log: log.clone(),
            copy_verificado: true,
        };
        let plan = plan_base();
        let mut cargar = || traza_queja();
        let _ = ejecutar(&mut n, &plan, &mut cargar, 2000);
        let l = log.borrow();
        let i_evidencia = l.iter().rposition(|x| x.starts_with("collect:")).unwrap();
        // El revert de limpieza es el ÚLTIMO snapshot_restore.
        let i_revert_limpieza = l.iter().rposition(|x| x == "rua.snapshot_restore").unwrap();
        assert!(
            i_evidencia < i_revert_limpieza,
            "la evidencia debe recolectarse antes del revert de limpieza: {l:?}"
        );
    }

    #[test]
    fn copy_no_verificado_aborta_sin_arrancar() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log: log.clone(),
            copy_verificado: false,
        };
        let mut plan = plan_base();
        plan.escrituras_almacen = vec![Op::CopyInVerificado {
            host_path: "/host/muestra".into(),
            guest_dir: "/var/lib/eje".into(),
            esperado_sha256: None,
        }];
        let mut cargar = || traza_queja();
        let r = ejecutar(&mut n, &plan, &mut cargar, 2000);
        assert!(r.abortado.is_some());
        assert_eq!(r.veredicto, Veredicto::NoConcluyente);
        let l = log.borrow();
        assert!(
            !l.iter().any(|x| x == "run:/usr/bin/systemctl"),
            "no debe arrancar el agente si la entrega no se verificó: {l:?}"
        );
    }

    #[test]
    fn identidad_binario_equivocado_aborta_sin_arrancar() {
        // El transporte da Verificado (origen==destino), pero el binario NO es
        // el de procedencia registrada → no se instala, no se arranca.
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log: log.clone(),
            copy_verificado: true,
        };
        let mut plan = plan_base();
        plan.escrituras_almacen = vec![Op::CopyInVerificado {
            host_path: "/host/eje-agente".into(),
            guest_dir: "/usr/local/bin".into(),
            esperado_sha256: Some("dba88063".into()), // el mock entrega "deadbeef"
        }];
        let mut cargar = || traza_queja();
        let r = ejecutar(&mut n, &plan, &mut cargar, 2000);
        assert!(r.abortado.as_deref().unwrap_or("").contains("procedencia"));
        let l = log.borrow();
        assert!(
            !l.iter().any(|x| x == "run:/usr/bin/systemctl"),
            "binario de procedencia equivocada no debe arrancar: {l:?}"
        );
    }

    #[test]
    fn identidad_binario_correcto_pasa() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log,
            copy_verificado: true,
        };
        let mut plan = plan_base();
        plan.escrituras_almacen = vec![Op::CopyInVerificado {
            host_path: "/host/eje-agente".into(),
            guest_dir: "/usr/local/bin".into(),
            esperado_sha256: Some("deadbeef".into()), // coincide con el mock
        }];
        let mut cargar = || traza_queja();
        let r = ejecutar(&mut n, &plan, &mut cargar, 2000);
        assert!(
            r.abortado.is_none(),
            "el binario correcto no debe abortar: {:?}",
            r.abortado
        );
    }

    #[test]
    fn semilla_no_viaja_a_la_golden() {
        assert!(es_semilla("/home/u/cliente.sem"));
        assert!(es_semilla("C:\\x\\clave.SEM"));
        assert!(!es_semilla("/host/inventario.inv"));
        // Un plan que intente copiar una semilla se aborta SIN arrancar.
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log: log.clone(),
            copy_verificado: true,
        };
        let mut plan = plan_base();
        plan.escrituras_almacen = vec![Op::CopyInVerificado {
            host_path: "/home/u/cliente.sem".into(),
            guest_dir: "/var/lib/eje".into(),
            esperado_sha256: None,
        }];
        let mut cargar = || traza_queja();
        let r = ejecutar(&mut n, &plan, &mut cargar, 2000);
        assert!(r.abortado.as_deref().unwrap_or("").contains("semilla"));
        let l = log.borrow();
        assert!(
            !l.iter().any(|x| x == "copy_in_verified"),
            "no debe ni intentar copiar la semilla: {l:?}"
        );
        assert!(
            !l.iter().any(|x| x == "run:/usr/bin/systemctl"),
            "no debe arrancar el agente: {l:?}"
        );
    }

    #[test]
    fn persistente_no_revierte() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log: log.clone(),
            copy_verificado: true,
        };
        let mut plan = plan_base();
        plan.tolera_revert = false; // escenario de marca de agua / registro
        plan.nombre = "rojo-marca-de-agua".into();
        let mut cargar = || traza_queja();
        let _ = ejecutar(&mut n, &plan, &mut cargar, 2000);
        let l = log.borrow();
        assert!(
            !l.iter().any(|x| x == "rua.snapshot_restore"),
            "una VM persistente no debe revertirse nunca: {l:?}"
        );
    }

    #[test]
    fn rojo_con_queja_da_correcto() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut n = NucleoMock {
            log,
            copy_verificado: true,
        };
        let plan = plan_base();
        let mut cargar = || traza_queja();
        let r = ejecutar(&mut n, &plan, &mut cargar, 2000);
        assert_eq!(r.veredicto, Veredicto::Correcto);
    }
}
