@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diag_disk_run.ps1" > "%~dp0diag_disk.out.txt" 2>&1
