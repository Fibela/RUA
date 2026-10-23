@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0greenfield_run.ps1" > "%~dp0greenfield.out.txt" 2>&1
