#!/bin/sh
echo "=== ip -br link ==="
ip -br link
echo "=== ip -o link (con MAC) ==="
ip -o link show
echo "=== id vboxuser ==="
id vboxuser
echo "=== getent group vboxuser ==="
getent group vboxuser
echo "=== getent group eje-ipc (puede no existir aun) ==="
getent group eje-ipc || echo "(eje-ipc no existe: lo crea el instalador)"
echo "=== FIN_PROBE ==="
