@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0chk.ps1" > "%~dp0chk.out.txt" 2>&1
