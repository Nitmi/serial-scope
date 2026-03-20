@echo off
setlocal enabledelayedexpansion

set APP_NAME=serial-scope
set VERSION=%~1
if "%VERSION%"=="" set VERSION=0.1.1

set ROOT_DIR=%~dp0..\..
pushd "%ROOT_DIR%" >nul

set DIST_DIR=dist\%APP_NAME%-windows-x86_64
if exist "%DIST_DIR%" rmdir /s /q "%DIST_DIR%"
mkdir "%DIST_DIR%"

copy /y "target\release\%APP_NAME%.exe" "%DIST_DIR%\%APP_NAME%.exe" >nul
copy /y "assets\app-icon.ico" "%DIST_DIR%\%APP_NAME%.ico" >nul
copy /y "README.md" "%DIST_DIR%\README.md" >nul
copy /y "LICENSE" "%DIST_DIR%\LICENSE" >nul

powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut((Join-Path (Resolve-Path '%DIST_DIR%').Path 'Serial Scope.lnk')); $s.TargetPath = (Resolve-Path '%DIST_DIR%\%APP_NAME%.exe').Path; $s.WorkingDirectory = (Resolve-Path '%DIST_DIR%').Path; $s.IconLocation = (Resolve-Path '%DIST_DIR%\%APP_NAME%.ico').Path; $s.Save()" >nul
powershell -NoProfile -Command "Compress-Archive -Path '%DIST_DIR%\*' -DestinationPath 'dist\%APP_NAME%-windows-x86_64.zip' -Force" >nul

echo Created dist\%APP_NAME%-windows-x86_64.zip

popd >nul
endlocal
