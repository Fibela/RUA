#!/bin/sh
set -eu
PW="$(cat /tmp/.p)"
sudo_s() { printf '%s\n' "$PW" | sudo -S "$@"; }
chk() { got=$(sha256sum "$1" | cut -d' ' -f1); if [ "$got" = "$2" ]; then echo "OK   $1"; else echo "MISMATCH $1 : $got != $2"; exit 2; fi; }

echo "== 1) integridad tarball =="
chk /tmp/eje-agente.tar.gz b38fe3cef5298b66832da80859a8d48a3bb1c09a920fae3d641c8ef7a47429ba

echo "== 2) extraer =="
rm -rf /tmp/pkg; mkdir -p /tmp/pkg; tar -xzf /tmp/eje-agente.tar.gz -C /tmp/pkg

echo "== 3) procedencia binario (gate dba88063) =="
chk /tmp/pkg/eje-agente/eje-agente dba88063f712e581c6150447c99546377f9424d292335e2dc1b4fd6a5266eb66

echo "== 4) instalar (root; no arranca servicio) =="
( cd /tmp/pkg/eje-agente && sudo_s sh instalar.sh )

echo "== 5) HUMO: config AUSENTE -> --interfaz obligatoria -> debe negarse =="
if [ -f /etc/eje-latam/agente.conf.firmado ]; then echo "ABORTO: ya hay config antes del humo"; exit 3; fi
set +e
/usr/local/bin/eje-agente >/tmp/humo.out 2>&1
HE=$?
set -e
echo "humo exit=$HE"; echo "---- salida ----"; head -6 /tmp/humo.out
if [ "$HE" -eq 0 ]; then echo "ABORTO: el binario NO se nego sin config (exit 0)"; exit 4; fi
echo "humo OK: se nego como se esperaba"

echo "== 6) verificar los 5 en /tmp =="
chk /tmp/agente.conf.firmado    1f95c1b5714dc83294995ce35fc42a2cd13bb1fc929c62db3ea7235bc04808a6
chk /tmp/centinela.dat          11b0d10a72d4bb4a3fa6ca05c875b741b61406efcddb34945c8b11781526e6ba
chk /tmp/clave-cliente.pub      7707ca2799be068b5d1c01e773dcff51c38e88d771b077858cd4bc9085a6ddb5
chk /tmp/clave-recuperacion.pub 768a3b59d453594b9f159e3b8aa753758f729a8b12b2d889d8c6ee48bbb8ac3b
chk /tmp/inventario.inv         23fffe6e8ace44fcf45f19309499fc5e6624bd0584dde3eafd59cdb8e584df84

echo "== 7) colocar (root) =="
sudo_s install -m 0644 /tmp/agente.conf.firmado    /etc/eje-latam/agente.conf.firmado
sudo_s install -m 0644 /tmp/centinela.dat          /var/lib/eje-latam/centinela.dat
sudo_s install -m 0644 /tmp/clave-cliente.pub      /var/lib/eje-latam/clave-cliente.pub
sudo_s install -m 0644 /tmp/clave-recuperacion.pub /var/lib/eje-latam/clave-recuperacion.pub
sudo_s install -m 0644 /tmp/inventario.inv         /var/lib/eje-latam/inventario.inv

echo "== 8) verificar EN DESTINO =="
chk /etc/eje-latam/agente.conf.firmado          1f95c1b5714dc83294995ce35fc42a2cd13bb1fc929c62db3ea7235bc04808a6
chk /var/lib/eje-latam/centinela.dat            11b0d10a72d4bb4a3fa6ca05c875b741b61406efcddb34945c8b11781526e6ba
chk /var/lib/eje-latam/clave-cliente.pub        7707ca2799be068b5d1c01e773dcff51c38e88d771b077858cd4bc9085a6ddb5
chk /var/lib/eje-latam/clave-recuperacion.pub   768a3b59d453594b9f159e3b8aa753758f729a8b12b2d889d8c6ee48bbb8ac3b
chk /var/lib/eje-latam/inventario.inv           23fffe6e8ace44fcf45f19309499fc5e6624bd0584dde3eafd59cdb8e584df84

echo "== 9) servicio NO activo/enabled =="
printf 'is-active='; systemctl is-active eje-agente || true
printf 'is-enabled='; systemctl is-enabled eje-agente || true

echo "== 10) limpiar secreto y temporales =="
rm -f /tmp/.p /tmp/eje-agente.tar.gz /tmp/provision.sh /tmp/humo.out
rm -f /tmp/agente.conf.firmado /tmp/centinela.dat /tmp/clave-cliente.pub /tmp/clave-recuperacion.pub /tmp/inventario.inv
rm -rf /tmp/pkg

echo "== listado final =="
ls -la /etc/eje-latam /var/lib/eje-latam
echo "FIN_PROVISION"
