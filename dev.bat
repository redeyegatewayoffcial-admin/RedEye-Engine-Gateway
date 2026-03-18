@echo off
echo Starting RedEye AI Engine Microservices...

start "RedEye Gateway" cmd /k "cargo run --bin redeye_gateway"
start "RedEye Cache" cmd /k "cargo run --bin redeye_cache"
start "RedEye Tracer" cmd /k "cargo run --bin redeye_tracer"
start "RedEye Compliance" cmd /k "cargo run --bin redeye_compliance"

default-run = "redeye_gateway"

echo All services started in separate windows!