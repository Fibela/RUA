//! Colector de syslog del laboratorio RUA — lógica de marco y parseo.
//!
//! Contrato del agente Eje-Latam (fijado por su equipo, no inventado aquí):
//!
//! - Transporte: TCP en claro, una conexión por mensaje. Sin puerto por
//!   omisión.
//! - Marco: RFC 6587 *octet counting* — `«longitud» SP «frame»`. NO hay
//!   terminación por salto de línea; es deliberado (un `\n` dentro del mensaje
//!   inyectaría una línea de syslog completa). Este colector RECHAZA de plano
//!   cualquier marco que no empiece por el conteo de octetos.
//! - Header: RFC 5424 `<PRI>1 TS HOST eje-agente - MSGID - MSG`. PROCID y
//!   STRUCTURED-DATA son `-`. PRI = facilidad 13 (log audit) × 8 + gravedad.
//! - Cuerpo (MSG): texto plano `clave=valor` separado por espacios. No hay
//!   JSON ni structured-data. `condiciones` puede ser `ninguna` (afirmación) o
//!   varias separadas por coma sin espacios. Los caracteres de control ya
//!   vienen sustituidos por espacio en origen.
//!
//! Este módulo no interpreta ni ejecuta nada del contenido: lo trata como dato
//! no confiable, lo parsea de forma defensiva y lo deja listo para almacenar.

use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{BufRead, Read};

/// Tope defensivo del tamaño de un marco. Un `longitud` declarado mayor se
/// rechaza sin intentar reservar memoria (anti-DoS por conteo mentiroso).
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// Facilidad esperada del agente (RFC 5424: log audit = 13).
pub const FACILIDAD_ESPERADA: u8 = 13;

#[derive(Debug, PartialEq)]
pub enum FrameError {
    /// EOF limpio antes de empezar un marco (la conexión se cerró).
    Eof,
    /// El flujo no empieza por el conteo de octetos (p. ej. llega `<` directo,
    /// o alguien intenta delimitar por `\n`). Se rechaza por contrato.
    NoOctetCounting,
    /// El conteo declarado excede MAX_FRAME_BYTES, o es cero.
    LongitudInvalida(String),
    /// Error de E/S o marco truncado.
    Io(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Eof => write!(f, "eof"),
            FrameError::NoOctetCounting => write!(
                f,
                "marco no RFC 6587 (octet counting): se rechaza delimitación por salto de línea u otra"
            ),
            FrameError::LongitudInvalida(s) => write!(f, "longitud de marco inválida: {s}"),
            FrameError::Io(s) => write!(f, "e/s: {s}"),
        }
    }
}

/// Lee un marco RFC 6587 con conteo de octetos desde `r`.
///
/// Formato: una o más cifras ASCII (la longitud), un espacio, y exactamente
/// esa cantidad de octetos (el SYSLOG-MSG). Devuelve los bytes del mensaje.
/// Devuelve `Err(FrameError::Eof)` cuando no hay más marcos.
pub fn read_frame<R: BufRead>(r: &mut R) -> Result<Vec<u8>, FrameError> {
    // 1) Leer las cifras de la longitud, byte a byte, hasta el espacio.
    let mut len_digits: Vec<u8> = Vec::with_capacity(8);
    loop {
        let mut b = [0u8; 1];
        match r.read(&mut b) {
            Ok(0) => {
                if len_digits.is_empty() {
                    return Err(FrameError::Eof);
                } else {
                    return Err(FrameError::Io("marco truncado en la longitud".into()));
                }
            }
            Ok(_) => {}
            Err(e) => return Err(FrameError::Io(e.to_string())),
        }
        let c = b[0];
        if c == b' ' {
            if len_digits.is_empty() {
                return Err(FrameError::NoOctetCounting);
            }
            break;
        }
        if !c.is_ascii_digit() {
            // El primer byte no es dígito → no es octet counting. Esto atrapa
            // el intento de mandar `<PRI>...` directo o delimitar por `\n`.
            return Err(FrameError::NoOctetCounting);
        }
        if len_digits.len() >= 10 {
            return Err(FrameError::LongitudInvalida("demasiadas cifras".into()));
        }
        len_digits.push(c);
    }

    // 2) Interpretar y acotar la longitud ANTES de reservar nada.
    let len_str = String::from_utf8_lossy(&len_digits);
    let len: usize = len_str
        .parse()
        .map_err(|_| FrameError::LongitudInvalida(len_str.to_string()))?;
    if len == 0 {
        return Err(FrameError::LongitudInvalida("cero".into()));
    }
    if len > MAX_FRAME_BYTES {
        return Err(FrameError::LongitudInvalida(format!(
            "{len} excede el tope {MAX_FRAME_BYTES}"
        )));
    }

    // 3) Leer exactamente `len` octetos.
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)
        .map_err(|e| FrameError::Io(format!("marco truncado: {e}")))?;
    Ok(buf)
}

