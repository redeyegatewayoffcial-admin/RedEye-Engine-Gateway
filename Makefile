# ==============================================================================
# RedEye AI Engine - Developer Workflow Automation
# ==============================================================================

.PHONY: setup run test clean logs db-shell help

# Default target
help:
	@echo "Available commands:"
	@echo "  make setup     - One-click install for the entire workspace"
	@echo "  make run       - Bootstraps the stack and runs the gateway"
	@echo "  make test      - Executes all Rust and Frontend tests"
	@echo "  make clean     - Destroys artifacts, containers, and volumes"
	@echo "  make logs      - Streams container logs"
	@echo "  make db-shell  - Enters the Postgres terminal"

setup:
	@chmod +x scripts/setup.sh
	./scripts/setup.sh

run:
	@echo "🚀 Starting RedEye Gateway..."
	docker-compose up -d
	cargo run --bin redeye_gateway

test:
	@echo "🧪 Running workspace-wide tests..."
	cargo test --workspace
	cd redeye_dashboard && npm test

clean:
	@echo "🧹 Cleaning up..."
	docker-compose down -v --remove-orphans
	cargo clean
	rm -rf redeye_dashboard/node_modules redeye_dashboard/dist
	@echo "✅ Cleanup finished."

logs:
	docker-compose logs -f

db-shell:
	docker exec -it redeye_postgres psql -U RedEye -d RedEye
