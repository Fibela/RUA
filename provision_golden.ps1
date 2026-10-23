$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$art  = "C:\Users\Alexx\Aura\artefactos-golden"
$prov = "C:\Users\Alexx\Aura\prov.sh"
$vm   = "golden-eje-linux"
function gc { param($a) (& $vbox guestcontrol $vm @a 2>&1 | Out-String) }

Write-Output "== arrancar golden =="
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output
$ok = $false
for ($i=0; $i -lt 70; $i++) {
  Start-Sleep -Seconds 5
  $r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
  if ($r -notmatch "not ready" -and $r -notmatch "error:" -and $r.Trim() -match '^[A-Za-z0-9._-]+$') { $ok=$true; Write-Output ("readiness OK " + ($i+1) + " host=" + $r.Trim()); break }
}
if (-not $ok) { Write-Output "readiness fallo; aborto"; (& $vbox controlvm $vm poweroff 2>&1|Out-String)|Write-Output; Write-Output "FIN_PROV_WRAP"; exit }

Write-Output "== copiar paquete + 5 artefactos + prov.sh + clave a /tmp =="
foreach ($f in @("eje-agente.tar.gz","agente.conf.firmado","centinela.dat","clave-cliente.pub","clave-recuperacion.pub","inventario.inv")) {
  (& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$art\$f" "/tmp/$f" 2>&1 | Out-String) | Write-Output
}
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$prov" "/tmp/prov.sh" 2>&1 | Out-String) | Write-Output
(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$pwf" "/tmp/.p" 2>&1 | Out-String) | Write-Output

Write-Output "== ejecutar prov.sh en el guest =="
$out = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/prov.sh 2>&1 | Out-String)
Write-Output $out

if ($out -match "FIN_PROV") {
  Write-Output "== provision OK -> apagar y snapshot base-identidad-anclada =="
  (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
  Start-Sleep -Seconds 6
  (& $vbox snapshot $vm take base-identidad-anclada --description "golden provisionada: binario dba88063, 5 artefactos, servicio detenido, marcas SIN establecer (pre-primer-arranque)" 2>&1 | Out-String) | Write-Output
  (& $vbox snapshot $vm list 2>&1 | Out-String) | Write-Output
} else {
  Write-Output "== provision NO completo (sin FIN_PROV): NO hago snapshot. Apago para dejar limpio. =="
  (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
}
Write-Output "FIN_PROV_WRAP"
