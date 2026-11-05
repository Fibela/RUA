#!/bin/bash
P=/tmp/.p
S(){ sudo -S -p '' "$@" < "$P"; }
echo "-- stop eje-agente --"; S systemctl stop eje-agente 2>&1
echo "-- instalar centinela nuevo --"; S install -m 0644 -o root -g root /tmp/centinela-new /var/lib/eje-latam/centinela.dat 2>&1
echo "-- SHA EN DESTINO (reja de RUA) --"; S sha256sum /var/lib/eje-latam/centinela.dat 2>&1
echo "-- sync --"; S sync
echo "-- start eje-agente --"; S systemctl start eje-agente 2>&1
echo "-- GUEST 7b swap OK --"
