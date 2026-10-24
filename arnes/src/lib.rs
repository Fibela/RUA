//! Arnés de validación del laboratorio RUA — motor de aserción (fase 2).
//!
//! Lee las trazas que el `sink` almacenó (JSONL, un registro por línea con la
//! forma `{recibido_ms, origen, evento}`) y las contrasta con la **firma
//! esperada** de un escenario. No ejecuta el escenario en la VM: eso lo hará el
//! orquestador cuando exista la golden; aquí vive la lógica que decide si lo
//! recibido coincide con lo esperado.
//!
//! Disciplina del proyecto (una sola regla): **nunca informar de un éxito que
//! no se ha medido.** Por eso toda evaluación es tri-estado —
//! `Cumplida` / `NoCumplida` / `NoMedible` — y "no hay con qué medir" (no ha
//! llegado ni un latido, falta `intervaloMs`, traza vacía) es un resultado
//! explícito, no un verde por defecto.
//!
//! Correcciones del equipo Eje-Latam ya incorporadas:
//! - "colector caído" NO produce `condiciones=sinColector` en el cable: esa
//!   condición nunca viaja por syslog (sólo por el socket local). El escenario
//!   de colector caído se valida por **AUSENCIA** de latidos (PA-105).
//! - El `intervaloMs` NO se fija como constante: se lee del último latido
//!   recibido (PA-147: plantilla 30000 vs código 60000).
//! - "todavía no ha llegado el primero" (NoMedible) ≠ "dejaron de llegar"
//!   (silencio).

use serde::Deserialize;

/// Un evento tal como lo serializó el sink (sólo los campos que el arnés mira).
#[derive(Debug, Clone, Deserialize)]
pub struct Evento {
    #[serde(default)]
    pub gravedad: u8,
    #[serde(default)]
    pub msgid: String,
    #[serde(default)]
    pub clase: String,
    #[serde(default)]
    pub campos: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub condiciones: Option<Vec<String>>,
}

/// Un registro del JSONL del sink: evento + metadatos de recepción del host.
#[derive(Debug, Clone, Deserialize)]
pub struct Registro {
    pub recibido_ms: u128,
    #[serde(default)]
    pub origen: String,
    pub evento: Evento,
}

/// Resultado tri-estado de una aserción. `NoMedible` lleva el motivo, porque
/// "no se pudo medir" es información, no ruido.
#[derive(Debug, Clone, PartialEq)]
pub enum Resultado {
    Cumplida,
    NoCumplida(String),
    NoMedible(String),
}

/// Clase del escenario: un verde debe cumplir su firma; un rojo
/// (perturbación) debe provocar que el agente se queje — su firma ES la queja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clase {
    Verde,
    Rojo,
}

