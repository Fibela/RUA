# RUA

Laboratorio de máquinas virtuales aisladas de Corporación Premos. Ejecuta
software controlado o peligroso en VMs, recolecta lo que hace y verifica si
nuestras herramientas lo detectaron. Infraestructura interna, no un producto
que se venda por sí solo.

El documento de arranque (qué es, para qué sirve, arquitectura, fases) vive
como doc de Claude y es la referencia conceptual. Este repo es la
implementación.

## Estructura

```
Aura/
  Cargo.toml          Workspace de Rust
  nucleo/             El núcleo: wrapper de VBoxManage como servidor MCP
  (próximamente)      aprovisionamiento de golden, arnés de validación, sink de syslog
```

## Estado

- **Fase 1 — núcleo:** hecho un primer cut. Ver `nucleo/README.md`.
- **Fase 2 — validación de Eje-Latam:** siguiente. Agente dentro de una golden
  **Windows**; emite por syslog a un **sink local**; la validación afirma el
  estado (Conforme / Violación / Comprobación imposible) y el evento correctos.

## Decisiones fijadas

- Núcleo determinista (sin IA), operaciones cerradas por MCP, log de auditoría,
  candado humano para detonar.
- Nivel bajo (validar Eje-Latam, probar ASC) con red interna; nivel alto
  (ransomware, malware real) con internet simulado y aprobación obligatoria.
- Una sola base **Windows** por capas: +agente Eje-Latam (postura) / +Sysmon
  +muestras (ASC, ransomware). Los tres estados son snapshots de esa base.
- Criptografía: estándares (AEAD). No se inventa cifrado propio para producto.
