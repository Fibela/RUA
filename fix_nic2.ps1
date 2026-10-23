$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
Write-Output "== asegurar golden apagada =="
$info = (& $vbox showvminfo golden-eje-linux --machinereadable 2>&1 | Out-String)
if ($info -match 'VMState="running"') {
  (& $vbox controlvm golden-eje-linux poweroff 2>&1 | Out-String) | Write-Output
  Start-Sleep -Seconds 5
} else { Write-Output "ya apagada" }

Write-Output "== borrar snapshot base-limpia (descripcion incorrecta) =="
(& $vbox snapshot golden-eje-linux delete base-limpia 2>&1 | Out-String) | Write-Output
Start-Sleep -Seconds 2

Write-Output "== aplicar nic2 intnet aislada (PA-61) =="
(& $vbox modifyvm golden-eje-linux --nic2 intnet --intnet2 "eje-pa61-aislada" --cableconnected2 on 2>&1 | Out-String) | Write-Output

Write-Output "== verificar NICs =="
$mi = (& $vbox showvminfo golden-eje-linux --machinereadable 2>&1 | Out-String)
foreach ($ln in ($mi -split "`n")) {
  if ($ln -match '^nic[1-4]=|^intnet[1-4]=|hostonlyadapter1=|cableconnected[1-4]=') { Write-Output $ln.Trim() }
}

Write-Output "== snapshot base-limpia (nuevo, correcto) =="
(& $vbox snapshot golden-eje-linux take base-limpia --description "clon RUA; nic1 hostonly (host 192.168.56.1), nic2 intnet eje-pa61-aislada sin servicios" 2>&1 | Out-String) | Write-Output
(& $vbox snapshot golden-eje-linux list 2>&1 | Out-String) | Write-Output
Write-Output "FIN_FIX"
