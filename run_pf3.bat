@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0pf3.ps1" > "%~dp0pf3.out.txt" 2>&1
