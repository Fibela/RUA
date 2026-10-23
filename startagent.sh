#!/bin/sh
PW="$(cat /tmp/.p 2>/dev/null)"
printf "%s\n" "$PW" | sudo -S systemctl start eje-agente
sleep 2
echo "is-active=$(systemctl is-active eje-agente)"
echo "is-enabled=$(systemctl is-enabled eje-agente 2>/dev/null)"
echo "--- journalctl eje-agente (ultimas 15) ---"
printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente --no-pager -n 15 2>/dev/null || true
rm -f /tmp/.p /tmp/startagent.sh
echo FIN_STARTAGENT
