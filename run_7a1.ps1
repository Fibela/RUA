$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$vm="golden-eje-linux"
$pwf="C:\Users\Alexx\Aura\secrets\guest.pwd"
$bin="C:\Users\Alexx\Aura\artefactos-golden\eje-agente"
$gsh="C:\Users\Alexx\Aura\7a_guest.sh"
$out="C:\Users\Alexx\Aura\7a.out.txt"
Set-Content $out "== 7a FASE 1 (sin snapshot) =="
& $vbox controlvm $vm poweroff *>$null; Start-Sleep 2
& $vbox discardstate $vm *>$null; Start-Sleep 1
"== restore d520dd87 ==" | Add-Content $out
(& $vbox snapshot $vm restore base-identidad-anclada 2>&1) | Add-Content $out
(& $vbox startvm $vm --type headless 2>&1) | Add-Content $out
$ok=$false
for($i=0;$i -lt 30;$i++){ Start-Sleep 4; (& $vbox guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/true 2>&1) | Out-Null; if($LASTEXITCODE -eq 0){ $ok=$true; break } }
"== guest listo: $ok (intentos $i) ==" | Add-Content $out
if(-not $ok){ "ABORT: guest no respondio" | Add-Content $out; exit 1 }
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $pwf "/tmp/.p" 2>&1) | Add-Content $out
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $bin "/tmp/eje-agente-new" 2>&1) | Add-Content $out
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $gsh "/tmp/7a_guest.sh" 2>&1) | Add-Content $out
"== salida del guest ==" | Add-Content $out
(& $vbox guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/bash -- /tmp/7a_guest.sh 2>&1) | Add-Content $out
(& $vbox guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/rm -- -f /tmp/.p 2>&1) | Add-Content $out
"== FIN FASE 1 (esperando verificacion de RUA antes del savestate) ==" | Add-Content $out
