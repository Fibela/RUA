#!/bin/sh
set -eu
PW="$(cat /tmp/.p)"
printf "%s\n" "$PW" | sudo -S systemctl stop eje-agente 2>/dev/null || true
printf "%s\n" "$PW" | sudo -S systemctl reset-failed eje-agente 2>/dev/null || true
rm -rf /tmp/pkg3; mkdir -p /tmp/pkg3; tar -xzf /tmp/eje-agente.tar.gz -C /tmp/pkg3
echo "origen:"; wc -c /tmp/pkg3/eje-agente/eje-agente; sha256sum /tmp/pkg3/eje-agente/eje-agente
echo "dest ANTES:"; ls -la /usr/local/bin/eje-agente
printf "%s\n" "$PW" | sudo -S install -m 0755 /tmp/pkg3/eje-agente/eje-agente /usr/local/bin/eje-agente
echo "dest DESPUES (920064 / dba88063 / ELF):"; ls -la /usr/local/bin/eje-agente; sha256sum /usr/local/bin/eje-agente; file /usr/local/bin/eje-agente
printf "%s\n" "$PW" | sudo -S systemctl daemon-reload
echo "is-active (debe inactive):"; printf "%s\n" "$PW" | sudo -S systemctl is-active eje-agente 2>/dev/null
printf "%s\n" "$PW" | sudo -S rm -f /tmp/.p
rm -rf /tmp/pkg3 /tmp/eje-agente.tar.gz /tmp/fix_bin.sh
echo FIN_FIXBIN