/// Firma esperada de un escenario. Cada variante es una aserción concreta sobre
/// la traza.
#[derive(Debug, Clone)]
pub enum Firma {
    /// Calma: al menos un `latido-de-sensor` con `condiciones=ninguna`
    /// (afirmación de que no hay condiciones, no ausencia del campo).
    LatidoEnCalma,
    /// Al menos un `sello-de-evidencia`.
    SelloDeEvidencia,
    /// Amenaza: al menos un `amenaza-incontenible` con `asiento` presente
    /// (correlacionable al registro de evidencia local).
    AmenazaConAsiento,
    /// Ausencia de latidos: el colector dejó de recibir. Se mide el silencio
    /// contra `factor × intervaloMs` del ÚLTIMO latido recibido. NoMedible si
    /// no ha llegado ningún latido. `factor >= 1.5` (el equipo: por debajo
    /// aparecen silencios falsos del reloj, no del agente; 2.0 da margen).
    Silencio { factor: f64 },
    /// Auto-chequeo del laboratorio (PA-147): en calma, el colector ve
    /// actividad y NO debe declarar un silencio falso. Es el inverso de
    /// `Silencio`: Cumplida si hay un latido reciente dentro de `factor ×
    /// intervaloMs` **del cable**. Mide al LABORATORIO, no al agente: si el
    /// arnés fijara el intervalo a mano (el TOML dice 30000, el código vale
    /// 60000), con latidos cada 60000 declararía silencio contra 2×30000 y
    /// acusaría a un agente sano. Leer el intervalo del último latido lo evita.
    SinSilencioFalso { factor: f64 },
    /// Caso en rojo: la queja del agente es una **transición de condición**
    /// (MSGID `condicion`, `estado=activa`, `condicion=<nombre>`), cuya
    /// **gravedad** codifica la URGENCIA, NO un juicio de manipulación (§10):
    ///   - 3 (Error, PRI 107): urgente, "no espera".
    ///   - 4 (Aviso, PRI 108): aviso, menos urgente.
    ///
    /// Si hubo manipulación es una columna APARTE del contrato
    /// (`manipulacion: bool` por condición), NO derivable de la gravedad: hay
    /// condiciones 107 sin manipulación (p. ej. `registroSaturado`). Que en los
    /// escenarios de abajo las 107 coincidan con manipulación es del conjunto
    /// elegido, no una regla. El arnés exige la gravedad porque es la urgencia
    /// que lleva la transición; no mide manipulación.
    ///
    /// Se exigen las DOS señales (el equipo): la **transición** da el instante
    /// y la gravedad; el **latido** confirma que la condición sigue siendo
    /// verdad (persiste en `condiciones=`). Reglas tri-estado:
    ///   - transición con gravedad esperada → Cumplida.
    ///   - transición presente con OTRA gravedad → NoCumplida (urgencia mal codificada).
    ///   - sin transición pero la condición persiste en un latido → NoMedible
    ///     (llegamos tarde: no se puede juzgar la gravedad).
    ///   - ni transición ni latido → NoCumplida (el agente no la cazó).
    ///
    /// OJO: `amenaza-incontenible` (gravedad 1) es OTRO suceso (dispositivo
    /// incontenible del segmento), NUNCA es una condición del almacén;
    /// usarla aquí haría que los rojos pasaran como verdes.
    Perturbacion {
        condicion: &'static str,
        gravedad: u8,
    },
    /// Discriminación actualización ≠ ataque. Ante un evento legítimo (p. ej.
    /// reponer un inventario de **formato anterior** = `FormatoObsoleto`), el
    /// agente DEBE emitir la condición **benigna** (`accionAdministrativa`,
    /// gravedad 4 / PRI 108) y **NO** debe encender ninguna alarma de ataque
    /// (`no_alarmas`, p. ej. `inventarioNoVerifica`/`inventarioSuprimido`, 107).
    ///
    /// Protege el rojo de marca de agua: si el orquestador repone un inventario
    /// de otra versión del emisor, el agente dice "formato anterior, reemita"
    /// (108 correcto) y un rojo que espere 107 pasaría como verde. Este
    /// escenario prueba, en positivo, que el agente distingue las dos cosas.
    ///
    /// Reglas:
    ///   - benigna (gravedad correcta) y ninguna alarma → Cumplida.
    ///   - alguna alarma de ataque presente → NoCumplida (acusó una
    ///     actualización legítima).
    ///   - benigna con otra gravedad → NoCumplida.
    ///   - sin benigna pero persiste en latido → NoMedible (llegó tarde).
    ///   - nada → NoCumplida (no reconoció la actualización).
    Discriminacion {
        benigna: &'static str,
        gravedad_benigna: u8,
        no_alarmas: &'static [&'static str],
    },
}

/// Un escenario del catálogo: nombre, clase (verde/rojo) y su firma.
#[derive(Debug, Clone)]
pub struct Escenario {
    pub nombre: &'static str,
    pub clase: Clase,
    pub firma: Firma,
    /// Qué provoca el escenario en la VM (documentación para el orquestador;
    /// el motor de aserción no lo usa).
    pub provoca: &'static str,
}

fn latidos(tr: &[Registro]) -> Vec<&Registro> {
    let mut v: Vec<&Registro> = tr.iter().filter(|r| r.evento.clase == "latido").collect();
    v.sort_by_key(|r| r.recibido_ms);
    v
}

/// Transiciones `condicion=<nombre> estado=activa` de un nombre dado.
fn transiciones_activas<'a>(tr: &'a [Registro], nombre: &str) -> Vec<&'a Registro> {
    tr.iter()
        .filter(|r| {
            r.evento.msgid == "condicion"
                && r.evento.campos.get("condicion").map(|s| s.as_str()) == Some(nombre)
                && r.evento.campos.get("estado").map(|s| s.as_str()) == Some("activa")
        })
        .collect()
}

/// ¿La condición persiste como verdad en algún latido (`condiciones=`)?
fn persiste_en_latido(tr: &[Registro], nombre: &str) -> bool {
    tr.iter().any(|r| {
        r.evento.clase == "latido"
            && r.evento
                .condiciones
                .as_ref()
                .map(|c| c.iter().any(|x| x == nombre))
                .unwrap_or(false)
    })
}

