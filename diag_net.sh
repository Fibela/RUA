#!/bin/sh
PW="$(cat /tmp/.p 2>/dev/null)"
echo "=== ip -br addr ==="; ip -br addr
echo "=== ruta a 192.168.56.1 ==="; ip route get 192.168.56.1 2>&1
echo "=== test TCP a 192.168.56.1:5514 ==="; timeout 5 bash -c "echo > /dev/tcp/192.168.56.1/5514" 2>&1 && echo "CONECTA" || echo "NO-CONECTA rc=$?"
echo "=== journal eje-agente (30) ==="; printf "%s\n" "$PW" | sudo -S journalctl -u eje-agente --no-pager -n 30 2>/dev/null
rm -f /tmp/.p
echo FIN_DIAGNET
