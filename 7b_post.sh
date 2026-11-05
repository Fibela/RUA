#!/bin/bash
P=/tmp/.p
sudo -S -p '' journalctl -u eje-agente -n 70 --no-pager < "$P" 2>&1
rm -f "$P" /tmp/centinela-new
echo "-- POST 7b OK --"
