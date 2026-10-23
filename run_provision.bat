@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0provision_golden.ps1" > "%~dp0provision.out.txt" 2>&1
