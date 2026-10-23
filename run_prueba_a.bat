@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0prueba_a.ps1" > "%~dp0prueba_a.out.txt" 2>&1
