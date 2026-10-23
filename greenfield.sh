#!/bin/sh
set +e
PW="$(cat /tmp/.p)"
echo "== almacen ANTES =="; ls -la /var/lib/eje-latam
echo "== quitar SOLO centinela.dat (sin anclar) =="
printf "%s\n" "$PW" | sudo -S rm -f /var/lib/eje-latam/centinela.dat
printf "%s\n" "$PW" | sudo -S sync
echo "== almacen DESPUES =="; ls -la /var/lib/eje-latam
echo "== START observado =="
printf "%s\n" "$PW" | sudo -S systemctl reset-failed eje-agente 2>/dev/null
printf "%s\n" "$PW" | sudo -S systemctl start eje-agente 2>/dev/null
sleep 4
printf "is-active="; printf "%s\n" "$PW" | sudo -S systemctl is-active eje-agente 2>/dev/null
echo "== journal arranque (30) =="
printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente -b --no-pager -n 30 2>/dev/null
printf "%s\n" "$PW" | sudo -S rm -f /tmp/.p /tmp/greenfield.sh
echo FIN_GREENFIELD
