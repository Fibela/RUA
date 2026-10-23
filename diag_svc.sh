#!/bin/sh
echo "=== ls unit ==="; ls -la /etc/systemd/system/eje-agente.service
echo "=== readlink -f ==="; readlink -f /etc/systemd/system/eje-agente.service
echo "=== status ==="; systemctl status eje-agente --no-pager 2>&1 | head -12
echo "=== buscar unidades/masks eje-agente ==="; find /etc/systemd/system /run/systemd/system /usr/lib/systemd/system -iname "eje-agente*" 2>/dev/null | while read f; do ls -la "$f"; done
echo "=== is-enabled ==="; systemctl is-enabled eje-agente 2>&1
echo "FIN_DIAGSVC"
