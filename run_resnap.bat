@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0resnap_run.ps1" > "%~dp0resnap.out.txt" 2>&1
