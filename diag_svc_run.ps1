$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$vm   = "golden-eje-linux"
for ($k=0;$k -lt 20;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "C:\Users\Alexx\Aura\diag_svc.sh" "/tmp/diag_svc.sh" 2>&1 | Out-String); if ($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){break}; Start-Sleep 5 }
$out=""
for ($k=0;$k -lt 20;$k++){ $out=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/diag_svc.sh 2>&1 | Out-String); if ($out -notmatch 'Error starting guest session'){break}; Start-Sleep 5 }
Write-Output $out
Write-Output "FIN_DIAGSVC_PS"
