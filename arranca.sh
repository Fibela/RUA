#!/bin/sh
PW="$(cat /tmp/.p)"
printf "%s\n" "$PW" | sudo -S systemctl start eje-agente
sleep 3
printf "is-active="; printf "%s\n" "$PW" | sudo -S systemctl is-active eje-agente 2>/dev/null
printf "is-enabled="; printf "%s\n" "$PW" | sudo -S systemctl is-enabled eje-agente 2>/dev/null
echo "--- journal eje-agente (ultimas 25) ---"
printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente --no-pager -n 25 2>/dev/null
rm -f /tmp/.p
echo FIN_ARRANCA
