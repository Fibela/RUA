# RUA — núcleo (fase 1)

Wrapper determinista de VBoxManage expuesto como servidor JSON-RPC/MCP por
stdio. La IA (a futuro) habla por este canal; **nunca** recibe una shell del
host.

## Compilar y probar (en Windows, con tu toolchain)

```powershell
cd C:\Users\Alexx\Aura\nucleo
cargo test            # 8 tests: token ed25519 + cadena del log
cargo build --release
copy rua.config.example.json rua.config.json   # ajusta nombres/rutas
```

## Operaciones (métodos JSON-RPC)

| Método          | Peligrosa | Notas |
|-----------------|-----------|-------|
| `rua.list_vms`  | no        | `VBoxManage list vms` |
| `rua.revert`    | no        | solo snapshots declarados en config |
| `rua.start`     | no        | `startvm --type headless` |
| `rua.poweroff`  | no        | `controlvm poweroff` |
| `rua.collect`   | no        | `guestcontrol copyfrom` (guest→host) |
| `rua.copy_to`   | **sí**    | `guestcontrol copyto` (entrega payload) |
| `rua.run`       | **sí**    | `guestcontrol run` (ejecuta payload) |

## Candado de aprobación

- **Nivel bajo** (`risk_level: "bajo"`): las operaciones peligrosas se rechazan
  por construcción. Validar Eje-Latam y probar ASC no las necesitan.
- **Nivel alto** (`risk_level: "alto"` + `allow_dangerous: true`): cada
  operación peligrosa exige un `approval_token` firmado (ed25519) por el
  aprobador host-side. El núcleo solo tiene la pública (`approval_pubkey_hex`).

El campo `confirm` que mande el cliente **no se lee nunca**. La decisión no
puede entrar por el mismo canal que la petición.

El token liga el permiso a la petición exacta (vm + exe + args + hash del
payload), con nonce de un solo uso y expiración contra el reloj del host.
El binario que acuña tokens (`rua-approve`) es de la fase 5; aquí está cableada
solo la verificación.

## Log de auditoría

`audit_log_path`: una línea JSON por evento, encadenada por SHA-256
(`prev_hash` + contenido). Alterar o borrar una línea intermedia rompe la
cadena. Limitación conocida: no detecta truncado de la cola (ver `audit.rs`);
el ancla externa se añade cuando el log sea evidencia de cliente (fase 4+).

## Pendiente antes de cerrar la fase

- Verificar las banderas de credenciales de `guestcontrol` (`--username` /
  `--password-file`) contra `VBoxManage guestcontrol --help` en tu 7.2.14; se
  centralizan en `vbox.rs`.
- Prueba contra un VirtualBox real: `rua.list_vms` y un `rua.revert` sobre la
  golden.
