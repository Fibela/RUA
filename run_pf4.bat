@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0pf4.ps1" > "%~dp0pf4.out.txt" 2>&1
