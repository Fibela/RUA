$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$arr  = "C:\Users\Alexx\Aura\arranca.sh"
$vm   = "golden-eje-linux"
$disp = "atacante-pa61"
function Copia($src,$dst){ for($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){return ("copiado "+$dst)}; Start-Sleep 5 }; return ("FALLO "+$dst) }

Write-Output "== apagar ambas =="
foreach ($m in @($vm,$disp)) { $info=(& $vbox showvminfo $m --machinereadable 2>&1 | Out-String); if ($info -match 'VMState="running"'){ (& $vbox controlvm $m poweroff 2>&1|Out-String)|Write-Output; Start-Sleep 6 } }

Write-Output "== rehacer base-identidad-anclada con el unit corregido =="
(& $vbox snapshot $vm delete base-identidad-anclada 2>&1 | Out-String) | Write-Output
Start-Sleep 3
(& $vbox snapshot $vm take base-identidad-anclada --description "golden provisionada + unit systemd corregido (2815B f6f965c5); servicio detenido; marcas sin establecer" 2>&1 | Out-String) | Write-Output

Write-Output "== restaurar dispositivo y encender ambas =="
(& $vbox snapshot $disp restore base-dispositivo 2>&1 | Out-String) | Write-Output
(& $vbox startvm $disp --type headless 2>&1 | Out-String) | Write-Output
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output

Write-Output "== readiness golden =="
$ok=$false
for ($i=0;$i -lt 70;$i++){ Start-Sleep 5; $r=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String); if($r -notmatch 'not ready' -and $r -notmatch 'error:' -and $r.Trim() -match '^[A-Za-z0-9._-]+$'){ $ok=$true; Write-Output ("golden lista intento "+($i+1)); break } }
if (-not $ok){ Write-Output "golden no respondio; aborto"; Write-Output "FIN_CALMA2"; exit }
Start-Sleep 20

Write-Output "== copiar arranca.sh + clave =="
Write-Output (Copia "$arr" "/tmp/arranca.sh")
Write-Output (Copia "$pwf" "/tmp/.p")

Write-Output "== PRIMER ARRANQUE OBSERVADO (systemctl start) =="
$out=""
for ($k=0;$k -lt 24;$k++){ $out=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/arranca.sh 2>&1 | Out-String); if($out -notmatch 'Error starting guest session'){break}; Start-Sleep 5 }
Write-Output $out

Write-Output "== dejar fluir latidos (100s) =="
Start-Sleep 100
Write-Output "FIN_CALMA2"
