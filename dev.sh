#!/bin/bash

echo ""
echo "============================================================"
echo "   RedEye AI Gateway - Full Stack Development Mode (WSL)"
echo "============================================================"
echo ""

# 1. Infrastructure Bootup (Commented out like in dev.bat)
# echo "[1/3] Starting Docker Infrastructure (Postgres, Redis, ClickHouse)..."
# docker-compose up -d
# echo "Waiting 5 seconds for DB and Cache to be ready..."
# sleep 5

# Cleanup function to safely kill all background processes on Ctrl+C
cleanup() {
    echo ""
    echo "Shutting down all RedEye services..."
    # Kills all child processes spawned by this script
    kill $(jobs -p) 2>/dev/null
    exit
}
# Trap SIGINT (Ctrl+C) and SIGTERM to trigger the cleanup
trap cleanup EXIT INT TERM

# 2. Start Rust Microservices
echo "[2/3] Launching Rust Services in the background..."

echo "Starting RedEye Auth..."
cargo run -p redeye_auth &

echo "Starting RedEye Gateway..."
cargo run -p redeye_gateway &

echo "Starting Supporting Services (Cache, Compliance, Tracer)..."
cargo run -p redeye_cache &
cargo run -p redeye_compliance &
cargo run -p redeye_tracer &

# 3. Frontend Dashboard
echo "[3/3] Launching React Dashboard..."
cd redeye_dashboard || exit
npm run dev &
cd ..

echo ""
echo "============================================================"
echo "   ALL CARGO FEATURES ARE RUNNING! 🚀"
echo "============================================================"
echo ""
echo "Dashboard: http://localhost:5173"
echo "Gateway:   http://localhost:8080"
echo "Auth:      http://localhost:8084"
echo ""
echo "Press [Ctrl+C] to gracefully stop all services."

# Wait indefinitely to keep the script running and trap active
wait