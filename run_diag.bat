@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diag.ps1" > "%~dp0diag.out.txt" 2>&1
