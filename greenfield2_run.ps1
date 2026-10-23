$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf="C:\Users\Alexx\Aura\secrets\guest.pwd"
$vm="golden-eje-linux"
function Copia($src,$dst){ for($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){return ("copiado "+$dst)}; Start-Sleep 5 }; return ("FALLO "+$dst) }
Write-Output "== restaurar d520dd87 =="
& $vbox controlvm $vm poweroff 2>&1 | Out-Null
Start-Sleep 4
(& $vbox snapshot $vm restore base-identidad-anclada 2>&1 | Out-String) | Write-Output
Start-Sleep 2
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output
Start-Sleep 15
Write-Output (Copia "$pwf" "/tmp/.p")
Write-Output (Copia "C:\Users\Alexx\Aura\greenfield2.sh" "/tmp/greenfield2.sh")
$out=""
for ($k=0;$k -lt 24;$k++){ $out=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/greenfield2.sh 2>&1 | Out-String); if($out -notmatch 'Error starting guest session' -and $out -notmatch 'not ready'){break}; Start-Sleep 5 }
Write-Output "===== GF2 OUT ====="
Write-Output $out
Write-Output "== dejar latir 75s (sink captura) =="
Start-Sleep 75
Write-Output "== limpieza: restaurar d520dd87 =="
& $vbox controlvm $vm poweroff 2>&1 | Out-Null
Start-Sleep 4
(& $vbox snapshot $vm restore base-identidad-anclada 2>&1 | Out-String) | Write-Output
Write-Output "FIN_GF2_PS"
