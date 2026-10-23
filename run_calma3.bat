@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0calma3_run.ps1" > "%~dp0calma3.out.txt" 2>&1
