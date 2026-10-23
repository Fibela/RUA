#!/bin/sh
set -eu
PW="$(cat /tmp/.p)"
sudo_run() { printf '%s\n' "$PW" | sudo -S "$@"; }

cd /tmp
echo "== 1) verificar tarball (reja, falla cerrado) =="
printf '%s  eje-agente.tar.gz\n' b38fe3cef5298b66832da80859a8d48a3bb1c09a920fae3d641c8ef7a47429ba | sha256sum -c -

echo "== 2) extraer =="
rm -rf /tmp/pkg; mkdir /tmp/pkg; tar -xzf eje-agente.tar.gz -C /tmp/pkg
cd /tmp/pkg/eje-agente

echo "== 3) verificar binario interno (el gate, en el guest) =="
printf '%s  eje-agente\n' dba88063f712e581c6150447c99546377f9424d292335e2dc1b4fd6a5266eb66 | sha256sum -c -

echo "== 4) instalar (sudo); su propia reja de MANIFIESTO corre aqui =="
sudo_run sh instalar.sh

echo "== 5) HUMO: eje-agente sin args, SIN config aun (--interfaz obligatoria -> uso + salida 1) =="
set +e
/usr/local/bin/eje-agente >/tmp/humo.txt 2>&1
rc=$?
set -e
echo "HUMO_RC=$rc"
head -3 /tmp/humo.txt
if [ "$rc" -ne 1 ]; then
  echo "!! HUMO_INESPERADO: esperaba salida 1 sin config; aborto antes de colocar nada ni arrancar."
  exit 1
fi

echo "== 6) colocar los 5 artefactos (sudo) =="
sudo_run install -m 0644 /tmp/agente.conf.firmado    /etc/eje-latam/agente.conf.firmado
sudo_run install -m 0644 /tmp/centinela.dat          /var/lib/eje-latam/centinela.dat
sudo_run install -m 0644 /tmp/clave-cliente.pub      /var/lib/eje-latam/clave-cliente.pub
sudo_run install -m 0644 /tmp/clave-recuperacion.pub /var/lib/eje-latam/clave-recuperacion.pub
sudo_run install -m 0644 /tmp/inventario.inv         /var/lib/eje-latam/inventario.inv

echo "== 7) verificar hashes EN DESTINO (reja, falla cerrado) =="
cat > /tmp/destino.sha <<EOF
1f95c1b5714dc83294995ce35fc42a2cd13bb1fc929c62db3ea7235bc04808a6  /etc/eje-latam/agente.conf.firmado
11b0d10a72d4bb4a3fa6ca05c875b741b61406efcddb34945c8b11781526e6ba  /var/lib/eje-latam/centinela.dat
7707ca2799be068b5d1c01e773dcff51c38e88d771b077858cd4bc9085a6ddb5  /var/lib/eje-latam/clave-cliente.pub
768a3b59d453594b9f159e3b8aa753758f729a8b12b2d889d8c6ee48bbb8ac3b  /var/lib/eje-latam/clave-recuperacion.pub
23fffe6e8ace44fcf45f19309499fc5e6624bd0584dde3eafd59cdb8e584df84  /var/lib/eje-latam/inventario.inv
EOF
sha256sum -c /tmp/destino.sha

echo "== 8) servicio: debe NO estar enabled ni active =="
printf '%s\n' "$PW" | sudo -S systemctl is-enabled eje-agente 2>/dev/null || echo "is-enabled: no (esperado)"
printf '%s\n' "$PW" | sudo -S systemctl is-active  eje-agente 2>/dev/null || echo "is-active: no (esperado)"

echo "== 9) limpiar secretos y temporales =="
rm -f /tmp/.p /tmp/eje-agente.tar.gz /tmp/agente.conf.firmado /tmp/centinela.dat \
      /tmp/clave-cliente.pub /tmp/clave-recuperacion.pub /tmp/inventario.inv \
      /tmp/destino.sha /tmp/humo.txt /tmp/prov.sh
rm -rf /tmp/pkg
echo "FIN_PROV"
