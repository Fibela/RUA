$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$src  = "C:\Users\Alexx\Aura\probe.sh"
$vm   = "golden-eje-linux"

Write-Output "== MACs del host (showvminfo) =="
$mi = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
foreach ($ln in ($mi -split "`n")) { if ($ln -match '^macaddress[12]=|^nic[12]=|^intnet2=|hostonlyadapter1=') { Write-Output $ln.Trim() } }

Write-Output "== arrancar golden =="
$info = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
if ($info -match 'VMState="running"') { Write-Output "ya corriendo" }
else { (& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output }

Write-Output "== readiness real (guestcontrol) =="
$ok = $false
for ($i = 0; $i -lt 70; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') { $ok = $true; Write-Output ("guestcontrol OK intento " + ($i+1) + " host=" + $r.Trim()); break }
}
if (-not $ok) { Write-Output "guestcontrol no respondio; aborto"; (& $vbox controlvm $vm poweroff 2>&1|Out-String)|Write-Output; Write-Output "FIN_PROBE"; exit }

Write-Output "== copiar y correr probe =="
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "/tmp/probe.sh" 2>&1 | Out-String) | Write-Output
(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/probe.sh 2>&1 | Out-String) | Write-Output

Write-Output "== limpiar y apagar =="
(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/rm -- -f /tmp/probe.sh 2>&1 | Out-String) | Write-Output
(& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
Write-Output "FIN_PROBE"
