@echo off
cd /d "%~dp0"
"target\debug\tsb.exe" %*
if errorlevel 1 pause
