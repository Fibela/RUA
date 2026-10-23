@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0iso_disp.ps1" > "%~dp0iso_disp.out.txt" 2>&1
