$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$vm   = "atacante-pa61"

Write-Output "== asegurar apagada =="
$info = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
if ($info -match 'VMState="running"') { (& $vbox controlvm $vm poweroff 2>&1 | Out-String) | Write-Output; Start-Sleep -Seconds 6 }
else { Write-Output "ya apagada" }

Write-Output "== snapshot con-nat-alcanzable (ANTES de aislar) =="
(& $vbox snapshot $vm take con-nat-alcanzable --description "clon de Atacante antes de aislar: nic1 NAT, alcanzable para arreglar netplan/GA desde dentro" 2>&1 | Out-String) | Write-Output
Start-Sleep -Seconds 2

Write-Output "== aislar: nic1 -> intnet eje-pa61-aislada, resto none =="
(& $vbox modifyvm $vm --nic1 intnet --intnet1 "eje-pa61-aislada" --cableconnected1 on 2>&1 | Out-String) | Write-Output
(& $vbox modifyvm $vm --nic2 none --nic3 none --nic4 none 2>&1 | Out-String) | Write-Output

Write-Output "== MAC FINAL + nics (red definitiva) =="
$mi = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
foreach ($ln in ($mi -split "`n")) { if ($ln -match '^macaddress1=|^nic[1-4]=|^intnet1=|cableconnected1=') { Write-Output $ln.Trim() } }

Write-Output "== snapshot base-dispositivo (estado aislado) =="
(& $vbox snapshot $vm take base-dispositivo --description "nic unica intnet eje-pa61-aislada, DHCP sin servidor (DISCOVER periodico), sin NAT" 2>&1 | Out-String) | Write-Output
(& $vbox snapshot $vm list 2>&1 | Out-String) | Write-Output
Write-Output "FIN_ISO"
