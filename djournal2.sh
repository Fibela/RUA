#!/bin/sh
PW="$(cat /tmp/.p)"
echo "== bloque de arranque limpio (desde 03:23:30) =="
printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente --since "2026-10-20 03:23:30" --no-pager 2>/dev/null | head -70
printf "%s\n" "$PW" | sudo -S rm -f /tmp/.p
rm -f /tmp/djournal2.sh
echo FIN_DJOURNAL2
