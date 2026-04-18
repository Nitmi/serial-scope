@echo off
setlocal enabledelayedexpansion

set APP_NAME=serial-scope

set ROOT_DIR=%~dp0..\..
pushd "%ROOT_DIR%" >nul

if not exist "dist" mkdir "dist"
copy /y "target\release\%APP_NAME%.exe" "dist\%APP_NAME%-windows-x86_64.exe" >nul

echo Created dist\%APP_NAME%-windows-x86_64.exe

popd >nul
endlocal
