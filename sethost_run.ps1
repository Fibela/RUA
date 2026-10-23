$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$src  = "C:\Users\Alexx\Aura\sethost.sh"
$vm   = "golden-eje-linux"

Write-Output "== estado inicial =="
$info = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
if ($info -match 'VMState="running"') { Write-Output "ya corriendo" }
else { (& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output }

Write-Output "== readiness real (guestcontrol) arranque 1 =="
$before = ""
for ($i = 0; $i -lt 60; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') {
    $before = $r.Trim(); Write-Output ("guestcontrol OK intento " + ($i+1)); break
  }
}
if ($before -eq "") {
  Write-Output "guestcontrol nunca respondio (arranque 1); aborto"
  (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
  Write-Output "FIN_SETHOST"; exit
}
Write-Output ("HOSTNAME_ANTES=" + $before)

Write-Output "== copiar script y clave =="
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "/tmp/sethost.sh" 2>&1 | Out-String) | Write-Output
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$pwf" "/tmp/.p" 2>&1 | Out-String) | Write-Output

Write-Output "== fijar hostname + reiniciar =="
(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/sethost.sh 2>&1 | Out-String) | Write-Output

Write-Output "== esperar reinicio real + readiness arranque 2 =="
Start-Sleep -Seconds 35
$after = ""
for ($i = 0; $i -lt 60; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') {
    $after = $r.Trim(); Write-Output ("guestcontrol OK (2) intento " + ($i+1)); break
  }
}
$hn = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/hostname 2>&1 | Out-String)
Write-Output ("HOSTNAME_ANTES=" + $before)
Write-Output ("ETC_HOSTNAME_DESPUES=" + $after)
Write-Output ("HOSTNAME_CMD_DESPUES=" + $hn.Trim())

Write-Output "== apagar golden =="
(& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
Write-Output "FIN_SETHOST"
