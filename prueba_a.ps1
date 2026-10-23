$ws      = "C:\Users\Alexx\Aura"
$vbox    = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$exe     = Join-Path $ws "target\release\rua-nucleo.exe"
$cfg     = Join-Path $ws "nucleo\rua.config.json"
$pwdfile = Join-Path $ws "secrets\guest.pwd"
$bin     = "C:\Users\Alexx\prueba.bin"
Write-Output "== 1) build (release) =="
Push-Location $ws; (cargo build --release 2>&1 | Out-String) | Write-Output; Pop-Location
if (!(Test-Path $exe)) { Write-Output ("FALTA " + $exe); exit }
Write-Output "== 2) Guest Additions en RUA =="
(& $vbox guestproperty get RUA /VirtualBox/GuestAdd/Version 2>&1 | Out-String) | Write-Output
if (!(Test-Path $bin)) { "contenido de prueba RUA $(Get-Date -Format o)" | Out-File -Encoding ascii -NoNewline $bin }
if (!(Test-Path $pwdfile) -or ((Get-Content -Raw $pwdfile).Trim().Length -eq 0)) { Write-Output "FALTA guest.pwd"; exit }
$env:RUA_CONFIG = $cfg
$req = @{ jsonrpc="2.0"; id=1; method="tools/call"; params=@{
           name="rua.copy_in_verified"; arguments=@{
             vm="RUA"; host_path=$bin; guest_dir="/tmp";
             guest_user="vboxeruser"; password_file=$pwdfile } } } |
       ConvertTo-Json -Compress -Depth 8
Write-Output "== 3) copy_in_verified =="
($req | & $exe 2>&1 | Out-String) | Write-Output
Write-Output "== 4) CertUtil (compara con host_sha256) =="
(CertUtil -hashfile $bin SHA256 2>&1 | Out-String) | Write-Output
