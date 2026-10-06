@echo off
setlocal
title Cute Of Duty 一键启动
cd /d "%~dp0"

rem 一键启动：先起服务器，等端口就绪，再启动客户端；客户端退出后收尾服务器进程。
rem 本脚本随 Release zip 分发，须与 cod_server.exe / cod1.exe 在同一目录。

if not exist "cod_server.exe" (
    echo [错误] 未找到 cod_server.exe，请将本脚本与双端 exe 放在同一目录。
    pause
    exit /b 1
)
if not exist "cod1.exe" (
    echo [错误] 未找到 cod1.exe，请将本脚本与双端 exe 放在同一目录。
    pause
    exit /b 1
)

echo [1/3] 启动服务器 cod_server.exe（最小化窗口）...
start "COD Server" /min "cod_server.exe"

echo [2/3] 等待服务器就绪（最多 30 秒）...
powershell -NoProfile -Command "for($i=0;$i -lt 60;$i++){try{$c=New-Object Net.Sockets.TcpClient('127.0.0.1',8888);$c.Close();exit 0}catch{Start-Sleep -Milliseconds 500}};exit 1"
if errorlevel 1 (
    echo [警告] 服务器 30 秒内未就绪，仍继续启动客户端（客户端每 2 秒自动重连）。
)

echo [3/3] 启动客户端 cod1.exe（本窗口关闭前请勿手动关闭服务器窗口）...
start "" /wait "cod1.exe"

echo 客户端已退出，正在关闭服务器...
taskkill /f /im cod_server.exe >nul 2>&1
echo 再见！
timeout /t 2 /nobreak >nul
endlocal
