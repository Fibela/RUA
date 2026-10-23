$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$med  = "C:\Users\Alexx\Aura\med.sh"
$src  = "Atacante"
$vm   = "atacante-pa61"

Write-Output "== VMs actuales =="
$vms = (& $vbox list vms 2>&1 | Out-String)
if ($vms -match [regex]::Escape($vm)) {
  Write-Output ("$vm YA existe -- no clono (idempotente). Abortando para no pisar.")
  Write-Output "FIN_CLON"; exit
}

Write-Output "== clonar Atacante -> atacante-pa61 (full) =="
(& $vbox clonevm $src --name $vm --register 2>&1 | Out-String) | Write-Output

Write-Output "== BOOT 1: verificar GA con NAT puesto (heredado) =="
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output
$ga = $false
for ($i = 0; $i -lt 70; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') { $ga = $true; Write-Output ("GA OK intento " + ($i+1) + " host=" + $r.Trim()); break }
}
if (-not $ga) {
  Write-Output "GA_AUSENTE: guestcontrol no respondio con NAT puesto. NO aislo (dejo el clon con NAT para instalar GA)."
  (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
  Write-Output "FIN_CLON"; exit
}
Write-Output "== apagar para reconfigurar red =="
(& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
Start-Sleep -Seconds 8

Write-Output "== AISLAR: nic1 -> intnet eje-pa61-aislada, sin NAT, sin mas NICs =="
(& $vbox modifyvm $vm --nic1 intnet --intnet1 "eje-pa61-aislada" --cableconnected1 on 2>&1 | Out-String) | Write-Output
(& $vbox modifyvm $vm --nic2 none --nic3 none --nic4 none 2>&1 | Out-String) | Write-Output
$mi = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
foreach ($ln in ($mi -split "`n")) { if ($ln -match '^nic[1-4]=|^intnet1=|^macaddress1=|cableconnected1=') { Write-Output $ln.Trim() } }

Write-Output "== BOOT 2: medir MAC con la red definitiva =="
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output
$ok = $false
for ($i = 0; $i -lt 70; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') { $ok = $true; Write-Output ("guestcontrol OK (2) intento " + ($i+1)); break }
}
if ($ok) {
  (& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$med" "/tmp/med.sh" 2>&1 | Out-String) | Write-Output
  (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/med.sh 2>&1 | Out-String) | Write-Output
  (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/rm -- -f /tmp/med.sh 2>&1 | Out-String) | Write-Output
} else { Write-Output "guestcontrol no respondio en boot2 (raro, GA ya verificada)" }
Write-Output "== MAC segun host (showvminfo) =="
$mi2 = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
foreach ($ln in ($mi2 -split "`n")) { if ($ln -match '^macaddress1=|^nic1=|^intnet1=') { Write-Output $ln.Trim() } }

Write-Output "== apagar y snapshot base-dispositivo =="
(& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
Start-Sleep -Seconds 6
(& $vbox snapshot $vm take base-dispositivo --description "clon de Atacante; nic unica intnet eje-pa61-aislada, DHCP sin servidor (DISCOVER periodico), sin NAT" 2>&1 | Out-String) | Write-Output
(& $vbox snapshot $vm list 2>&1 | Out-String) | Write-Output
Write-Output "FIN_CLON"
