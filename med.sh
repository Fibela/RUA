#!/bin/sh
echo "=== hostname ==="; cat /etc/hostname
echo "=== ip -o link ==="; ip -o link show
echo "=== ip -4 addr (sin servidor DHCP: sin IPv4 o link-local) ==="; ip -4 addr show
echo "=== FIN_MED ==="
