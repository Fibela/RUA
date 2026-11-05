$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$vm="golden-eje-linux"
$pwf="C:\Users\Alexx\Aura\secrets\guest.pwd"
$g2="C:\Users\Alexx\Aura\7a2_guest.sh"
$out="C:\Users\Alexx\Aura\7a2.out.txt"
Set-Content $out "== 7a FASE 2: detener agente + savestate + snapshot =="
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $pwf "/tmp/.p" 2>&1) | Add-Content $out
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $g2 "/tmp/7a2_guest.sh" 2>&1) | Add-Content $out
(& $vbox guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/bash -- /tmp/7a2_guest.sh 2>&1) | Add-Content $out
Start-Sleep 2
"== savestate ==" | Add-Content $out
(& $vbox controlvm $vm savestate 2>&1) | Add-Content $out
"== snapshot take base-agente-pa163 ==" | Add-Content $out
(& $vbox snapshot $vm take base-agente-pa163 --description "7a binario b282c4bb PA-163; inventario sin anclar; detalle FrescuraNoEstablecida medido" 2>&1) | Add-Content $out
"== snapshot list ==" | Add-Content $out
(& $vbox snapshot $vm list 2>&1) | Add-Content $out
"== FIN FASE 2 ==" | Add-Content $out
