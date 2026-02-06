#!/bin/bash
# Start the Vectorize multi-agent development environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEV_DIR="$(dirname "$SCRIPT_DIR")"

# Detect docker compose command (new: "docker compose", old: "docker-compose")
if docker compose version &> /dev/null; then
    DOCKER_COMPOSE="docker compose"
elif docker-compose version &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
else
    echo "Error: Docker Compose not found. Please install Docker Desktop or Docker Compose."
    exit 1
fi

echo "=========================================="
echo "  Vectorize Multi-Agent Dev Environment  "
echo "=========================================="
echo ""
echo "Using: $DOCKER_COMPOSE"
echo ""

cd "$DEV_DIR"

# Start containers
echo "Starting Vector agents..."
$DOCKER_COMPOSE up -d

echo ""
echo "Waiting for agents to be healthy..."
sleep 5

# Check health of each agent
echo ""
echo "Agent Status:"
echo "-------------"

check_agent() {
    local name=$1
    local port=$2
    if curl -s "http://localhost:$port/health" > /dev/null 2>&1; then
        echo "✓ $name (localhost:$port) - healthy"
    else
        echo "✗ $name (localhost:$port) - not responding"
    fi
}

check_agent "vector-prod-1" 8686
check_agent "vector-prod-2" 8687
check_agent "vector-prod-3" 8688
check_agent "vector-staging-1" 8689
check_agent "vector-staging-2" 8690

echo ""
echo "=========================================="
echo "  Agent URLs for Registration            "
echo "=========================================="
echo ""
echo "Production Group:"
echo "  - http://localhost:8686 (vector-prod-1)"
echo "  - http://localhost:8687 (vector-prod-2)"
echo "  - http://localhost:8688 (vector-prod-3)"
echo ""
echo "Staging Group:"
echo "  - http://localhost:8689 (vector-staging-1)"
echo "  - http://localhost:8690 (vector-staging-2)"
echo ""
echo "=========================================="
echo "  Commands                               "
echo "=========================================="
echo ""
echo "View logs:     cd $DEV_DIR && $DOCKER_COMPOSE logs -f"
echo "Stop all:      $SCRIPT_DIR/stop-dev.sh"
echo "Scale prod:    cd $DEV_DIR && $DOCKER_COMPOSE up -d --scale vector-prod-1=5"
echo ""
