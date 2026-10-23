@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0fix_nic2.ps1" > "%~dp0fix_nic2.out.txt" 2>&1
