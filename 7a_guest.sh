#!/bin/bash
P=/tmp/.p
S(){ sudo -S -p '' "$@" < "$P"; }
echo "-- stop eje-agente --"; S systemctl stop eje-agente 2>&1
echo "-- instalar binario nuevo --"; S install -m 0755 -o root -g root /tmp/eje-agente-new /usr/local/bin/eje-agente 2>&1
echo "-- SHA EN DESTINO (reja de RUA) --"; sha256sum /usr/local/bin/eje-agente
echo "-- tipo/tamano --"; ls -l /usr/local/bin/eje-agente; file /usr/local/bin/eje-agente 2>/dev/null
echo "-- sync --"; S sync
echo "-- start eje-agente --"; S systemctl start eje-agente 2>&1
sleep 8
echo "-- BANNER (journal, ultimas 70) --"; S journalctl -u eje-agente -n 70 --no-pager 2>&1
echo "-- FIN GUEST --"
