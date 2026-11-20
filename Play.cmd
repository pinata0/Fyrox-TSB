@echo off
setlocal
cd /d "%~dp0"
cargo run --locked --release --bin tsb -- %*
set "tsb_exit_code=%errorlevel%"
if not "%tsb_exit_code%"=="0" pause
exit /b %tsb_exit_code%