/// Evalúa una firma contra la traza. `ahora_ms` es el momento de evaluación
/// (reloj de pared del host), necesario para medir silencio; en pruebas se
/// pasa explícito.
pub fn evaluar(firma: &Firma, tr: &[Registro], ahora_ms: u128) -> Resultado {
    match firma {
        Firma::LatidoEnCalma => {
            let hay = tr.iter().any(|r| {
                r.evento.clase == "latido"
                    && r.evento
                        .condiciones
                        .as_ref()
                        .map(|c| c.iter().any(|x| x == "ninguna"))
                        .unwrap_or(false)
            });
            if hay {
                Resultado::Cumplida
            } else if tr.iter().any(|r| r.evento.clase == "latido") {
                Resultado::NoCumplida("hay latidos pero ninguno con condiciones=ninguna".into())
            } else {
                Resultado::NoMedible("no llegó ningún latido".into())
            }
        }

        Firma::SelloDeEvidencia => {
            if tr.iter().any(|r| r.evento.clase == "sello_evidencia") {
                Resultado::Cumplida
            } else {
                Resultado::NoCumplida("no llegó ningún sello-de-evidencia".into())
            }
        }

        Firma::AmenazaConAsiento => {
            let hay = tr
                .iter()
                .any(|r| r.evento.clase == "amenaza" && r.evento.campos.contains_key("asiento"));
            if hay {
                Resultado::Cumplida
            } else if tr.iter().any(|r| r.evento.clase == "amenaza") {
                Resultado::NoCumplida("hay amenaza pero sin campo asiento".into())
            } else {
                Resultado::NoCumplida("no llegó ninguna amenaza-incontenible".into())
            }
        }

        Firma::Silencio { factor } => {
            let lat = latidos(tr);
            let ultimo = match lat.last() {
                None => {
                    // "todavía no ha llegado el primero" NO es silencio.
                    return Resultado::NoMedible(
                        "no ha llegado ningún latido: no se puede distinguir silencio de arranque"
                            .into(),
                    );
                }
                Some(u) => *u,
            };
            let intervalo = match ultimo
                .evento
                .campos
                .get("intervaloMs")
                .and_then(|v| v.parse::<u128>().ok())
            {
                Some(i) if i > 0 => i,
                _ => {
                    return Resultado::NoMedible(
                        "el último latido no trae un intervaloMs usable".into(),
                    )
                }
            };
            let umbral = (intervalo as f64 * factor) as u128;
            let transcurrido = ahora_ms.saturating_sub(ultimo.recibido_ms);
            if transcurrido > umbral {
                Resultado::Cumplida // silencio confirmado
            } else {
                Resultado::NoCumplida(format!(
                    "siguen llegando latidos: {transcurrido}ms desde el último ≤ umbral {umbral}ms ({factor}×{intervalo})"
                ))
            }
        }

        Firma::SinSilencioFalso { factor } => {
            // Inverso de Silencio, leyendo el intervalo del cable. Cumplida si
            // NO hay silencio (el colector ve actividad dentro de la ventana).
            match evaluar(&Firma::Silencio { factor: *factor }, tr, ahora_ms) {
                Resultado::Cumplida => Resultado::NoCumplida(
                    "se declaró silencio cuando el agente seguía dentro de ventana: el laboratorio mediría contra un intervalo equivocado (PA-147)".into(),
                ),
                Resultado::NoCumplida(_) => Resultado::Cumplida,
                Resultado::NoMedible(m) => Resultado::NoMedible(m),
            }
        }

        Firma::Perturbacion {
            condicion,
            gravedad,
        } => {
            // Transiciones de la condición esperada que se activan.
            let transiciones = transiciones_activas(tr, condicion);
            let persiste = persiste_en_latido(tr, condicion);

            if transiciones.is_empty() {
                if persiste {
                    return Resultado::NoMedible(format!(
                        "la condición '{condicion}' persiste en el latido pero no se capturó su transición: el arnés llegó tarde y no puede juzgar la gravedad (debe escuchar ANTES de provocar el estado)"
                    ));
                }
                return Resultado::NoCumplida(format!(
                    "el agente no emitió la condición '{condicion}' ni la refleja en ningún latido"
                ));
            }

            // Hay transición: ¿con la gravedad esperada (el juicio correcto)?
            if transiciones.iter().any(|r| r.evento.gravedad == *gravedad) {
                let nota = if persiste {
                    ""
                } else {
                    " (sin confirmación aún en latido)"
                };
                let _ = nota;
                Resultado::Cumplida
            } else {
                let vista = transiciones[0].evento.gravedad;
                Resultado::NoCumplida(format!(
                    "transición de '{condicion}' con gravedad {vista}, esperada {gravedad}: el agente codificó mal la urgencia (gravedad) de la condición"
                ))
            }
        }

        Firma::Discriminacion {
            benigna,
            gravedad_benigna,
            no_alarmas,
        } => {
            // ¿Acusó el agente una actualización legítima como si fuera ataque?
            let alarmas: Vec<&str> = no_alarmas
                .iter()
                .copied()
                .filter(|a| !transiciones_activas(tr, a).is_empty())
                .collect();
            if !alarmas.is_empty() {
                return Resultado::NoCumplida(format!(
                    "el agente marcó {alarmas:?} (condiciones de manipulación) ante una actualización legítima: debía emitir '{benigna}' (acción administrativa, no manipulación)"
                ));
            }
            let benignas = transiciones_activas(tr, benigna);
            if benignas.is_empty() {
                if persiste_en_latido(tr, benigna) {
                    return Resultado::NoMedible(format!(
                        "'{benigna}' persiste en el latido pero no se capturó su transición: el arnés llegó tarde (debe escuchar ANTES de provocar)"
                    ));
                }
                return Resultado::NoCumplida(format!(
                    "el agente no emitió '{benigna}' (108): no reconoció la actualización como acción administrativa"
                ));
            }
            if benignas
                .iter()
                .any(|r| r.evento.gravedad == *gravedad_benigna)
            {
                Resultado::Cumplida
            } else {
                let vista = benignas[0].evento.gravedad;
                Resultado::NoCumplida(format!(
                    "'{benigna}' con gravedad {vista}, esperada {gravedad_benigna}"
                ))
            }
        }
    }
}

