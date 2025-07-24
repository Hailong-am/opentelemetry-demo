# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The OpenTelemetry Demo is a microservice-based e-commerce application demonstrating OpenTelemetry instrumentation across 11 services using 8 different technology stacks. It showcases distributed tracing, metrics, and logging in a realistic environment.

## Architecture

### Service Architecture

| Service | Tech Stack | Port | Primary Function | Communication |
|---------|------------|------|------------------|---------------|
| **Frontend** | Node.js (Next.js 15) | 8080 | Web UI & API Gateway | HTTP (client) + gRPC (backend) |
| **Product Catalog** | Go | 3550 | Product listing/search | gRPC |
| **Cart** | .NET 8 (C#) | 7070 | Shopping cart | gRPC + Valkey |
| **Checkout** | Go | 5050 | Order processing | gRPC + Kafka |
| **Currency** | C++ | 7001 | Currency conversion | gRPC |
| **Payment** | Node.js | 50051 | Payment processing | gRPC |
| **Email** | Ruby (Sinatra) | 6060 | Email notifications | HTTP |
| **Recommendation** | Python (Flask) | 9001 | Product recommendations | gRPC |
| **Shipping** | Rust | 50050 | Shipping quotes | HTTP |
| **Accounting** | .NET 8 (C#) | - | Financial processing | gRPC + Kafka |
| **Fraud Detection** | Kotlin (JVM) | - | Fraud analysis | Kafka |
| **Ad Service** | Java | 9555 | Advertisement serving | gRPC |

### Communication Patterns

- **Primary**: gRPC with Protocol Buffers (`/pb/demo.proto`)
- **Event Streaming**: Kafka for async processing
- **Browser**: HTTP/REST via frontend
- **Caching**: Valkey (Redis protocol) for cart service

### Data Flow

1. **User Request** → Frontend (Next.js) → Envoy Proxy
2. **Frontend** translates HTTP to gRPC for backend services
3. **Checkout Service** orchestrates order flow across Payment, Shipping, Email
4. **Kafka** streams orders to Accounting & Fraud Detection
5. **OpenTelemetry** captures traces, metrics, logs across all services

## Development Commands

### Quick Start

```bash
# Start full demo
make start

# Start minimal version (fewer services)
make start-minimal

# Stop all services
make stop
```

### Development Workflow

```bash
# Build all services
make build

# Build specific service
make restart service=frontend
make redeploy service=cart

# Generate protobuf files after changes
make generate-protobuf

# Clean generated files
make clean
```

### Testing

```bash
# Run all tests
make run-tests

# Run specific trace-based tests
make run-tracetesting SERVICES_TO_TEST="frontend payment"

# Run linting and checks
make check
make misspell
make markdownlint
```

### Local Development

For frontend development:
```bash
docker compose run --service-ports -e NODE_ENV=development --volume $(pwd)/src/frontend:/app --volume $(pwd)/pb:/app/pb --user node --entrypoint sh frontend
# Inside container: npm run dev
```

## Key Configuration Files

### Protocol Buffers
- **File**: `/pb/demo.proto`
- **Purpose**: Defines all gRPC services and messages
- **Usage**: Regenerate with `make generate-protobuf`

### Environment Configuration
- **File**: `.env`
- **Purpose**: Service ports, image versions, connection strings
- **Override**: `.env.override` for local changes

### Docker Compose
- **File**: `docker-compose.yml`
- **Minimal**: `docker-compose.minimal.yml`
- **Testing**: `docker-compose-tests.yml`

### OpenTelemetry Configuration
- **Collector**: `src/otel-collector/otelcol-config.yml`
- **Prometheus**: `src/prometheus/prometheus-config.yaml`
- **Grafana**: `src/grafana/provisioning/`

## Service Structure

### Go Services (Checkout, Product Catalog)
- **Entry**: `main.go`
- **Telemetry**: OpenTelemetry initialization in main
- **gRPC**: Server setup with `otelgrpc` interceptors
- **Dependencies**: Environment variables for service discovery

### Node.js Services (Frontend, Payment)
- **Frontend**: Next.js 15 with TypeScript, React 19
- **Telemetry**: `@opentelemetry/auto-instrumentations-node`
- **Build**: `npm run build` and `npm run dev`
- **gRPC**: Generated TypeScript clients from protobuf

### .NET Services (Cart, Accounting)
- **Framework**: .NET 8 with C#
- **Entry**: `Program.cs`
- **Docker**: Multi-stage builds in service directories

### Python Services (Recommendation)
- **Framework**: Flask
- **Telemetry**: OpenTelemetry Python SDK
- **gRPC**: Generated Python modules from protobuf

## Telemetry Setup

### Components
- **Collector**: Centralized OpenTelemetry Collector
- **Jaeger**: Trace storage and UI (`http://localhost:8080/jaeger/ui`)
- **Prometheus**: Metrics collection (`http://localhost:8080/prometheus`)
- **Grafana**: Dashboards (`http://localhost:8080/grafana`)
- **OpenSearch**: Log aggregation via Data Prepper

### Instrumentation
- **Auto-instrumentation** across all languages
- **Manual spans** for business logic
- **Feature flags** via FlagD for controlled testing
- **Load generation** with Locust for realistic traffic

## Testing Strategy

### Trace-based Testing
- **Framework**: Tracetest
- **Location**: `/test/tracetesting/`
- **Tests**: Service-specific and end-to-end flows
- **Validation**: Full request traces across services

### Test Types
1. **Unit tests** for individual services
2. **Integration tests** for service interactions
3. **End-to-end tests** simulating user journeys
4. **Load tests** with configurable user scenarios

## Common Tasks

### Adding a New Service
1. Create service directory in `/src/[service-name]/`
2. Define gRPC interface in `/pb/demo.proto`
3. Implement service with OpenTelemetry
4. Add Dockerfile and update docker-compose.yml
5. Add service to .env with port configuration
6. Generate protobuf files: `make generate-protobuf`
7. Add trace-based tests

### Modifying Protocol Buffers
1. Edit `/pb/demo.proto`
2. Run `make generate-protobuf`
3. Update service implementations
4. Run tests: `make run-tests`

### Debugging Traces
1. Start Jaeger: `http://localhost:8080/jaeger/ui`
2. Look for traces by service or operation
3. Check individual spans for errors
4. Use Grafana for metrics correlation

### Feature Flags
- **Service**: FlagD on port 8013
- **UI**: `http://localhost:8080/feature/`
- **Config**: `src/flagd/demo.flagd.json`
- **SDK**: OpenFeature SDKs available for all languages