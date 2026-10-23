@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0greenfield3_run.ps1" > "%~dp0greenfield3.out.txt" 2>&1
