$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
Write-Output "== host-only ifs (IP del anfitrion donde escucha el sink) =="
(& $vbox list hostonlyifs 2>&1 | Out-String) | Write-Output
Write-Output "== golden nic/vm info =="
$mi = (& $vbox showvminfo golden-eje-linux --machinereadable 2>&1 | Out-String)
foreach ($ln in ($mi -split "`n")) {
  if ($ln -match '^name=|^nic[1-4]=|hostonlyadapter1=|^intnet[1-4]=|cableconnected[1-4]=|macaddress[12]=') { Write-Output $ln.Trim() }
}
Write-Output "== snapshots golden =="
(& $vbox snapshot golden-eje-linux list 2>&1 | Out-String) | Write-Output
Write-Output "== dhcpservers =="
(& $vbox list dhcpservers 2>&1 | Out-String) | Write-Output
Write-Output "FIN_CHK"
