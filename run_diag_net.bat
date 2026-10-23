@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diag_net_run.ps1" > "%~dp0diag_net.out.txt" 2>&1