/// Un evento ya parseado, listo para almacenar/validar.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Evento {
    pub pri: u16,
    pub facilidad: u8,
    pub gravedad: u8,
    pub version: u8,
    pub ts: String,
    pub host: String,
    pub app: String,
    pub procid: String,
    pub msgid: String,
    /// Clasificación por MSGID conocido.
    pub clase: Clase,
    /// Campos `clave=valor` del cuerpo, en orden de aparición preservado por el
    /// mapa ordenado (para lectura estable).
    pub campos: BTreeMap<String, String>,
    /// `condiciones` desdoblada por comas, si el campo está presente. `ninguna`
    /// se conserva literal (es una afirmación, no ausencia).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condiciones: Option<Vec<String>>,
    /// MSG crudo, tras sanitizar caracteres de control a espacio (defensa en
    /// profundidad; el origen ya lo hace, no confiamos en ello).
    pub msg_crudo: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Clase {
    /// MSGID `latido-de-sensor` (informativo, PRI 110).
    Latido,
    /// MSGID `sello-de-evidencia` (informativo, PRI 110).
    SelloEvidencia,
    /// MSGID `amenaza-incontenible` (alerta, PRI 105). Dispositivo del segmento
    /// que no se puede contener; NUNCA sale por manipulación del almacén.
    Amenaza,
    /// MSGID `condicion` — transición de condición: `condicion=<nombre>
    /// estado=activa|resuelta`. La gravedad se deriva (Error/107 si es
    /// manipulación que se activa, Aviso/108 si no-manipulación que se activa,
    /// Informativo/110 si se resuelve). Es la señal de las quejas del agente.
    Condicion,
    /// Cualquier otro MSGID: se guarda tal cual, no se clasifica.
    Otro,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Formato(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Formato(s) => write!(f, "formato syslog inválido: {s}"),
        }
    }
}

