#!/bin/sh
echo "=== disco ==="; df -h / /usr /var /tmp 2>/dev/null
echo "=== binario instalado ==="; ls -la /usr/local/bin/eje-agente
echo "=== file ==="; file /usr/local/bin/eje-agente 2>/dev/null || true
echo "=== magic (7f454c46 = ELF) ==="; head -c 4 /usr/local/bin/eje-agente | xxd 2>/dev/null || head -c 4 /usr/local/bin/eje-agente | od -An -tx1
echo "=== sha256 (esperado dba88063) ==="; sha256sum /usr/local/bin/eje-agente
echo "=== tamano esperado 920064 ==="; wc -c /usr/local/bin/eje-agente
echo FIN_DIAGDISK
