@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0paso1.ps1" > "%~dp0paso1.out.txt" 2>&1
