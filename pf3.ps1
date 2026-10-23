$ws="C:\Users\Alexx\Aura"
$vbox="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$exe="C:\Users\Alexx\Aura\target\release\rua-nucleo.exe"
$cfg="C:\Users\Alexx\Aura\nucleo\rua.config.json"
$pwdfile="C:\Users\Alexx\Aura\secrets\guest.pwd"
$bin="C:\Users\Alexx\prueba.bin"
Write-Output "== build (release, con el fix de copy_in) =="
Push-Location $ws; (cargo build --release 2>&1 | Out-String) | Write-Output; Pop-Location
$info=(& $vbox showvminfo RUA --machinereadable 2>&1 | Out-String)
if ($info -notmatch 'VMState="running"'){ Write-Output "arrancando..."; (& $vbox startvm RUA --type headless 2>&1|Out-String)|Write-Output; Start-Sleep -Seconds 45 }
if (!(Test-Path $bin)) { "contenido de prueba RUA $(Get-Date -Format o)" | Out-File -Encoding ascii -NoNewline $bin }
Write-Output "== copy_in_verified =="
$env:RUA_CONFIG=$cfg
$req = @{ jsonrpc="2.0"; id=1; method="tools/call"; params=@{
           name="rua.copy_in_verified"; arguments=@{
             vm="RUA"; host_path=$bin; guest_dir="/tmp";
             guest_user="vboxuser"; password_file=$pwdfile } } } |
       ConvertTo-Json -Compress -Depth 8
($req | & $exe 2>&1 | Out-String) | Write-Output
Write-Output "== CertUtil =="
(CertUtil -hashfile $bin SHA256 2>&1 | Out-String) | Write-Output
