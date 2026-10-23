$ErrorActionPreference = "Continue"
$vbox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vbox)) {
  $alt = (Get-Command VBoxManage.exe -ErrorAction SilentlyContinue).Source
  if ($alt) { $vbox = $alt }
}
Write-Output ("VBOXMANAGE=" + $vbox)

Write-Output "== VMs actuales =="
$vms = (& $vbox list vms 2>&1 | Out-String)
Write-Output $vms

if ($vms -match "golden-eje-linux") {
  Write-Output "golden-eje-linux YA existe -- no clono (idempotente)."
} else {
  Write-Output "== apagar/limpiar estado de RUA =="
  $info = (& $vbox showvminfo RUA --machinereadable 2>&1 | Out-String)
  if ($info -match 'VMState="running"') {
    (& $vbox controlvm RUA poweroff 2>&1 | Out-String) | Write-Output
    Start-Sleep -Seconds 6
  } elseif ($info -match 'VMState="saved"') {
    (& $vbox discardstate RUA 2>&1 | Out-String) | Write-Output
  } else {
    Write-Output "RUA no estaba corriendo"
  }

  Write-Output "== adaptador host-only =="
  $hoi = (& $vbox list hostonlyifs 2>&1 | Out-String)
  Write-Output $hoi
  $name = [regex]::Match($hoi, "Name:\s+(.+)").Groups[1].Value.Trim()
  if (-not $name) {
    Write-Output "no hay host-only; creando..."
    (& $vbox hostonlyif create 2>&1 | Out-String) | Write-Output
    $hoi = (& $vbox list hostonlyifs 2>&1 | Out-String)
    $name = [regex]::Match($hoi, "Name:\s+(.+)").Groups[1].Value.Trim()
  }
  $ip = [regex]::Match($hoi, "IPAddress:\s+([\d.]+)").Groups[1].Value.Trim()
  if (-not $ip) {
    (& $vbox hostonlyif ipconfig "$name" --ip 192.168.56.1 --netmask 255.255.255.0 2>&1 | Out-String) | Write-Output
    $ip = "192.168.56.1"
  }
  Write-Output ("host-only adapter = [" + $name + "]  IP_HOST = " + $ip)

  Write-Output "== clonar RUA -> golden-eje-linux =="
  (& $vbox clonevm RUA --name golden-eje-linux --register 2>&1 | Out-String) | Write-Output

  Write-Output "== adaptadores golden (nic1 hostonly, nic2 aislada PA-61) =="
  (& $vbox modifyvm golden-eje-linux --nic1 hostonly --hostonlyadapter1 "$name" --cableconnected1 on 2>&1 | Out-String) | Write-Output
  (& $vbox modifyvm golden-eje-linux --nic2 intnet --intnet2 "eje-pa61-aislada" --cableconnected2 on 2>&1 | Out-String) | Write-Output

  Write-Output "== snapshot base-limpia =="
  (& $vbox snapshot golden-eje-linux take base-limpia --description "clon de RUA; nic1 hostonly, nic2 aislada" 2>&1 | Out-String) | Write-Output

  Write-Output ("COLECTOR_IP=" + $ip)
}

Write-Output "== arrancar golden (headless) para leer hostname =="
(& $vbox startvm golden-eje-linux --type headless 2>&1 | Out-String) | Write-Output

$pwf = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$hn = ""
$gip = ""
for ($i = 0; $i -lt 30; $i++) {
  Start-Sleep -Seconds 5
  $ga = (& $vbox guestproperty get golden-eje-linux "/VirtualBox/GuestAdd/Version" 2>&1 | Out-String)
  if ($ga -match "Value:") {
    $h = (& $vbox guestcontrol golden-eje-linux run --username vboxuser --passwordfile "$pwf" --exe /bin/cat -- cat /etc/hostname 2>&1 | Out-String)
    if ($h -and $h.Trim() -ne "") { $hn = $h.Trim() }
    $g = (& $vbox guestproperty get golden-eje-linux "/VirtualBox/GuestInfo/Net/0/V4/IP" 2>&1 | Out-String)
    $gip = [regex]::Match($g, "Value:\s+([\d.]+)").Groups[1].Value.Trim()
    if ($hn -ne "") { break }
  }
  Write-Output ("...esperando GuestAdditions intento " + ($i + 1))
}
Write-Output ("ETC_HOSTNAME=" + $hn)
Write-Output ("GUEST_IP_NET0=" + $gip)

Write-Output "== apagar golden =="
(& $vbox controlvm golden-eje-linux poweroff 2>&1 | Out-String) | Write-Output

Write-Output "== VMs tras paso 1 =="
(& $vbox list vms 2>&1 | Out-String) | Write-Output
Write-Output "FIN_PASO1"
