@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0prueba_full.ps1" > "%~dp0prueba_full.out.txt" 2>&1
