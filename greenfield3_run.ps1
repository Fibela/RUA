$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$pwf="C:\Users\Alexx\Aura\secrets\guest.pwd"
$vm="golden-eje-linux"
$sinkExe="C:\Users\Alexx\Aura\target\release\rua-sink.exe"
$sinkOut="C:\Users\Alexx\Aura\sink-eventos.jsonl"
function Copia($src,$dst){ for($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm copyto --username vboxuser --passwordfile "$pwf" "$src" "$dst" 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){return ("copiado "+$dst)}; Start-Sleep 5 }; return ("FALLO "+$dst) }
Write-Output "== sink limpio en segundo plano =="
Get-Process rua-sink -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep 2
[System.IO.File]::WriteAllText($sinkOut,"")
$sink = Start-Process -FilePath $sinkExe -ArgumentList '--listen','192.168.56.1:5514','--out',$sinkOut,'--print' -PassThru -WindowStyle Hidden
Start-Sleep 3
Write-Output ("sink PID " + $sink.Id)
Write-Output "== restaurar d520dd87 =="
& $vbox controlvm $vm poweroff 2>&1 | Out-Null
Start-Sleep 4
(& $vbox snapshot $vm restore base-identidad-anclada 2>&1 | Out-String) | Write-Output
Start-Sleep 2
(& $vbox startvm $vm --type headless 2>&1 | Out-String) | Write-Output
Start-Sleep 15
Write-Output (Copia "$pwf" "/tmp/.p")
Write-Output (Copia "C:\Users\Alexx\Aura\greenfield2.sh" "/tmp/greenfield2.sh")
$o=""
for ($k=0;$k -lt 24;$k++){ $o=(& $vbox guestcontrol $vm run --username vboxuser --passwordfile "$pwf" --exe /bin/sh -- /tmp/greenfield2.sh 2>&1 | Out-String); if($o -notmatch 'Error starting guest session' -and $o -notmatch 'not ready'){break}; Start-Sleep 5 }
Write-Output "===== GF3 OUT ====="; Write-Output $o
Write-Output "== latir 80s (sink captura) =="
Start-Sleep 80
Write-Output "== parar sink y restaurar d520dd87 =="
Stop-Process -Id $sink.Id -Force -ErrorAction SilentlyContinue
& $vbox controlvm $vm poweroff 2>&1 | Out-Null
Start-Sleep 4
(& $vbox snapshot $vm restore base-identidad-anclada 2>&1 | Out-String) | Write-Output
Write-Output "FIN_GF3_PS"
