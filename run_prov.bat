@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0prov_run.ps1" > "%~dp0prov_run.out.txt" 2>&1
