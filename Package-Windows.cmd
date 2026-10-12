@echo off
setlocal DisableDelayedExpansion
chcp 65001 >nul
title YYPlayer Windows Packaging
where pwsh.exe >nul 2>nul
if errorlevel 1 (
    set "YYPLAYER_PACKAGING_SHELL=%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe"
) else (
    set "YYPLAYER_PACKAGING_SHELL=pwsh.exe"
)
"%YYPLAYER_PACKAGING_SHELL%" -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\Package-Windows-Interactive.ps1" %*
set "YYPLAYER_PACKAGING_EXIT=%ERRORLEVEL%"
echo.
pause
exit /b %YYPLAYER_PACKAGING_EXIT%
