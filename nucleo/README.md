# rua-nucleo

Núcleo determinista de RUA: un servidor **MCP** que envuelve `VBoxManage` con
un conjunto **cerrado** de operaciones. No hay IA aquí; esto solo ejecuta
comandos fijos de VirtualBox y registra todo. La IA vive más arriba (la capa
de traducción) y solo puede pedir estas operaciones por nombre: nunca recibe
una shell del host.

Probado contra **VirtualBox 7.2.14** (CLI 7.x).

## Qué hace

Habla MCP por **stdio** (JSON-RPC 2.0, un mensaje JSON por línea) y expone 7
herramientas: `rua.list_vms`, `rua.snapshot_restore`, `rua.start`, `rua.stop`,
`rua.copy_in`, `rua.collect`, `rua.run`.

## Los controles de seguridad

1. **Allowlist de VMs.** El núcleo solo opera VMs declaradas en `vms` de la
   config. Cualquier otra se rechaza.
2. **Snapshots declarados.** `snapshot_restore` solo acepta snapshots listados
   para esa VM.
3. **Aprobación host-side (fuera del canal MCP).** `rua.run` con
   `dangerous=true` (detonar una muestra) **no** se aprueba con un campo que
   mande el cliente. En nivel **bajo** es imposible por construcción. En nivel
   **alto** exige un `approval_token` firmado con la llave privada ed25519 del
   aprobador del host; el núcleo solo tiene la **pública** (`approval_pubkey_hex`),
   así que la IA, aunque lea la config, no puede forjarlo. El token liga el
   permiso a esa petición concreta (vm + programa + argumentos), con nonce de un
   solo uso y expiración.
4. **Log de auditoría encadenado.** Cada línea lleva `prev_hash` y su propio
   `hash` (SHA-256). Borrar o alterar una línea pasada rompe la cadena.

## Configuración

Copia `rua.config.example.json` a `rua.config.json`. El núcleo la busca en
`rua.config.json` o en la ruta de `RUA_CONFIG`.

- `vboxmanage_path`: ruta a `VBoxManage.exe`.
- `risk_level`: `"bajo"` o `"alto"`.
- `allow_dangerous`: solo tiene efecto en nivel alto.
- `approval_pubkey_hex`: clave pública ed25519 del aprobador (32 bytes en hex).
  `null` hasta que exista el aprobador; sin ella no hay nivel alto posible.
- `vms`: la allowlist, con sus snapshots permitidos.
- `audit_log`: archivo del log encadenado.

## Compilar, probar y correr

```powershell
cd nucleo
cargo test            # incluye la verificación del token de aprobación
cargo build --release
$env:RUA_CONFIG = "C:\Users\Alexx\Aura\nucleo\rua.config.json"
.\target\release\rua-nucleo.exe
```

El binario queda a la espera de mensajes MCP por stdin; normalmente lo lanza un
cliente MCP como subproceso.

## Autenticación de guestcontrol

`copy_in`, `collect` y `run` entran a la VM con `--username` y
**`--password-file`** (ruta a un archivo con la contraseña), nunca la
contraseña en claro. Esos `*.pass` están en el `.gitignore`.

## Pendientes

- **Aprobador (rua-approve):** el binario que acuña los tokens firmados se
  construye en la fase 5 (nivel alto). Aquí ya está cableada la verificación.
- **Sink de syslog:** siguiente pieza de la fase 2 (el colector local al que
  emite la golden, y sobre el que afirma la validación).
- **Anti-replay:** los nonces usados se recuerdan por proceso. Persistirlos
  entre reinicios (contra el log) queda para cuando llegue el aprobador.
- Falta probarlo contra un VirtualBox real en el host (compila y pasa tests).
