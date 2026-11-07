# Receta de reconstruccion de los snapshots de `golden-eje-linux`

Los snapshots son estado de VM y no viven en git. **Esta receta si.** Sin ella una
medicion es una observacion de una sola vez en una sola maquina. Ata *que* se probo,
no *que* se probara (PA-14c).

VM: `golden-eje-linux` · hostname `eje-golden-01` · nic1 host-only (`enp0s3`,
host `192.168.56.1`) · nic2 intnet aislada `eje-pa61-aislada` (`enp0s8`) ·
colector/sink en `192.168.56.1:5514`.

## Arbol y que script construye cada nodo

```
base-limpia (6c23d098)
  └ base-identidad-anclada (d520dd87)        <- run_prov / run_provision (pasos 3-5)
      └ base-agente-pa163 (b66a08c3)         <- run_7a1.bat + run_7a2.bat
          └ base-inventario-aceptado (fea9e632) <- run_7b1.bat + run_7b2.bat
```

`run_d2.bat` NO produce snapshot: es una medicion destructiva sobre
`base-agente-pa163` (voltea 1 byte de la config firmada). Se descarta el estado y
se restaura el snapshot al terminar.

## Verificacion por nodo (sha256 **en destino**, no en origen)

| Nodo | Artefacto que lo define | sha256 |
|---|---|---|
| `base-identidad-anclada` | `inventario.inv` | `23fffe6e…` |
| | `agente.conf.firmado` | `1f95c1b5…` |
| | `centinela.dat` (sin marca) | `11b0d10a…` |
| `base-agente-pa163` | `/usr/local/bin/eje-agente` | `b282c4bb…` |
| `base-inventario-aceptado` | `centinela.dat` (marca inventario sec 1) | `ffb9b1c8e2387c551d8c1c590d1305e86ff32c3b46f285e5b6cf29e3ae5c331d` |

Los valores con `…` son prefijos; el hash completo se comprueba en el guest con
`sha256sum` y se compara contra el artefacto de la ceremonia. Un nodo sin SHA
verificado en destino no cuenta como reconstruido.

## Precondiciones de cualquier medicion con sink

1. **Tunel VPN CAIDO.** Hay NordVPN en el host. Si el tunel esta arriba se mide el
   tunel, no el cable. Bajarlo antes de medir y **anotar su estado en la hoja de la
   medicion**, junto al estado del cortafuegos.
2. **Cortafuegos del host.** El adaptador host-only cae en perfil **Public**:
   `Set-NetFirewallProfile -Name Public -AllowInboundRules True`, y comprobar que no
   existe regla BLOCK por programa para `rua-sink.exe` (block gana a allow).
   **Revertir a False al cerrar la sesion** — no se deja entrada abierta cruzando
   sesiones. (Estado 04-nov-2026: cero reglas de salida en bloqueo.)
3. Las mediciones de banner (7a, D2) no necesitan sink; la calma de 7b si.

## Reglas de operacion que no se negocian

- **Nunca `poweroff` duro antes de un snapshot.** `sync` + `savestate`. El apagado
  duro corrompe por delalloc en ext4. `poweroff`/`discardstate` solo para DESCARTAR
  estado y volver a un snapshot limpio.
- **Credenciales por fichero, nunca inline.** `--username vboxuser --passwordfile
  <ruta>\secrets\guest.pwd` (ese fichero esta en .gitignore). En el guest, `sudo -S`
  se alimenta de `/tmp/.p` y el script lo borra al final.
- `guestcontrol run --exe X -- ARGS`: VBoxManage pone argv0=X solo; no repetir el
  nombre del programa. Todo comando de guest por **ruta de script** (`-- /tmp/x.sh`),
  porque PowerShell se come el `--` y los argumentos con guion fallan.
- Tras reanudar un `savestate`, esperar ~25 s a que asienten las guest additions y
  reintentar cada comando: si no, `VERR_DUPLICATE`.
- En PowerShell, `Gv` es alias de `Get-Variable` (los alias ganan a las funciones):
  usar `Invoke-Vbx` como nombre del envoltorio.

## Capturas

Las salidas (`sink-*.jsonl`) son resultado, no receta: estan en .gitignore. Lo que se
versiona es el script que las produce y esta hoja.

## Toolchain de construccion — CONFUNDIDOR DECLARADO

Medido sobre el propio ELF, no supuesto:

| Artefacto | sha256 | rustc que lo construyo | Fecha del binario |
|---|---|---|---|
| `eje-agente` de la golden | `b282c4bb…` | **1.85.1** (`/rustc/4eb161250e340c8f48f66e2b929ef4a5bed7c181`) | 24-oct-2026 |

Metodo: `sha256sum` del binario + cadena de version embebida y ruta `/rustc/<commit>`
extraidas con `strings`. Reproducible con el propio binario en mano.

**Los tres compiladores en juego no coinciden:**

- El binario del sensor que mide el laboratorio: **1.85.1**.
- Lo que el CI de Eje-Latam bendice hoy: **1.88**.
- Lo que `Fibela/RUA` pina para sus propios crates: **1.97.0**.

**Por que importa, y cuanto.** Dos cosas distintas, que no hay que mezclar:

1. **Para PA-169 no es causa plausible.** PA-169 es un defecto de enrutado a nivel de fuente
   (`syslog: None` en la rama `NoVerifica`), determinista; la version del compilador no lo
   produce ni lo tapa. La medicion del Paso 0 sigue siendo valida *declarando esto*.
2. **Lo que si importa es la FECHA del build.** `b282c4bb` es del 24-oct. Antes de correr el
   Paso 0 hay que confirmar que los cuatro anclajes —`salida.rs:579`, `main.rs:632`,
   `main.rs:671`, `main.rs:713`— no han cambiado entre la fuente de ese binario y HEAD. Si
   cambiaron, el Paso 0 mediria comportamiento rancio y hay que reconstruir la cadena antes.

**Y un hecho aparte, mas serio que el desajuste:** 1.85 es justamente el pin que impedia
remediar **RUSTSEC-2026-0385** (saltos sobre valores secretos en las primitivas de tiempo
constante bajo ML-KEM y ML-DSA, RPT-109/110). El sensor de la golden es, por tanto, un build
**pre-remediacion** de ese aviso. Para PA-169 da igual; para los otros usos del laboratorio
—validar detecciones de ASC, demo de ransomware, analisis de malware— no da igual.

**Tarea pendiente, deliberada y aparte:** reconstruir la cadena desde HEAD con el toolchain que
CI bendice. Cambia el sha256 del binario, asi que **invalida `b282c4bb`, `base-agente-pa163` y
`base-inventario-aceptado`**: es un rehacer completo con re-medicion de procedencia. Se agenda
a proposito; no se cuela dentro de otra medicion.
