#!/bin/sh
PW="$(cat /tmp/.p 2>/dev/null)"
printf "%s\n" "$PW" | sudo -S hostnamectl set-hostname eje-golden-01
rm -f /tmp/.p /tmp/sethost.sh
printf "%s\n" "$PW" | sudo -S systemctl reboot
