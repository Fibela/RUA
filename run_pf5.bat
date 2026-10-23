@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0pf5.ps1" > "%~dp0pf5.out.txt" 2>&1
