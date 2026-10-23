@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0greenfield2_run.ps1" > "%~dp0greenfield2b.out.txt" 2>&1
