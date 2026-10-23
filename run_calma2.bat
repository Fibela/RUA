@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0calma2_run.ps1" > "%~dp0calma2_run.out.txt" 2>&1