/// Veredicto de un escenario, ya interpretado según su clase.
#[derive(Debug, Clone, PartialEq)]
pub enum Veredicto {
    /// Verde que cumplió, o rojo que provocó la queja: el lab midió bien.
    Correcto,
    /// Un rojo que pasó como verde: el agente NO se quejó cuando debía. Es un
    /// falso OK del propio laboratorio y hace fallar la corrida entera.
    FalsoOkDelLab,
    /// Un verde que no cumplió su firma: posible fallo del agente.
    FalloDelAgente,
    /// No se pudo medir. No se puede afirmar éxito ("no sé").
    NoConcluyente,
}

/// Interpreta el resultado de una aserción según la clase del escenario.
pub fn veredicto(clase: Clase, r: &Resultado) -> Veredicto {
    match (clase, r) {
        (_, Resultado::NoMedible(_)) => Veredicto::NoConcluyente,
        (Clase::Verde, Resultado::Cumplida) => Veredicto::Correcto,
        (Clase::Verde, Resultado::NoCumplida(_)) => Veredicto::FalloDelAgente,
        // Rojo: la firma ES la queja esperada. Que se cumpla = el agente cazó la
        // manipulación = el lab midió bien. Que NO se cumpla = falso OK del lab.
        (Clase::Rojo, Resultado::Cumplida) => Veredicto::Correcto,
        (Clase::Rojo, Resultado::NoCumplida(_)) => Veredicto::FalsoOkDelLab,
    }
}

/// Resultado de una corrida completa sobre el catálogo.
#[derive(Debug, Clone, PartialEq)]
pub enum Corrida {
    /// Todos los escenarios concluyentes y correctos.
    Aprobada,
    /// Al menos un falso OK del lab: la corrida entera falla (la regla del §4).
    FalsoOkDelLab,
    /// Hubo fallos del agente pero ningún falso OK del lab.
    FalloDelAgente,
    /// Todo lo demás concluyente quedó bien, pero algo no se pudo medir.
    NoConcluyente,
}

/// Agrega veredictos de una corrida con la precedencia del §4: un falso OK del
/// lab manda sobre todo; luego el fallo del agente; luego lo no medible.
pub fn agregar(veredictos: &[Veredicto]) -> Corrida {
    if veredictos.contains(&Veredicto::FalsoOkDelLab) {
        return Corrida::FalsoOkDelLab;
    }
    if veredictos.contains(&Veredicto::FalloDelAgente) {
        return Corrida::FalloDelAgente;
    }
    if veredictos.contains(&Veredicto::NoConcluyente) {
        return Corrida::NoConcluyente;
    }
    Corrida::Aprobada
}

