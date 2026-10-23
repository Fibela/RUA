# Prueba B - sink en la IP host-only del anfitrion. Ctrl+C para parar.
# CORREGIDO: puerto 5514 (lo que dice la config firmada; 6514 era TLS-syslog, el
# agente manda TCP en claro) e IP fija 192.168.56.1 (la config la nombra literal).
$ws = "C:\Users\Alexx\Aura"
Write-Host "Sink en 192.168.56.1:5514 -> coincide con colector de la config firmada. Ctrl+C para parar." -ForegroundColor Cyan
& "$ws\target\release\rua-sink.exe" --listen 192.168.56.1:5514 --out "$ws\sink-eventos.jsonl" --print
