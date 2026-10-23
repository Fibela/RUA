$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwdfile="C:\Users\Alexx\Aura\secrets\guest.pwd"
$sh="C:\Users\Alexx\Aura\survey.sh"
$U="vboxuser"
Write-Output "== estado VM =="
$info=(& $vbox showvminfo RUA --machinereadable 2>&1 | Out-String)
if ($info -notmatch 'VMState="running"'){ Write-Output "arrancando RUA..."; (& $vbox startvm RUA --type headless 2>&1|Out-String)|Write-Output; Start-Sleep -Seconds 35 } else { Write-Output "ya corriendo" }
$ready=$false
for($i=0;$i -lt 18;$i++){
  $t=(& $vbox guestcontrol RUA run --username $U --passwordfile $pwdfile --exe /bin/true -- /bin/true 2>&1 | Out-String)
  if($t -match 'not ready|not installed'){ Start-Sleep -Seconds 10; continue }
  $ready=$true; break
}
Write-Output ("guestcontrol listo = " + $ready)
if(-not $ready){ Write-Output "guestcontrol no disponible"; exit }
Write-Output "== copiar survey.sh al guest =="
(& $vbox guestcontrol RUA --username $U --passwordfile $pwdfile copyto $sh /tmp/survey.sh 2>&1 | Out-String) | Write-Output
Write-Output "== SURVEY =="
(& $vbox guestcontrol RUA run --username $U --passwordfile $pwdfile --exe /bin/sh --timeout 90000 -- /tmp/survey.sh 2>&1 | Out-String) | Write-Output
