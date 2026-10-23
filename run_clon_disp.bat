@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0clon_disp.ps1" > "%~dp0clon_disp.out.txt" 2>&1
