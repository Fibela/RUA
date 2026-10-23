$vbox    = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$exe     = "C:\Users\Alexx\Aura\target\release\rua-nucleo.exe"
$cfg     = "C:\Users\Alexx\Aura\nucleo\rua.config.json"
$pwdfile = "C:\Users\Alexx\Aura\secrets\guest.pwd"
$bin     = "C:\Users\Alexx\prueba.bin"

Write-Output "== 0) Estado de la VM =="
$info = (& $vbox showvminfo RUA --machinereadable 2>&1 | Out-String)
if ($info -notmatch 'VMState="running"') {
  Write-Output "Arrancando RUA (headless)..."
  (& $vbox startvm RUA --type headless 2>&1 | Out-String) | Write-Output
  Start-Sleep -Seconds 30
} else { Write-Output "VM ya corriendo" }

Write-Output "== 1) Esperando a que guestcontrol este LISTO (reintenta hasta 180s) =="
$ready = $false; $authOut = ""; $authExit = -1
for ($i=0; $i -lt 18; $i++) {
  $authOut = (& $vbox guestcontrol RUA run --username vboxuser --passwordfile $pwdfile --exe /bin/true -- /bin/true 2>&1 | Out-String)
  $authExit = $LASTEXITCODE
  if ($authOut -match 'not installed or not ready') { Start-Sleep -Seconds 10; continue }
  $ready = $true; break
}
Write-Output ("guestcontrol listo = " + $ready + " (intentos=" + $i + ")")

Write-Output "== 2) Resultado auth por passwordfile =="
Write-Output ("ExitA=" + $authExit)
Write-Output $authOut

Write-Output "== 3) Resultado auth por password en linea =="
$o2 = (& $vbox guestcontrol RUA run --username vboxuser --password Atacamesta67 --exe /bin/true -- /bin/true 2>&1 | Out-String)
Write-Output ("ExitB=" + $LASTEXITCODE)
Write-Output $o2

Write-Output "== 4) copy_in_verified (via nucleo) =="
if (!(Test-Path $bin)) { "contenido de prueba RUA $(Get-Date -Format o)" | Out-File -Encoding ascii -NoNewline $bin }
$env:RUA_CONFIG = $cfg
$req = @{ jsonrpc="2.0"; id=1; method="tools/call"; params=@{
           name="rua.copy_in_verified"; arguments=@{
             vm="RUA"; host_path=$bin; guest_dir="/tmp";
             guest_user="vboxuser"; password_file=$pwdfile } } } |
       ConvertTo-Json -Compress -Depth 8
($req | & $exe 2>&1 | Out-String) | Write-Output
