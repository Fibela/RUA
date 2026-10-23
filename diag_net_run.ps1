$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf  = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$vm   = "golden-eje-linux"
function Copia($src,$dst){ for($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){return ("copiado "+$dst)}; Start-Sleep 5 }; return ("FALLO "+$dst) }
Write-Output (Copia "C:\Users\Alexx\Aura\diag_net.sh" "/tmp/diag_net.sh")
Write-Output (Copia "$pwf" "/tmp/.p")
$out=""
for ($k=0;$k -lt 24;$k++){ $out=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/diag_net.sh 2>&1 | Out-String); if($out -notmatch 'Error starting guest session'){break}; Start-Sleep 5 }
Write-Output $out
Write-Output "FIN_DIAGNET_PS"
