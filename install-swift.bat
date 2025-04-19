@echo off

REM Define source directory
set SRC_DIR=src

REM Name of the executable
set EXECUTABLE=main.exe

REM Ensure the source directory exists
if not exist "%SRC_DIR%" (
    echo Error: Source directory "%SRC_DIR%" not found.
    exit /b 1
)

REM Compile Swift source files
swiftc "%SRC_DIR%\main.swift" "%SRC_DIR%\ls.swift" "%SRC_DIR%\cat.swift" "%SRC_DIR%\color.swift" "%SRC_DIR%\cd.swift" "%SRC_DIR%\sleep.swift" "%SRC_DIR%\cp.swift" "%SRC_DIR%\mkdir.swift" "%SRC_DIR%\rm.swift" "%SRC_DIR%\time.swift" -o "%EXECUTABLE%"

REM Check if compilation was successful
if %errorlevel% equ 0 (
    echo Compilation successful.
) else (
    echo Compilation failed.
)
