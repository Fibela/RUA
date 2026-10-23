@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diag_ata.ps1" > "%~dp0diag_ata.out.txt" 2>&1
