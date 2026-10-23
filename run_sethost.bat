@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0sethost_run.ps1" > "%~dp0sethost.out.txt" 2>&1
