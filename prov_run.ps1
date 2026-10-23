$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$art  = "C:\Users\Alexx\Aura\artefactos-golden"
$sh   = "C:\Users\Alexx\Aura\provision.sh"
$vm   = "golden-eje-linux"

function Copia($src, $dst) {
  for ($k = 0; $k -lt 24; $k++) {
    $o = (& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String)
    if ($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready') { return ("copiado " + $dst) }
    Start-Sleep -Seconds 5
  }
  return ("FALLO_COPIA " + $dst + " :: " + $o)
}

Write-Output "== arrancar golden =="
$info = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
if ($info -match 'VMState="running"') { Write-Output "ya corriendo" } else { (& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output }

Write-Output "== readiness real =="
$ok = $false
for ($i = 0; $i -lt 70; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') { $ok = $true; Write-Output ("guestcontrol OK intento " + ($i+1) + " host=" + $r.Trim()); break }
}
if (-not $ok) { Write-Output "GA no respondio; aborto"; (& $vbox controlvm $vm poweroff 2>&1|Out-String)|Write-Output; Write-Output "FIN_PROV"; exit }

Write-Output "== asentar sesion guestcontrol (evitar carrera starting) =="
Start-Sleep -Seconds 20

Write-Output "== copiar al guest (con reintentos) =="
Write-Output (Copia "$art\eje-agente.tar.gz" "/tmp/eje-agente.tar.gz")
Write-Output (Copia "$art\agente.conf.firmado" "/tmp/agente.conf.firmado")
Write-Output (Copia "$art\centinela.dat" "/tmp/centinela.dat")
Write-Output (Copia "$art\clave-cliente.pub" "/tmp/clave-cliente.pub")
Write-Output (Copia "$art\clave-recuperacion.pub" "/tmp/clave-recuperacion.pub")
Write-Output (Copia "$art\inventario.inv" "/tmp/inventario.inv")
Write-Output (Copia "$pwf" "/tmp/.p")
Write-Output (Copia "$sh" "/tmp/provision.sh")

Write-Output "== ejecutar provision.sh (reintentos ante starting) =="
$out = ""
for ($k = 0; $k -lt 24; $k++) {
  $out = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/provision.sh 2>&1 | Out-String)
  if ($out -notmatch 'Error starting guest session') { break }
  Start-Sleep -Seconds 5
}
Write-Output $out

if ($out -match "FIN_PROVISION") {
  Write-Output "== provision COMPLETA -> apagar y snapshot =="
  (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
  Start-Sleep -Seconds 6
  (& $vbox snapshot $vm take base-identidad-anclada --description "golden provisionada: eje-agente dba88063 + 5 artefactos; servicio detenido; marcas sin establecer (pre-primer-arranque)" 2>&1 | Out-String) | Write-Output
  (& $vbox snapshot $vm list 2>&1 | Out-String) | Write-Output
  Write-Output "SNAPSHOT_TOMADO"
} else {
  Write-Output "== provision NO completa -> NO snapshot; apago =="
  (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
}
Write-Output "FIN_PROV"
