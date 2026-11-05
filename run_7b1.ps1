$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$vm="golden-eje-linux"
$pwf="C:\Users\Alexx\Aura\secrets\guest.pwd"
$cent="C:\Users\Alexx\Aura\artefactos-golden\centinela.dat"
$g="C:\Users\Alexx\Aura\7b_guest.sh"
$post="C:\Users\Alexx\Aura\7b_post.sh"
$sink="C:\Users\Alexx\Aura\target\release\rua-sink.exe"
$sinkout="C:\Users\Alexx\Aura\sink-7b.jsonl"
$out="C:\Users\Alexx\Aura\7b.out.txt"
Set-Content $out "== 7b FASE 1 (swap centinela + calma; sin snapshot) =="

function Invoke-Vbx { param([Parameter(ValueFromRemainingArguments=$true)]$rest)
  for($k=1;$k -le 10;$k++){
    $o = (& $vbox @rest 2>&1 | Out-String)
    if($LASTEXITCODE -eq 0){ return $o }
    Start-Sleep 6
  }
  return ("[fallo tras 10 intentos]`n" + $o)
}

& $vbox controlvm $vm poweroff *>$null; Start-Sleep 3
& $vbox discardstate $vm *>$null; Start-Sleep 2
"== restore base-agente-pa163 ==" | Add-Content $out
(& $vbox snapshot $vm restore base-agente-pa163 2>&1 | Out-String) | Add-Content $out
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Add-Content $out

Start-Sleep 25
$ready=$false
for($i=1;$i -le 20;$i++){
  (& $vbox guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/true 2>&1) | Out-Null
  if($LASTEXITCODE -eq 0){ $ready=$true; break }
  Start-Sleep 5
}
"== guest listo: $ready ($i) ==" | Add-Content $out

(Invoke-Vbx guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $pwf "/tmp/.p") | Add-Content $out
(Invoke-Vbx guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $cent "/tmp/centinela-new") | Add-Content $out
(Invoke-Vbx guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $g "/tmp/7b_guest.sh") | Add-Content $out
(Invoke-Vbx guestcontrol $vm copyto --username vboxuser --passwordfile $pwf $post "/tmp/7b_post.sh") | Add-Content $out

Remove-Item $sinkout -ErrorAction SilentlyContinue
$sp = Start-Process -FilePath $sink -ArgumentList '--listen','192.168.56.1:5514','--out',$sinkout,'--print' -PassThru -WindowStyle Hidden
Start-Sleep 2

"== swap centinela (guest) ==" | Add-Content $out
(Invoke-Vbx guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/bash -- /tmp/7b_guest.sh) | Add-Content $out

"== esperando latidos ~95s (intervalo 30000 -> 2-3) ==" | Add-Content $out
Start-Sleep 95

"== banner + condiciones (journal) ==" | Add-Content $out
(Invoke-Vbx guestcontrol $vm run --username vboxuser --passwordfile $pwf --exe /bin/bash -- /tmp/7b_post.sh) | Add-Content $out

Start-Sleep 3
Stop-Process -Id $sp.Id -Force -ErrorAction SilentlyContinue
Start-Sleep 1
"== sink-7b.jsonl (latidos capturados) ==" | Add-Content $out
if(Test-Path $sinkout){ $sc=(Get-Content $sinkout); if($sc){ $sc | Add-Content $out } else { "(sink sin lineas)" | Add-Content $out } } else { "(sink no existe)" | Add-Content $out }
"== FIN FASE 1 ==" | Add-Content $out
