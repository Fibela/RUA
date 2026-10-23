$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$vm   = "atacante-pa61"

Write-Output "== arrancar =="
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output

Write-Output "== GA por guestproperty (SIN credenciales) =="
$gaver = ""
for ($i = 0; $i -lt 50; $i++) {
  Start-Sleep -Seconds 5
  $g = (& $vbox guestproperty get $vm "/VirtualBox/GuestAdd/Version" 2>&1 | Out-String)
  if ($g -match "Value:\s*(.+)") { $gaver = $matches[1].Trim(); break }
}
Write-Output ("GUESTADD_VERSION=" + $gaver)

Write-Output "== propiedades GuestInfo (usuario logueado, red) =="
$en = (& $vbox guestproperty enumerate $vm 2>&1 | Out-String)
foreach ($ln in ($en -split "`n")) { if ($ln -match 'LoggedInUsers|OS/Product|Net/0/V4/IP|Net/Count') { Write-Output $ln.Trim() } }

Write-Output "== guestcontrol crudo con vboxuser/clave-de-RUA (ver el error real) =="
$r = (& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- /etc/hostname 2>&1 | Out-String)
Write-Output ("RAW_GUESTCONTROL<<<" + $r.Trim() + ">>>")

Write-Output "== apagar =="
(& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output
Write-Output "FIN_DIAG"
