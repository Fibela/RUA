#!/bin/sh
set -eu
PW="$(cat /tmp/.p)"
sudo_s() { printf "%s\n" "$PW" | sudo -S "$@"; }
chk() { got=$(sha256sum "$1" | cut -d" " -f1); n=$(wc -c <"$1"); if [ "$got" = "$2" ]; then echo "OK   $1 ($n B)"; else echo "MISMATCH $1 : $got != $2 ($n B)"; exit 2; fi; }
sudo_s systemctl stop eje-agente 2>/dev/null || true
sudo_s systemctl reset-failed eje-agente 2>/dev/null || true
sudo_s mkdir -p /etc/eje-latam /var/lib/eje-latam
sudo_s install -m 0644 /tmp/agente.conf.firmado    /etc/eje-latam/agente.conf.firmado
sudo_s install -m 0644 /tmp/centinela.dat          /var/lib/eje-latam/centinela.dat
sudo_s install -m 0644 /tmp/clave-cliente.pub      /var/lib/eje-latam/clave-cliente.pub
sudo_s install -m 0644 /tmp/clave-recuperacion.pub /var/lib/eje-latam/clave-recuperacion.pub
sudo_s install -m 0644 /tmp/inventario.inv         /var/lib/eje-latam/inventario.inv
echo "== verificar EN DESTINO =="
chk /etc/eje-latam/agente.conf.firmado          1f95c1b5714dc83294995ce35fc42a2cd13bb1fc929c62db3ea7235bc04808a6
chk /var/lib/eje-latam/centinela.dat            11b0d10a72d4bb4a3fa6ca05c875b741b61406efcddb34945c8b11781526e6ba
chk /var/lib/eje-latam/clave-cliente.pub        7707ca2799be068b5d1c01e773dcff51c38e88d771b077858cd4bc9085a6ddb5
chk /var/lib/eje-latam/clave-recuperacion.pub   768a3b59d453594b9f159e3b8aa753758f729a8b12b2d889d8c6ee48bbb8ac3b
chk /var/lib/eje-latam/inventario.inv           23fffe6e8ace44fcf45f19309499fc5e6624bd0584dde3eafd59cdb8e584df84
chk /usr/local/bin/eje-agente                   dba88063f712e581c6150447c99546377f9424d292335e2dc1b4fd6a5266eb66
echo "== FLUSH a disco (sync) =="
sync; sudo_s sync; sudo_s sh -c "echo 3 > /proc/sys/vm/drop_caches" 2>/dev/null || true
set +e
printf "is-active="; sudo_s systemctl is-active eje-agente 2>/dev/null
echo "== listado destino =="; ls -la /etc/eje-latam /var/lib/eje-latam
sudo_s rm -f /tmp/.p /tmp/reparar.sh /tmp/agente.conf.firmado /tmp/centinela.dat /tmp/clave-cliente.pub /tmp/clave-recuperacion.pub /tmp/inventario.inv
echo FIN_REPARAR
