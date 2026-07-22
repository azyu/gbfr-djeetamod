@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-release.ps1"
set "BUILD_EXIT_CODE=%ERRORLEVEL%"
echo.
pause
exit /b %BUILD_EXIT_CODE%
