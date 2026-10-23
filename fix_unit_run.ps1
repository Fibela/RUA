$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$art  = "C:\Users\Alexx\Aura\artefactos-golden"
$vm   = "golden-eje-linux"
function Copia($src,$dst){ for($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){return ("copiado "+$dst)}; Start-Sleep 5 }; return ("FALLO "+$dst) }
# asegurar golden encendida
$info = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
if ($info -notmatch 'VMState="running"') {
  (& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output
  for ($i=0;$i -lt 70;$i++){ Start-Sleep 5; $r=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String); if($r -match 'eje-golden-01'){break} }
  Start-Sleep 20
}
Write-Output (Copia "$art\eje-agente.tar.gz" "/tmp/eje-agente.tar.gz")
Write-Output (Copia "$pwf" "/tmp/.p")
Write-Output (Copia "C:\Users\Alexx\Aura\fix_unit.sh" "/tmp/fix_unit.sh")
$out=""
for ($k=0;$k -lt 24;$k++){ $out=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/fix_unit.sh 2>&1 | Out-String); if($out -notmatch 'Error starting guest session'){break}; Start-Sleep 5 }
Write-Output $out
Write-Output "FIN_FIXUNIT_PS"
