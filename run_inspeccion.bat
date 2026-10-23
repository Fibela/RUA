@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0inspeccion.ps1" > "%~dp0inspeccion.out.txt" 2>&1
