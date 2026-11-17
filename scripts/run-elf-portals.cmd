@echo off
pushd "%~dp0.."
"target\debug\tsb.exe" --solo %*
set "portal_exit_code=%errorlevel%"
popd
exit /b %portal_exit_code%
