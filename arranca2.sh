#!/bin/sh
set +e
PW="$(cat /tmp/.p)"
echo "=== preflight config/artefactos ==="
ls -la /etc/eje-latam/ 2>&1
echo "--- binario ---"; ls -la /usr/local/bin/eje-agente; file /usr/local/bin/eje-agente
echo "--- unit ---"; printf "is-enabled="; printf "%s\n" "$PW" | sudo -S systemctl is-enabled eje-agente 2>/dev/null; wc -c /etc/systemd/system/eje-agente.service
echo "=== START (observado) ==="
printf "%s\n" "$PW" | sudo -S systemctl start eje-agente 2>/dev/null
sleep 3
printf "is-active="; printf "%s\n" "$PW" | sudo -S systemctl is-active eje-agente 2>/dev/null
echo "--- journal (20) ---"; printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente --no-pager -n 20 2>/dev/null
printf "%s\n" "$PW" | sudo -S rm -f /tmp/.p
echo FIN_ARRANCA
