$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$vm   = "golden-eje-linux"
$atk  = "atacante-pa61"
function Copia($src,$dst){ for($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){return ("copiado "+$dst)}; Start-Sleep 5 }; return ("FALLO "+$dst) }

Write-Output "== golden: restaurar base-identidad-anclada (marcas=0) =="
(& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Out-Null
Start-Sleep 4
(& $vbox snapshot $vm restore base-identidad-anclada 2>&1 | Out-String) | Write-Output
Start-Sleep 2
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output

Write-Output "== atacante-pa61: restaurar base-dispositivo y arrancar =="
(& $vbox controlvm $atk poweroff 2>&1 | Out-String) | Out-Null
Start-Sleep 3
(& $vbox snapshot $atk restore base-dispositivo 2>&1 | Out-String) | Write-Output
Start-Sleep 2
(& $vbox startvm $atk --type headless 2>&1 | Out-String) | Write-Output
Start-Sleep 15

Write-Output (Copia "$pwf" "/tmp/.p")
Write-Output (Copia "C:\Users\Alexx\Aura\arranca2.sh" "/tmp/arranca2.sh")
$out=""
for ($k=0;$k -lt 24;$k++){ $out=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/arranca2.sh 2>&1 | Out-String); if($out -notmatch 'Error starting guest session' -and $out -notmatch 'not ready'){break}; Start-Sleep 5 }
Write-Output "===== ARRANCA OUT ====="
Write-Output $out
Write-Output "== esperando 2 ciclos de latido (intervalo 30s) =="
Start-Sleep 45
Write-Output "== MAC atacante =="
(& $vbox guestproperty enumerate $atk 2>&1 | Select-String 'MAC' | Out-String) | Write-Output
Write-Output "FIN_CALMA3_PS"
