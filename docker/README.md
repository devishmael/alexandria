# 🐳 Docker Container Sandbox for Alexandria

This directory provides containerized deployment and test isolation for Alexandria.

## Build Image Locally
```bash
docker build -t alexandria-kernel:latest -f docker/Dockerfile .
```

## Run Alexandria MCP Server in Container
```bash
docker run -i --rm -v $(pwd):/workspace alexandria-kernel:latest mcp
```

## Run via Docker Compose
```bash
docker compose -f docker/docker-compose.yml up
```