/// Catálogo de fase 2 (alcance acordado con el equipo): el camino al colector
/// completo + las tres condiciones de identidad alcanzables hoy. El orquestador
/// provoca cada `provoca` en la golden y recolecta la traza del sink.
///
/// NOTA: las firmas de los rojos de identidad y de marca de agua usan
/// `AlertaDeQueja` como mejor señal conocida; la firma exacta de cada queja
/// está pendiente de confirmación del equipo y se afinará sin tocar el motor.
pub fn catalogo_fase2() -> Vec<Escenario> {
    vec![
        Escenario {
            nombre: "calma",
            clase: Clase::Verde,
            firma: Firma::LatidoEnCalma,
            provoca: "golden en reposo con colector configurado; esperar >1 intervalo",
        },
        Escenario {
            nombre: "sello-periodico",
            clase: Clase::Verde,
            firma: Firma::SelloDeEvidencia,
            provoca: "golden en reposo; esperar a que selle el registro",
        },
        Escenario {
            nombre: "amenaza-inyectada",
            clase: Clase::Verde,
            firma: Firma::AmenazaConAsiento,
            provoca: "inyectar dispositivo/condición que dispare amenaza-incontenible",
        },
        Escenario {
            nombre: "colector-caido",
            clase: Clase::Verde,
            firma: Firma::Silencio { factor: 2.0 },
            provoca: "detener el sink tras recibir latidos; medir ausencia (PA-105)",
        },
        Escenario {
            // Auto-chequeo del laboratorio (PA-147): mide al arnés, no al
            // agente. Con sensor.toml declarando 30000 pero el agente latiendo a
            // 60000, exige que NO se declare silencio falso dentro de 2× del
            // intervalo DEL CABLE. Si el arnés fijara 30000 a mano, este caso
            // fallaría — que es justo lo que debe cazar.
            nombre: "auto-chequeo-intervalo-pa147",
            clase: Clase::Verde,
            firma: Firma::SinSilencioFalso { factor: 2.0 },
            provoca: "cargar sensor.toml (intervalo_latido_ms=30000); dejar latir al agente (60000) dentro de ventana; exigir que el arnés use el intervalo del cable",
        },
        // Rojos de identidad. La gravedad codifica la URGENCIA (§10): 3
        // (Error/107) urgente, 4 (Aviso/108) aviso. Si hubo manipulación es
        // columna aparte del contrato, NO la gravedad; aquí la de recuperación
        // (108) resulta no-manipulación y las otras (107) manipulación, pero
        // eso es del conjunto elegido, no una regla.
        // El arnés debe escuchar ANTES de provocar (la transición se emite una
        // sola vez, al cambiar — `transiciones()` del agente devuelve vacío si
        // no cambió nada).
        Escenario {
            nombre: "rojo-clave-borrada",
            clase: Clase::Rojo,
            firma: Firma::Perturbacion { condicion: "operativaSuprimida", gravedad: 3 },
            provoca: "escuchar → borrar el fichero de clave operativa del almacén → arrancar el agente",
        },
        Escenario {
            nombre: "rojo-clave-sustituida",
            clase: Clase::Rojo,
            firma: Firma::Perturbacion { condicion: "operativaNoVerifica", gravedad: 3 },
            provoca: "escuchar → sustituir la clave operativa por otra → arrancar el agente",
        },
        Escenario {
            // 108, NO 107: falta la ceremonia, no es manipulación. Exigir 107
            // aquí acusaría al agente de un fallo que no comete.
            nombre: "rojo-sin-clave-recuperacion",
            clase: Clase::Rojo,
            firma: Firma::Perturbacion { condicion: "recuperacionNoAprovisionada", gravedad: 4 },
            provoca: "escuchar → arrancar sin clave de recuperación anclada",
        },
        Escenario {
            // Reponer SÓLO el inventario viejo (sec ≤ marca) con copy_in,
            // dejando el centinela en N+1. El fichero está, se lee, la carga lo
            // rechaza contra el centinela → Err(_) => NoVerifica (107). Un
            // revert de VM completa no reproduce el ataque.
            nombre: "rojo-marca-de-agua",
            clase: Clase::Rojo,
            firma: Firma::Perturbacion { condicion: "inventarioNoVerifica", gravedad: 3 },
            provoca: "escuchar → snapshot → emitir inventario sec>N e instalarlo → arrancar (acepta, marca N+1) → apagar, revertir → reponer SÓLO el inventario viejo (copy_in) → arrancar → exigir queja",
        },
        Escenario {
            // Borrar el fichero (no reponer). NoExiste + centinela con secuencia
            // → Supresion (107). Ataque distinto, más fácil de provocar.
            nombre: "rojo-inventario-suprimido",
            clase: Clase::Rojo,
            firma: Firma::Perturbacion { condicion: "inventarioSuprimido", gravedad: 3 },
            provoca: "escuchar → borrar inventario.inv del almacén (centinela queda en N+1) → arrancar → exigir queja",
        },
        Escenario {
            // Discriminación actualización ≠ ataque. Reponer un inventario de
            // FORMATO ANTERIOR → el agente separa FormatoObsoleto ANTES de
            // NoVerifica y emite accionAdministrativa (108), no 107. Verde: el
            // comportamiento correcto es la clasificación benigna; fallar aquí
            // (acusar 107 o no reconocerla) es fallo del agente, no falso OK del
            // lab. Protege el rojo de marca de agua de un mis-provoke.
            nombre: "discrimina-formato-obsoleto",
            clase: Clase::Verde,
            firma: Firma::Discriminacion {
                benigna: "accionAdministrativa",
                gravedad_benigna: 4,
                no_alarmas: &["inventarioNoVerifica", "inventarioSuprimido"],
            },
            provoca: "escuchar → reponer un inventario emitido por OTRA versión del emisor (formato anterior) → arrancar → exigir accionAdministrativa (108), NO un 107",
        },
    ]
}