/// Sustituye todo carácter de control (incluido `\n`, `\r`, `\t`) por espacio.
fn sanitizar(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Parsea un SYSLOG-MSG RFC 5424 (ya extraído del marco) al `Evento`.
pub fn parse_syslog(raw: &[u8]) -> Result<Evento, ParseError> {
    let texto = String::from_utf8_lossy(raw);
    let texto = sanitizar(&texto);

    // <PRI>
    if !texto.starts_with('<') {
        return Err(ParseError::Formato("no empieza por '<PRI>'".into()));
    }
    let cierre = texto
        .find('>')
        .ok_or_else(|| ParseError::Formato("falta '>' del PRI".into()))?;
    let pri: u16 = texto[1..cierre]
        .parse()
        .map_err(|_| ParseError::Formato("PRI no numérico".into()))?;
    let facilidad = (pri >> 3) as u8;
    let gravedad = (pri & 7) as u8;

    // Resto: "1 TS HOST APP PROCID MSGID SD MSG..."
    let resto = &texto[cierre + 1..];
    let partes: Vec<&str> = resto.splitn(8, ' ').collect();
    if partes.len() < 7 {
        return Err(ParseError::Formato("header RFC 5424 incompleto".into()));
    }
    let version: u8 = partes[0]
        .parse()
        .map_err(|_| ParseError::Formato("VERSION no numérica".into()))?;
    let ts = partes[1].to_string();
    let host = partes[2].to_string();
    let app = partes[3].to_string();
    let procid = partes[4].to_string();
    let msgid = partes[5].to_string();
    // partes[6] = STRUCTURED-DATA (esperado "-"); no lo usamos.
    let msg = partes.get(7).copied().unwrap_or("").to_string();

    let clase = match msgid.as_str() {
        "latido-de-sensor" => Clase::Latido,
        "sello-de-evidencia" => Clase::SelloEvidencia,
        "amenaza-incontenible" => Clase::Amenaza,
        "condicion" => Clase::Condicion,
        _ => Clase::Otro,
    };

    let (campos, condiciones) = parse_cuerpo(&msg);

    Ok(Evento {
        pri,
        facilidad,
        gravedad,
        version,
        ts,
        host,
        app,
        procid,
        msgid,
        clase,
        campos,
        condiciones,
        msg_crudo: msg,
    })
}

/// Parsea el cuerpo `clave=valor`. Un token sin `=` se une (con espacio) al
/// valor anterior, de modo que un campo final de texto libre con espacios
/// (p. ej. `detalle=...`) no rompe el parseo.
fn parse_cuerpo(msg: &str) -> (BTreeMap<String, String>, Option<Vec<String>>) {
    let mut campos: BTreeMap<String, String> = BTreeMap::new();
    let mut ultimo_clave: Option<String> = None;

    for token in msg.split(' ') {
        if token.is_empty() {
            continue;
        }
        match token.split_once('=') {
            Some((k, v)) if !k.is_empty() && !k.contains(char::is_whitespace) => {
                campos.insert(k.to_string(), v.to_string());
                ultimo_clave = Some(k.to_string());
            }
            _ => {
                // Continuación del valor anterior (texto libre con espacios).
                if let Some(k) = &ultimo_clave {
                    if let Some(v) = campos.get_mut(k) {
                        v.push(' ');
                        v.push_str(token);
                    }
                }
                // Si no hay valor anterior, se ignora el token suelto.
            }
        }
    }

    let condiciones = campos
        .get("condiciones")
        .map(|v| v.split(',').map(|s| s.to_string()).collect());

    (campos, condiciones)
}

/// Codifica un mensaje como marco RFC 6587 (conteo de octetos). Útil para
/// pruebas y para herramientas que inyecten eventos sintéticos al colector.
pub fn encode_frame(msg: &str) -> Vec<u8> {
    let bytes = msg.as_bytes();
    let mut out = format!("{} ", bytes.len()).into_bytes();
    out.extend_from_slice(bytes);
    out
}

/// Lee todos los marcos disponibles de un flujo hasta EOF, parseando cada uno.
/// Un marco corrupto corta el flujo (devuelve lo acumulado y el error).
pub fn leer_todos<R: Read>(r: R) -> (Vec<Evento>, Option<String>) {
    let mut br = std::io::BufReader::new(r);
    let mut out = Vec::new();
    loop {
        match read_frame(&mut br) {
            Ok(bytes) => match parse_syslog(&bytes) {
                Ok(ev) => out.push(ev),
                Err(e) => return (out, Some(e.to_string())),
            },
            Err(FrameError::Eof) => return (out, None),
            Err(e) => return (out, Some(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // Nota: en la realidad `condiciones=sinColector` NO viaja por syslog
    // (solo por el socket local); aquí el valor es arbitrario a propósito,
    // para probar que el parser acepta cualquier token sin interpretarlo.
    const LATIDO: &str = "<110>1 2026-09-27T03:40:12.418Z sensor-eje-prueba eje-agente - latido-de-sensor - latido=842 interfaz=eth1 sello=9f3ca21b asiento=1507 intervaloMs=60000 condiciones=ninguna";

    #[test]
    fn framing_roundtrip() {
        let framed = encode_frame(LATIDO);
        let mut cur = Cursor::new(framed);
        let bytes = read_frame(&mut cur).unwrap();
        assert_eq!(bytes, LATIDO.as_bytes());
        // Siguiente lectura: EOF.
        assert_eq!(read_frame(&mut cur), Err(FrameError::Eof));
    }

    #[test]
    fn rechaza_delimitacion_por_salto_de_linea() {
        // Un mensaje "crudo" sin conteo de octetos, al estilo \n-delimitado.
        let crudo = format!("{LATIDO}\n");
        let mut cur = Cursor::new(crudo.into_bytes());
        assert_eq!(read_frame(&mut cur), Err(FrameError::NoOctetCounting));
    }

    #[test]
    fn rechaza_longitud_gigante() {
        let mut cur = Cursor::new(b"999999999 x".to_vec());
        match read_frame(&mut cur) {
            Err(FrameError::LongitudInvalida(_)) => {}
            otro => panic!("esperaba LongitudInvalida, fue {otro:?}"),
        }
    }

    #[test]
    fn parsea_latido() {
        let ev = parse_syslog(LATIDO.as_bytes()).unwrap();
        assert_eq!(ev.pri, 110);
        assert_eq!(ev.facilidad, 13);
        assert_eq!(ev.gravedad, 6);
        assert_eq!(ev.version, 1);
        assert_eq!(ev.host, "sensor-eje-prueba");
        assert_eq!(ev.app, "eje-agente");
        assert_eq!(ev.msgid, "latido-de-sensor");
        assert_eq!(ev.clase, Clase::Latido);
        assert_eq!(ev.campos.get("latido").unwrap(), "842");
        assert_eq!(ev.campos.get("asiento").unwrap(), "1507");
        // En calma, `condiciones=ninguna` es una afirmación, no ausencia.
        assert_eq!(
            ev.condiciones.as_ref().unwrap(),
            &vec!["ninguna".to_string()]
        );
    }

    #[test]
    fn condiciones_multiples_por_coma() {
        let m = LATIDO.replace("condiciones=ninguna", "condiciones=unaCosa,otraCosa");
        let ev = parse_syslog(m.as_bytes()).unwrap();
        assert_eq!(
            ev.condiciones.unwrap(),
            vec!["unaCosa".to_string(), "otraCosa".to_string()]
        );
    }

    #[test]
    fn parsea_sello_de_evidencia() {
        let sello = "<110>1 2026-09-27T03:45:00.000Z sensor-eje-prueba eje-agente - sello-de-evidencia - sello=ab12cd34 asiento=1509 interfaz=eth1";
        let ev = parse_syslog(sello.as_bytes()).unwrap();
        assert_eq!(ev.gravedad, 6);
        assert_eq!(ev.clase, Clase::SelloEvidencia);
        assert_eq!(ev.campos.get("sello").unwrap(), "ab12cd34");
    }

    #[test]
    fn parsea_alerta() {
        let alerta = "<105>1 2026-09-27T03:41:55.002Z sensor-eje-prueba eje-agente - amenaza-incontenible - asiento=1508 dispositivo=00:1a:2b:3c:4d:7f detalle=cae fuera de linea base";
        let ev = parse_syslog(alerta.as_bytes()).unwrap();
        assert_eq!(ev.pri, 105);
        assert_eq!(ev.gravedad, 1); // alerta
        assert_eq!(ev.clase, Clase::Amenaza);
        assert_eq!(ev.campos.get("dispositivo").unwrap(), "00:1a:2b:3c:4d:7f");
        // detalle con espacios: la continuación lo reensambla completo.
        assert_eq!(ev.campos.get("detalle").unwrap(), "cae fuera de linea base");
    }

    #[test]
    fn msgid_desconocido_es_otro() {
        let m = LATIDO.replace("latido-de-sensor", "sello-de-registro");
        let ev = parse_syslog(m.as_bytes()).unwrap();
        assert_eq!(ev.clase, Clase::Otro);
        assert_eq!(ev.msgid, "sello-de-registro");
    }

    #[test]
    fn parsea_condicion_de_manipulacion() {
        // PRI 107 = facilidad 13 × 8 + gravedad 3 (Error): manipulación activa.
        let c = "<107>1 2026-09-27T03:50:00.000Z sensor eje-agente - condicion - condicion=operativaSuprimida estado=activa";
        let ev = parse_syslog(c.as_bytes()).unwrap();
        assert_eq!(ev.clase, Clase::Condicion);
        assert_eq!(ev.gravedad, 3);
        assert_eq!(ev.campos.get("condicion").unwrap(), "operativaSuprimida");
        assert_eq!(ev.campos.get("estado").unwrap(), "activa");
    }

    #[test]
    fn leer_todos_varios_marcos() {
        let mut flujo = encode_frame(LATIDO);
        flujo.extend(encode_frame(LATIDO));
        let (evs, err) = leer_todos(Cursor::new(flujo));
        assert!(err.is_none());
        assert_eq!(evs.len(), 2);
    }
}
