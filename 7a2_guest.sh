#!/bin/bash
P=/tmp/.p
sudo -S -p '' systemctl stop eje-agente < "$P" 2>&1
sync
rm -f "$P"
echo "-- agente detenido + sync + credencial borrada --"