/// Parsea un JSONL del sink (una línea por registro, ignora líneas vacías).
pub fn parse_jsonl(texto: &str) -> Result<Vec<Registro>, String> {
    let mut out = Vec::new();
    for (i, linea) in texto.lines().enumerate() {
        if linea.trim().is_empty() {
            continue;
        }
        let r: Registro =
            serde_json::from_str(linea).map_err(|e| format!("línea {}: {e}", i + 1))?;
        out.push(r);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(
        ms: u128,
        clase: &str,
        msgid: &str,
        gravedad: u8,
        campos: &[(&str, &str)],
        cond: Option<Vec<&str>>,
    ) -> Registro {
        let mut m = std::collections::BTreeMap::new();
        for (k, v) in campos {
            m.insert(k.to_string(), v.to_string());
        }
        Registro {
            recibido_ms: ms,
            origen: "test".into(),
            evento: Evento {
                gravedad,
                msgid: msgid.into(),
                clase: clase.into(),
                campos: m,
                condiciones: cond.map(|c| c.iter().map(|s| s.to_string()).collect()),
            },
        }
    }

    fn latido(ms: u128, intervalo: &str, cond: Vec<&str>) -> Registro {
        reg(
            ms,
            "latido",
            "latido-de-sensor",
            6,
            &[("intervaloMs", intervalo), ("latido", "1")],
            Some(cond),
        )
    }

    #[test]
    fn calma_cumple() {
        let tr = vec![latido(1000, "60000", vec!["ninguna"])];
        assert_eq!(
            evaluar(&Firma::LatidoEnCalma, &tr, 2000),
            Resultado::Cumplida
        );
    }

    #[test]
    fn calma_sin_latidos_es_nomedible() {
        let tr: Vec<Registro> = vec![];
        matches!(
            evaluar(&Firma::LatidoEnCalma, &tr, 2000),
            Resultado::NoMedible(_)
        );
        assert!(matches!(
            evaluar(&Firma::LatidoEnCalma, &tr, 2000),
            Resultado::NoMedible(_)
        ));
    }

    #[test]
    fn amenaza_con_asiento_cumple() {
        let tr = vec![reg(
            1000,
            "amenaza",
            "amenaza-incontenible",
            1,
            &[("asiento", "1508"), ("dispositivo", "x")],
            None,
        )];
        assert_eq!(
            evaluar(&Firma::AmenazaConAsiento, &tr, 2000),
            Resultado::Cumplida
        );
    }

    #[test]
    fn silencio_detectado_pasado_el_umbral() {
        // último latido en t=1000 con intervalo 60000; a los 200000ms (>2×60000)
        let tr = vec![latido(1000, "60000", vec!["ninguna"])];
        assert_eq!(
            evaluar(&Firma::Silencio { factor: 2.0 }, &tr, 1000 + 200_000),
            Resultado::Cumplida
        );
    }

    #[test]
    fn silencio_no_si_sigue_llegando() {
        let tr = vec![latido(1000, "60000", vec!["ninguna"])];
        // sólo 30s después: dentro de 2×60s
        assert!(matches!(
            evaluar(&Firma::Silencio { factor: 2.0 }, &tr, 1000 + 30_000),
            Resultado::NoCumplida(_)
        ));
    }

    #[test]
    fn silencio_sin_ningun_latido_es_nomedible() {
        let tr: Vec<Registro> = vec![];
        assert!(matches!(
            evaluar(&Firma::Silencio { factor: 2.0 }, &tr, 999_999),
            Resultado::NoMedible(_)
        ));
    }

    #[test]
    fn autochequeo_pa147_usa_intervalo_del_cable() {
        // Cable = 60000; a los 90000ms del último latido. Con el intervalo del
        // cable (2×60000=120000) NO hay silencio → SinSilencioFalso Cumplida.
        // Si el arnés fijara 30000 (2×30000=60000<90000), Silencio sería
        // Cumplida y SinSilencioFalso NoCumplida → el auto-chequeo lo cazaría.
        let tr = vec![latido(1000, "60000", vec!["ninguna"])];
        let r = evaluar(&Firma::SinSilencioFalso { factor: 2.0 }, &tr, 1000 + 90_000);
        assert_eq!(
            r,
            Resultado::Cumplida,
            "con el intervalo del cable no hay silencio a 90s"
        );
        // Prueba de que caza el error: el mismo instante con intervalo 30000
        // (TOML) SÍ daría silencio.
        let tr_malo = vec![latido(1000, "30000", vec!["ninguna"])];
        let r2 = evaluar(
            &Firma::SinSilencioFalso { factor: 2.0 },
            &tr_malo,
            1000 + 90_000,
        );
        assert!(
            matches!(r2, Resultado::NoCumplida(_)),
            "con 30000 el auto-chequeo debe fallar"
        );
    }

    #[test]
    fn silencio_lee_intervalo_del_latido_no_constante() {
        // intervalo 30000 (PA-147): a los 70000ms ya es silencio con 2× (umbral 60000)
        let tr = vec![latido(1000, "30000", vec!["ninguna"])];
        assert_eq!(
            evaluar(&Firma::Silencio { factor: 2.0 }, &tr, 1000 + 70_000),
            Resultado::Cumplida
        );
    }

    fn condicion(ms: u128, nombre: &str, estado: &str, gravedad: u8) -> Registro {
        reg(
            ms,
            "condicion",
            "condicion",
            gravedad,
            &[("condicion", nombre), ("estado", estado)],
            None,
        )
    }

    #[test]
    fn rojo_sin_queja_es_falso_ok_del_lab() {
        // Sólo latidos, ninguna transición de condición → el agente no cazó la
        // manipulación → falso OK del lab → corrida falla.
        let tr = vec![latido(1000, "60000", vec!["ninguna"])];
        let r = evaluar(
            &Firma::Perturbacion {
                condicion: "operativaSuprimida",
                gravedad: 3,
            },
            &tr,
            2000,
        );
        assert!(matches!(r, Resultado::NoCumplida(_)));
        assert_eq!(veredicto(Clase::Rojo, &r), Veredicto::FalsoOkDelLab);
        assert_eq!(
            agregar(&[Veredicto::Correcto, Veredicto::FalsoOkDelLab]),
            Corrida::FalsoOkDelLab
        );
    }

    #[test]
    fn rojo_con_transicion_correcta_es_correcto() {
        let tr = vec![condicion(1000, "operativaSuprimida", "activa", 3)];
        let r = evaluar(
            &Firma::Perturbacion {
                condicion: "operativaSuprimida",
                gravedad: 3,
            },
            &tr,
            2000,
        );
        assert_eq!(r, Resultado::Cumplida);
        assert_eq!(veredicto(Clase::Rojo, &r), Veredicto::Correcto);
    }

    #[test]
    fn rojo_amenaza_incontenible_no_cuenta_como_queja() {
        // El bug corregido: amenaza-incontenible (gravedad 1) NO es la queja de
        // manipulación. Un rojo con sólo esto debe seguir siendo FALSO-OK-LAB.
        let tr = vec![reg(
            1000,
            "amenaza",
            "amenaza-incontenible",
            1,
            &[("asiento", "9")],
            None,
        )];
        let r = evaluar(
            &Firma::Perturbacion {
                condicion: "operativaSuprimida",
                gravedad: 3,
            },
            &tr,
            2000,
        );
        assert!(matches!(r, Resultado::NoCumplida(_)));
        assert_eq!(veredicto(Clase::Rojo, &r), Veredicto::FalsoOkDelLab);
    }

    #[test]
    fn rojo_gravedad_equivocada_es_falso_ok_del_lab() {
        // Esperamos 107 (manipulación) pero el agente emitió 108 (aviso):
        // juzgó mal. NoCumplida → para un rojo, falso OK del lab.
        let tr = vec![condicion(1000, "operativaSuprimida", "activa", 4)];
        let r = evaluar(
            &Firma::Perturbacion {
                condicion: "operativaSuprimida",
                gravedad: 3,
            },
            &tr,
            2000,
        );
        assert!(matches!(r, Resultado::NoCumplida(_)));
        assert_eq!(veredicto(Clase::Rojo, &r), Veredicto::FalsoOkDelLab);
    }

    #[test]
    fn discriminacion_actualizacion_correcta() {
        // accionAdministrativa (108) y ninguna alarma 107 → el agente reconoció
        // la actualización. Verde → Correcto.
        let tr = vec![condicion(1000, "accionAdministrativa", "activa", 4)];
        let f = Firma::Discriminacion {
            benigna: "accionAdministrativa",
            gravedad_benigna: 4,
            no_alarmas: &["inventarioNoVerifica", "inventarioSuprimido"],
        };
        let r = evaluar(&f, &tr, 2000);
        assert_eq!(r, Resultado::Cumplida);
        assert_eq!(veredicto(Clase::Verde, &r), Veredicto::Correcto);
    }

    #[test]
    fn discriminacion_falsa_alarma_es_fallo_del_agente() {
        // El agente acusó inventarioNoVerifica (107) a una actualización
        // legítima → NoCumplida → para un verde, fallo del agente.
        let tr = vec![condicion(1000, "inventarioNoVerifica", "activa", 3)];
        let f = Firma::Discriminacion {
            benigna: "accionAdministrativa",
            gravedad_benigna: 4,
            no_alarmas: &["inventarioNoVerifica", "inventarioSuprimido"],
        };
        let r = evaluar(&f, &tr, 2000);
        assert!(matches!(r, Resultado::NoCumplida(_)));
        assert_eq!(veredicto(Clase::Verde, &r), Veredicto::FalloDelAgente);
    }

    #[test]
    fn rojo_sin_clave_recuperacion_es_108() {
        // "sin clave de recuperación" NO es manipulación → 108 (gravedad 4).
        let tr = vec![condicion(1000, "recuperacionNoAprovisionada", "activa", 4)];
        let r = evaluar(
            &Firma::Perturbacion {
                condicion: "recuperacionNoAprovisionada",
                gravedad: 4,
            },
            &tr,
            2000,
        );
        assert_eq!(r, Resultado::Cumplida);
    }

    #[test]
    fn rojo_llego_tarde_es_nomedible() {
        // No capturamos la transición, pero la condición persiste en el latido
        // → no se puede juzgar la gravedad → NoMedible (no falso OK).
        let mut lat = latido(5000, "60000", vec!["operativaSuprimida"]);
        lat.recibido_ms = 5000;
        let tr = vec![lat];
        let r = evaluar(
            &Firma::Perturbacion {
                condicion: "operativaSuprimida",
                gravedad: 3,
            },
            &tr,
            6000,
        );
        assert!(matches!(r, Resultado::NoMedible(_)));
        assert_eq!(veredicto(Clase::Rojo, &r), Veredicto::NoConcluyente);
    }

    #[test]
    fn verde_no_cumplido_es_fallo_del_agente() {
        let tr: Vec<Registro> = vec![reg(
            1,
            "amenaza",
            "amenaza-incontenible",
            1,
            &[("asiento", "1")],
            None,
        )];
        // esperábamos calma, llegó amenaza y ningún latido → NoMedible (no latidos)
        let r = evaluar(&Firma::LatidoEnCalma, &tr, 2);
        assert!(matches!(r, Resultado::NoMedible(_)));
        assert_eq!(veredicto(Clase::Verde, &r), Veredicto::NoConcluyente);
    }

    #[test]
    fn agregar_precedencia() {
        assert_eq!(
            agregar(&[Veredicto::Correcto, Veredicto::Correcto]),
            Corrida::Aprobada
        );
        assert_eq!(
            agregar(&[Veredicto::Correcto, Veredicto::NoConcluyente]),
            Corrida::NoConcluyente
        );
        assert_eq!(
            agregar(&[Veredicto::FalloDelAgente, Veredicto::NoConcluyente]),
            Corrida::FalloDelAgente
        );
        assert_eq!(
            agregar(&[Veredicto::FalloDelAgente, Veredicto::FalsoOkDelLab]),
            Corrida::FalsoOkDelLab
        );
    }

    #[test]
    fn jsonl_del_sink_se_parsea() {
        // Formato real que emite el sink: {recibido_ms, origen, evento:{...}}
        let linea = r#"{"recibido_ms":1791489805347,"origen":"127.0.0.1:49160","evento":{"pri":110,"facilidad":13,"gravedad":6,"version":1,"ts":"2026-09-27T03:40:12.418Z","host":"sensor","app":"eje-agente","procid":"-","msgid":"latido-de-sensor","clase":"latido","campos":{"intervaloMs":"60000","latido":"842"},"condiciones":["ninguna"],"msg_crudo":"x"}}"#;
        let tr = parse_jsonl(linea).unwrap();
        assert_eq!(tr.len(), 1);
        assert_eq!(
            evaluar(&Firma::LatidoEnCalma, &tr, 2_000_000_000_000),
            Resultado::Cumplida
        );
    }
}
