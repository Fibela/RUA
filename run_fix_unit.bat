@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0fix_unit_run.ps1" > "%~dp0fix_unit.out.txt" 2>&1
