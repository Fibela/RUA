$ws="C:\Users\Alexx\Aura"
$exe="C:\Users\Alexx\Aura\target\release\rua-nucleo.exe"
$cfg="C:\Users\Alexx\Aura\nucleo\rua.config.json"
$pwdfile="C:\Users\Alexx\Aura\secrets\guest.pwd"
Write-Output "== build (fix argv del hasher) =="
Push-Location $ws; (cargo build --release 2>&1 | Out-String) | Write-Output; Pop-Location
$stamp = Get-Random
$bin = "C:\Users\Alexx\prueba_$stamp.bin"
"RUA transporte verificado $(Get-Date -Format o) nonce=$stamp" | Out-File -Encoding ascii -NoNewline $bin
Write-Output ("archivo host: " + $bin)
Write-Output "== copy_in_verified =="
$env:RUA_CONFIG=$cfg
$req = @{ jsonrpc="2.0"; id=1; method="tools/call"; params=@{
           name="rua.copy_in_verified"; arguments=@{
             vm="RUA"; host_path=$bin; guest_dir="/tmp";
             guest_user="vboxuser"; password_file=$pwdfile } } } |
       ConvertTo-Json -Compress -Depth 8
($req | & $exe 2>&1 | Out-String) | Write-Output
Write-Output "== CertUtil del host =="
(CertUtil -hashfile $bin SHA256 2>&1 | Out-String) | Write-Output
