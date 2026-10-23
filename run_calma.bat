@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0calma_run.ps1" > "%~dp0calma_run.out.txt" 2>&1
