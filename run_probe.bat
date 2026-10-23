@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0probe_run.ps1" > "%~dp0probe.out.txt" 2>&1
