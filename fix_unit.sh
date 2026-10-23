#!/bin/sh
set -eu
PW="$(cat /tmp/.p)"
rm -rf /tmp/pkg2; mkdir -p /tmp/pkg2
tar -xzf /tmp/eje-agente.tar.gz -C /tmp/pkg2
echo "=== unit de ORIGEN (tarball) ==="; wc -c /tmp/pkg2/eje-agente/eje-agente.service; sha256sum /tmp/pkg2/eje-agente/eje-agente.service
echo "=== dest ANTES ==="; ls -la /etc/systemd/system/eje-agente.service
printf "%s\n" "$PW" | sudo -S install -m 0644 /tmp/pkg2/eje-agente/eje-agente.service /etc/systemd/system/eje-agente.service
printf "%s\n" "$PW" | sudo -S systemctl daemon-reload
echo "=== dest DESPUES (debe 2815 B / f6f965c5...) ==="; ls -la /etc/systemd/system/eje-agente.service; sha256sum /etc/systemd/system/eje-agente.service
echo "=== estado ==="; systemctl is-enabled eje-agente 2>&1; systemctl status eje-agente --no-pager 2>&1 | head -6
printf "%s\n" "$PW" | sudo -S rm -f /tmp/.p
rm -rf /tmp/pkg2 /tmp/eje-agente.tar.gz /tmp/fix_unit.sh
echo FIN_FIXUNIT
