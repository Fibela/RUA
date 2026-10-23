#!/bin/sh
PW="$(cat /tmp/.p)"
echo "== lineas de arranque/verificacion =="
printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente -b --no-pager 2>/dev/null | grep -iE "verifica|frescura|revers|dominio|firma|detalle|secuencia|inventario|centinela|almacen|manipul|fechar" | head -60
echo "== contexto: primeras 25 lineas del arranque =="
printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente -b --no-pager 2>/dev/null | head -25
printf "%s\n" "$PW" | sudo -S rm -f /tmp/.p
rm -f /tmp/djournal.sh
echo FIN_DJOURNAL
