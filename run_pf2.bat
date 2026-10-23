@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0pf2.ps1" > "%~dp0pf2.out.txt" 2>&1
