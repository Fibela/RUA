$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$vm="golden-eje-linux"
$pwf="C:\Users\Alexx\Aura\secrets\guest.pwd"
$g="C:\Users\Alexx\Aura\d2_guest.sh"
$out="C:\Users\Alexx\Aura\d2.out.txt"
Set-Content $out "== D2 configuracionNoVerifica (journal; sin sink) =="
& $vbox controlvm $vm poweroff *>$null; Start-Sleep 2
& $vbox discardstate $vm *>$null; Start-Sleep 1
"== restore base-agente-pa163 ==" | Add-Content $out
(& $vbox snapshot $vm restore base-agente-pa163 2>&1) | Add-Content $out
(& $vbox startvm $vm --type headless 2>&1) | Add-Content $out
$ok=$false
for($i=0;$i -lt 30;$i++){ Start-Sleep 4; (& $vbox guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/true 2>&1) | Out-Null; if($LASTEXITCODE -eq 0){ $ok=$true; break } }
"== guest listo: $ok ($i) ==" | Add-Content $out
if(-not $ok){ "ABORT guest" | Add-Content $out; exit 1 }
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $pwf "/tmp/.p" 2>&1) | Add-Content $out
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $g "/tmp/d2_guest.sh" 2>&1) | Add-Content $out
"== salida guest ==" | Add-Content $out
(& $vbox guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/bash -- /tmp/d2_guest.sh 2>&1) | Add-Content $out
"== limpieza: restaurar base-agente-pa163 ==" | Add-Content $out
& $vbox controlvm $vm poweroff *>$null; Start-Sleep 2
(& $vbox snapshot $vm restore base-agente-pa163 2>&1) | Add-Content $out
"== FIN D2 ==" | Add-Content $out
