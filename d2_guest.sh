#!/bin/bash
P=/tmp/.p
cat > /tmp/corrupt.py <<'PY'
f="/etc/eje-latam/agente.conf.firmado"
d=bytearray(open(f,'rb').read())
o=100 if len(d)>100 else len(d)//2
d[o]^=0xFF
open(f,'wb').write(d)
print("byte",o,"volteado; tam",len(d))
PY
echo "-- corromper 1 byte de la config firmada (root) --"
if sudo -S -p '' test -x /usr/bin/python3 < "$P" 2>/dev/null; then
  sudo -S -p '' python3 /tmp/corrupt.py < "$P" 2>&1
else
  sudo -S -p '' bash -c 'F=/etc/eje-latam/agente.conf.firmado; b=$(dd if=$F bs=1 skip=100 count=1 2>/dev/null|od -An -tu1|tr -d " "); n=$((b^255)); printf "$(printf "\\x%02x" $n)"|dd of=$F bs=1 seek=100 count=1 conv=notrunc 2>/dev/null; echo "byte100 $b->$n"' < "$P" 2>&1
fi
echo "-- restart eje-agente --"
sudo -S -p '' systemctl restart eje-agente < "$P" 2>&1
sleep 12
echo "-- BANNER + condiciones (journal, 60) --"
sudo -S -p '' journalctl -u eje-agente -n 60 --no-pager < "$P" 2>&1
rm -f "$P" /tmp/corrupt.py
echo "-- FIN GUEST D2 --"
