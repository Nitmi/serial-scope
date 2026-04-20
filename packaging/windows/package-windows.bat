@echo off
setlocal enabledelayedexpansion

set APP_NAME=serial-scope
set PORTABLE_NAME=%APP_NAME%-windows-x86_64-portable.exe
set SETUP_NAME=%APP_NAME%-windows-x86_64-setup.exe

set ROOT_DIR=%~dp0..\..
pushd "%ROOT_DIR%" >nul

if not exist "dist" mkdir "dist"
copy /y "target\release\%APP_NAME%.exe" "dist\%PORTABLE_NAME%" >nul

echo Created dist\%PORTABLE_NAME%

where iscc >nul 2>nul
if errorlevel 1 (
    echo Inno Setup compiler not found, skipping installer package.
) else (
    set ISCC_ARGS=
    if exist "%ProgramFiles(x86)%\Inno Setup 6\Languages\ChineseSimplified.isl" (
        set ISCC_ARGS=/DMyMessagesFile=compiler:Languages\ChineseSimplified.isl
    )
    iscc %ISCC_ARGS% packaging\windows\serial-scope.iss >nul
    if errorlevel 1 (
        popd >nul
        endlocal
        exit /b 1
    )
    echo Created dist\%SETUP_NAME%
)

popd >nul
endlocal
