echo "== id/host =="; id; hostname
echo "== which =="; which eje-manifiesto eje-agente eje 2>/dev/null
echo "== binarios =="; find / -maxdepth 6 \( -name "eje-manifiesto" -o -name "eje-agente" -o -name "eje_agente" -o -name "eje" \) -type f 2>/dev/null | head -20
echo "== systemd eje =="; systemctl list-unit-files 2>/dev/null | grep -i eje; ls -la /etc/systemd/system 2>/dev/null | grep -i eje
echo "== /var/lib eje =="; ls -la /var/lib 2>/dev/null | grep -i eje; echo "---almacen---"; ls -laR /var/lib/eje-latam 2>/dev/null | head -60
echo "== /run eje =="; ls -la /run/eje-latam 2>/dev/null
echo "== home vboxuser =="; ls -la /home/vboxuser 2>/dev/null | head -40
echo "== artefactos almacen =="; find / -maxdepth 7 \( -name "inventario.inv" -o -name "centinela*.dat" -o -name "evidencia.*" -o -name "*.sem" -o -name "agente.conf.firmado" -o -name "*.rev" \) 2>/dev/null | head -40
echo "== repos fuente (Cargo.toml) =="; find /home /root /opt /srv -maxdepth 4 -name Cargo.toml 2>/dev/null | head -20
echo "== cargo =="; which cargo rustc 2>/dev/null; cargo --version 2>/dev/null
echo "== FIN =="
