$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$vm="golden-eje-linux"
$pwf="C:\Users\Alexx\Aura\secrets\guest.pwd"
$g2="C:\Users\Alexx\Aura\7a2_guest.sh"
$out="C:\Users\Alexx\Aura\7b2.out.txt"
Set-Content $out "== 7b FASE 2: detener agente + savestate + snapshot =="
function Invoke-Vbx { param([Parameter(ValueFromRemainingArguments=$true)]$rest)
  for($k=1;$k -le 10;$k++){ $o=(& $vbox @rest 2>&1 | Out-String); if($LASTEXITCODE -eq 0){ return $o }; Start-Sleep 6 }
  return ("[fallo tras 10]`n"+$o) }
(Invoke-Vbx guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $pwf "/tmp/.p") | Add-Content $out
(Invoke-Vbx guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $g2 "/tmp/7a2_guest.sh") | Add-Content $out
(Invoke-Vbx guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/bash -- /tmp/7a2_guest.sh) | Add-Content $out
Start-Sleep 2
"== savestate ==" | Add-Content $out
(& $vbox controlvm $vm savestate 2>&1 | Out-String) | Add-Content $out
"== snapshot take base-inventario-aceptado ==" | Add-Content $out
(& $vbox snapshot $vm take base-inventario-aceptado --description "7b: centinela con marca; inventario aceptado; calma condiciones=ninguna sello=0 asiento=0" 2>&1 | Out-String) | Add-Content $out
(& $vbox snapshot $vm list 2>&1 | Out-String) | Add-Content $out
"== FIN FASE 2 ==" | Add-Content $out
