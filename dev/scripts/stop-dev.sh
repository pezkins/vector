#!/bin/bash
# Stop the Vectorize multi-agent development environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEV_DIR="$(dirname "$SCRIPT_DIR")"

# Detect docker compose command
if docker compose version &> /dev/null; then
    DOCKER_COMPOSE="docker compose"
elif docker-compose version &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
else
    echo "Error: Docker Compose not found."
    exit 1
fi

echo "Stopping Vector agents..."
cd "$DEV_DIR"

$DOCKER_COMPOSE down

echo ""
echo "All agents stopped."
echo ""
echo "To remove volumes (reset state): cd $DEV_DIR && $DOCKER_COMPOSE down -v"
