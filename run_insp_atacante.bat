@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0insp_atacante.ps1" > "%~dp0insp_atacante.out.txt" 2>&1
