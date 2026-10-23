$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$arr  = "C:\Users\Alexx\Aura\arranca.sh"
$vm   = "golden-eje-linux"
$disp = "atacante-pa61"

function Copia($src, $dst) {
  for ($k = 0; $k -lt 24; $k++) {
    $o = (& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String)
    if ($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready') { return ("copiado " + $dst) }
    Start-Sleep -Seconds 5
  }
  return ("FALLO_COPIA " + $dst)
}

Write-Output "== asegurar VMs apagadas =="
foreach ($m in @($vm, $disp)) {
  $info = (& $vbox showvminfo $m --machinereadable 2>&1 | Out-String)
  if ($info -match 'VMState="running"') { (& $vbox controlvm $m poweroff 2>&1 | Out-String) | Write-Output; Start-Sleep -Seconds 5 }
}

Write-Output "== reproducibilidad: restaurar snapshots base =="
(& $vbox snapshot $vm restore base-identidad-anclada 2>&1 | Out-String) | Write-Output
(& $vbox snapshot $disp restore base-dispositivo 2>&1 | Out-String) | Write-Output

Write-Output "== encender dispositivo (DHCPDISCOVER) y golden =="
(& $vbox startvm $disp --type headless 2>&1 | Out-String) | Write-Output
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output

Write-Output "== readiness golden =="
$ok = $false
for ($i = 0; $i -lt 70; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') { $ok = $true; Write-Output ("golden lista intento " + ($i+1) + " host=" + $r.Trim()); break }
}
if (-not $ok) { Write-Output "golden no respondio; aborto"; Write-Output "FIN_CALMA"; exit }
Start-Sleep -Seconds 20

Write-Output "== copiar arranca.sh + clave =="
Write-Output (Copia "$arr" "/tmp/arranca.sh")
Write-Output (Copia "$pwf" "/tmp/.p")

Write-Output "== PRIMER ARRANQUE OBSERVADO del agente (systemctl start) =="
$out = ""
for ($k = 0; $k -lt 24; $k++) {
  $out = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/arranca.sh 2>&1 | Out-String)
  if ($out -notmatch 'Error starting guest session') { break }
  Start-Sleep -Seconds 5
}
Write-Output $out

Write-Output "== dejar fluir latidos (100s ~ 3 a 30000ms) =="
Start-Sleep -Seconds 100
Write-Output "FIN_CALMA (golden sigue encendida; sink en el host tiene los eventos)"
