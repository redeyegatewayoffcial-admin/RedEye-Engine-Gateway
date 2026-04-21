@echo off
setlocal
title RedEye AI Engine - Master Launcher

echo.
echo ============================================================
echo    RedEye AI Gateway - Full Stack Development Mode
echo ============================================================
echo.

:: 1. Infrastructure Bootup
::echo [1/3] Starting Docker Infrastructure (Postgres, Redis, ClickHouse)...::
::docker-compose up -d
::ech   o.
::echo Waiting 5 seconds for DB and Cache to be ready...
::timeout /t 5 /nobreak > nul

:: 2. Start Rust Microservices
echo [2/3] Launching Rust Services in separate windows...

:: Auth Service (Port 8084)
echo Starting RedEye Auth...
start "RedEye-Auth" cmd /c "cargo run -p redeye_auth"

:: Gateway (Port 8080)
echo Starting RedEye Gateway...
start "RedEye-Gateway" cmd /c "cargo run -p redeye_gateway"

:: Cache, Compliance & Tracer
echo Starting Supporting Services (Cache, Compliance, Tracer)...
start "RedEye-Cache" cmd /c "cargo run -p redeye_cache"
start "RedEye-Compliance" cmd /c "cargo run -p redeye_compliance"
start "RedEye-Tracer" cmd /c "cargo run -p redeye_tracer"
start "RedEye-Config" cmd /c "cargo run -p redeye_config"

:: 3. Frontend Dashboard
echo [3/3] Launching React Dashboard...
cd redeye_dashboard
start "RedEye-Dashboard" cmd /c "npm run dev"

echo.
echo ============================================================
echo    ALL CARGO FEATURES ARE RUNNING! 🚀
echo ============================================================
echo.
echo Dashboard: http://localhost:5173
echo Gateway: http://localhost:8080
echo Auth: http://localhost:8084
echo.

pause