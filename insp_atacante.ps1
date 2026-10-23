$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
foreach ($vm in @("Atacante","eje-prueba-pa117","eje-prueba-nueva")) {
  Write-Output ("######## " + $vm + " ########")
  $mi = (& $vbox showvminfo $vm --machinereadable 2>&1 | Out-String)
  foreach ($ln in ($mi -split "`n")) {
    if ($ln -match '^ostype=|^VMState=|^memory=|^nic[1-4]=|^intnet[1-4]=|^hostonlyadapter[1-4]=|^macaddress[1-4]=|^cableconnected[1-4]=|^"SATA|^"IDE|^"NVMe|\.vdi|\.vmdk|^CurrentSnapshotName=|^SnapshotName') { Write-Output $ln.Trim() }
  }
  Write-Output ""
}
Write-Output "FIN_INSP"
