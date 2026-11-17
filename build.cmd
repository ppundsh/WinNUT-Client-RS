@echo off
rem Double-click to check tools, run tests and build the Windows installers (see scripts\build-windows.ps1).
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-windows.ps1" %*
set EXITCODE=%ERRORLEVEL%
echo.
pause
exit /b %EXITCODE%
