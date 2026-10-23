$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$art  = "C:\Users\Alexx\Aura\artefactos-golden"
$vm   = "golden-eje-linux"
function Copia($src,$dst){ for($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){return ("copiado "+$dst)}; Start-Sleep 5 }; return ("FALLO "+$dst) }
Write-Output (Copia "$art\eje-agente.tar.gz" "/tmp/eje-agente.tar.gz")
Write-Output (Copia "$pwf" "/tmp/.p")
Write-Output (Copia "C:\Users\Alexx\Aura\fix_bin.sh" "/tmp/fix_bin.sh")
$out=""
for ($k=0;$k -lt 24;$k++){ $out=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/fix_bin.sh 2>&1 | Out-String); if($out -notmatch 'Error starting guest session'){break}; Start-Sleep 5 }
Write-Output $out
if ($out -match "FIN_FIXBIN") {
  Write-Output "== rehacer baseline corregido =="
  (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
  Start-Sleep 6
  (& $vbox snapshot $vm delete base-identidad-anclada 2>&1 | Out-String) | Write-Output
  Start-Sleep 3
  (& $vbox snapshot $vm take base-identidad-anclada --description "golden provisionada: binario dba88063 + unit 2815B + 5 artefactos; servicio detenido; marcas sin establecer" 2>&1 | Out-String) | Write-Output
  (& $vbox snapshot $vm list 2>&1 | Out-String) | Write-Output
} else { Write-Output "fix_bin no completo; NO snapshot" }
Write-Output "FIN_FIXBIN_PS"
