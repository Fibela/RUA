@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diag_svc_run.ps1" > "%~dp0diag_svc.out.txt" 2>&1
