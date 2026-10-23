@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0fix_bin_run.ps1" > "%~dp0fix_bin.out.txt" 2>&1
