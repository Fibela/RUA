#!/bin/sh
echo "ip enp0s3:"; ip -4 addr show enp0s3 2>/dev/null | grep inet
echo "route .1:"; ip route get 192.168.56.1 2>/dev/null
echo "tcp 5514:"; timeout 6 bash -c "exec 3<>/dev/tcp/192.168.56.1/5514 && echo TCP-OK" 2>/dev/null || echo "TCP-FAIL rc=$?"
rm -f /tmp/.p /tmp/tcptest.sh
echo FIN_TCPTEST
