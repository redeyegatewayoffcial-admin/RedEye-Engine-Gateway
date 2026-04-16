#!/usr/bin/env bash

# ==============================================================================
# RedEye AI Engine - Golden Path Onboarding Script
# ------------------------------------------------------------------------------
# Usage: ./scripts/setup.sh
# description: Sets up the entire development environment idempotently.
# ==============================================================================

set -e # Exit on error
set -u # Exit on unset variables

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting RedEye AI Engine Setup...${NC}"

# 1. Check for required system dependencies
function check_dep() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}❌ $1 is not installed. Please install it to continue.${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ $1 found.${NC}"
}

echo -e "\n${BLUE}🔍 Checking dependencies...${NC}"
check_dep "docker"
check_dep "docker-compose"
check_dep "cargo"
check_dep "npm"

# 2. Setup Environment Variables
echo -e "\n${BLUE}🔐 Setting up environment variables...${NC}"
if [ ! -f .env ]; then
    echo -e "Copying .env.example to .env..."
    cp .env.example .env
    echo -e "${GREEN}✅ .env file created. Please update secrets if necessary.${NC}"
else
    echo -e "${GREEN}✅ .env file already exists.${NC}"
fi

# 3. Boot up infrastructure
echo -e "\n${BLUE}🐳 Starting Docker infrastructure...${NC}"
docker-compose up -d --remove-orphans

# 4. Wait for databases to be ready
echo -e "\n${BLUE}⏳ Waiting for databases to initialize...${NC}"
# Simple wait loop for Postgres
until docker exec redeye_postgres pg_isready -U RedEye -d RedEye > /dev/null 2>&1; do
  echo -n "."
  sleep 2
done
echo -e "\n${GREEN}✅ Databases are ready.${NC}"

# 5. Build components
echo -e "\n${BLUE}🛠️  Building components...${NC}"

echo -e "📦 Building Rust workspace..."
cargo build

echo -e "📦 Initializing dashboard..."
cd redeye_dashboard
npm install
cd ..

echo -e "\n${GREEN}✨ Setup complete!${NC}"
echo -e "${BLUE}💡 Use 'make run' to start the gateway.${NC}"
