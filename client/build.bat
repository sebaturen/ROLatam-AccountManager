@echo off
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
set PATH=C:\Program Files\LLVM\bin;%PATH%
set OPENCV_LINK_LIBS=opencv_world4100
set OPENCV_LINK_PATHS=C:\opencv\opencv\build\x64\vc16\lib
set OPENCV_INCLUDE_PATHS=C:\opencv\opencv\build\include
cargo build %*
if %ERRORLEVEL% EQU 0 (
    echo Copying DLLs to target\debug...
    copy /Y lib\opencv_world4100.dll target\debug\opencv_world4100.dll >nul
    echo Build completed successfully!
)
