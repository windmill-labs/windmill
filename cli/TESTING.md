# Testing Guide for Windmill CLI

This document describes the comprehensive testing structure for the Windmill CLI using a real containerized Windmill backend for reliable, maintainable testing.

## 🚀 Quick Start

### Run All Tests
```bash
cd /home/alex/windmill/windmill/cli

# Run all tests (unit + integration)
source ~/.zshrc && deno run --allow-all test/runner.ts

# Run only unit tests (fast, no containers)
source ~/.zshrc && deno run --allow-all test/runner.ts --unit-only

# Run only integration tests (containerized backend)
source ~/.zshrc && deno run --allow-all test/runner.ts --integration-only
```

### Run Specific Test Categories
```bash
# Run settings tests
source ~/.zshrc && deno test --allow-all test/containerized_backend.test.ts --filter "Settings"

# Run sync tests  
source ~/.zshrc && deno test --allow-all test/containerized_backend.test.ts --filter "Sync"

# Run with verbose output
source ~/.zshrc && deno run --allow-all test/runner.ts --verbose
```

## 📋 Test Architecture

### Real Containerized Backend Testing
All integration tests now run against a **real Windmill EE backend** using Docker containers:

- **Real PostgreSQL database** with actual schema and constraints
- **Real Windmill API** with authentication and git-sync features  
- **Real file operations** - sync pull/push creates and deletes actual files
- **Persistent containers** - backend stays running between tests for speed

### Test Structure
- ✅ **`utils.test.ts`** - Unit tests for utility functions (23 tests)
- ✅ **`containerized_backend.test.ts`** - Complete CLI integration tests (24 tests)

## 🏗️ Containerized Backend Architecture

### Backend Management (`containerized_backend.ts`)
- Docker container lifecycle management
- Real Windmill EE with license key integration
- Workspace setup and authentication
- Test data seeding (scripts, flows, apps, resources)
- Persistent backend for faster test execution

### Test Infrastructure
- **Docker Compose**: `test/docker-compose.test.yml` 
- **Backend Utilities**: `test/containerized_backend.ts`
- **Unified Runner**: `test/runner.ts`

## ✅ Features Tested

### Settings Commands (10 tests)
- ✅ Pull workspace settings to new wmill.yaml
- ✅ Pull merges with existing wmill.yaml  
- ✅ Push local settings to backend
- ✅ Push with --from-json override
- ✅ Pull with --diff shows changes
- ✅ Handle malformed wmill.yaml
- ✅ Permissive JSON input handling
- ✅ Authentication failure handling
- ✅ Help information display

### Sync Commands (9 tests)
- ✅ Pull with --settings-from-json override
- ✅ Push with --settings-from-json filters files
- ✅ Dry-run mode shows changes without applying
- ✅ Respects include and exclude patterns
- ✅ Handles type filtering correctly  
- ✅ Authentication failure handling
- ✅ Network error handling
- ✅ Help information display

### Core Backend Operations (5 tests)
- ✅ Backend startup and health checks
- ✅ CLI settings pull with real backend
- ✅ CLI sync pull creates real files
- ✅ CLI sync push with local files
- ✅ Settings update persistence

## 🏃‍♂️ Running Tests

### Test Runner Commands

```bash
# Show help and available test suites
source ~/.zshrc && deno run --allow-all test/runner.ts --help

# Run specific test suite
source ~/.zshrc && deno run --allow-all test/runner.ts --filter Utils
source ~/.zshrc && deno run --allow-all test/runner.ts --filter Containerized

# Run with detailed output for debugging
source ~/.zshrc && deno run --allow-all test/runner.ts --verbose
```

### Direct Test Execution

```bash
# Run specific test files directly
source ~/.zshrc && deno test --allow-all test/utils.test.ts
source ~/.zshrc && deno test --allow-all test/containerized_backend.test.ts

# Run specific test patterns
source ~/.zshrc && deno test --allow-all test/containerized_backend.test.ts --filter "Settings: pull"
source ~/.zshrc && deno test --allow-all test/containerized_backend.test.ts --filter "Sync: push"
```

## 🔧 Test Data and Environment

### Real Backend Operations
Tests use actual Windmill EE backend operations:

```typescript
// Real workspace setup
await backend.createWorkspace('test');
await backend.authenticate();

// Real file sync operations  
await backend.runCLICommand(['sync', 'pull'], tempDir);
await backend.runCLICommand(['sync', 'push'], tempDir);

// Real settings management
await backend.runCLICommand(['settings', 'pull'], tempDir);
await backend.runCLICommand(['settings', 'push'], tempDir);
```

### Test Workspace Structure
```
test workspace:
├── f/          # Scripts and flows
├── g/          # General scripts  
├── u/          # User scripts
├── apps/       # Applications
└── resources/  # Database connections, API keys
```

## 🚀 Performance and Reliability

### Speed Optimizations
- **Persistent containers**: Backend stays running between tests
- **First test**: ~22s (container startup)
- **Subsequent tests**: ~3-5s (reuse existing backend)
- **Health checks**: Fast validation of backend availability

### Reliability Benefits
1. **Real API behavior** instead of maintaining complex mocks
2. **Actual database constraints** catch real validation issues
3. **True file operations** test complete sync workflows
4. **EE features** available for git-sync and enterprise testing
5. **Deterministic** - fresh workspace for each test

## 🐳 Docker Requirements

### Prerequisites
```bash
# Ensure Docker is running
docker --version
docker-compose --version

# Ensure EE license key is available (handled automatically)
echo $EE_LICENSE_KEY
```

### Container Management
The test infrastructure automatically:
- Builds Windmill EE Docker image with proper features
- Starts PostgreSQL and Windmill containers
- Configures workspace and authentication
- Seeds test data
- Cleans up after tests

## 🔍 Debugging Failed Tests

### View Test Output
```bash
# Run with verbose output
source ~/.zshrc && deno run --allow-all test/runner.ts --verbose

# Run specific failing test with details
source ~/.zshrc && deno test --allow-all test/containerized_backend.test.ts --filter "failing test name"
```

### Inspect Backend State
```bash
# Check container status
docker ps

# View backend logs
docker logs test-test_windmill_server-1

# Connect to test database
PGPASSWORD=changeme psql -h localhost -p 5433 -U postgres -d windmill_test
```

### Manual Container Management
```bash
# Start containers manually
cd /home/alex/windmill/windmill/cli/test
docker compose -f docker-compose.test.yml up -d

# Stop containers
docker compose -f docker-compose.test.yml down

# Reset completely
docker compose -f docker-compose.test.yml down -v
```

## 🎯 Key Benefits

1. **🚀 Real Backend Testing**: Tests actual CLI behavior against real Windmill API
2. **⚡ Fast Execution**: Persistent containers minimize startup time
3. **🔧 Maintainable**: No mock server to keep in sync with backend changes
4. **🎪 Comprehensive**: Tests complete workflows including file operations
5. **🛠️ Reliable**: Deterministic test results with fresh workspace per test
6. **🔐 Enterprise Ready**: Tests EE features like git-sync

## 💡 Adding New Tests

When adding new functionality:

1. **For new CLI commands**: Add to `containerized_backend.test.ts`
2. **For utility functions**: Add to `utils.test.ts`
3. **For backend setup**: Extend `containerized_backend.ts`
4. Include both positive and negative test cases
5. Test authentication failures and network errors
6. Verify actual file operations where applicable

### Test Template
```typescript
Deno.test("Feature: specific behavior", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Setup test files/state
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, '...');
    
    // Run CLI command
    const result = await backend.runCLICommand(['command', 'args'], tempDir);
    
    // Verify results
    assertEquals(result.code, 0);
    assertStringIncludes(result.stdout, 'expected output');
    
    // Verify file operations if applicable
    const fileContent = await Deno.readTextFile(`${tempDir}/output.yaml`);
    assertStringIncludes(fileContent, 'expected content');
  });
});
```

## 🔄 CI/CD Integration

### GitHub Actions Example
```yaml
name: CLI Tests

on:
  push:
    branches: [ main, develop ]
    paths: [ 'cli/**' ]
  pull_request:
    branches: [ main ]
    paths: [ 'cli/**' ]

jobs:
  cli-tests:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Deno
      uses: denoland/setup-deno@v1
      with:
        deno-version: v1.40.x
    
    - name: Build Windmill EE Docker Image
      working-directory: ./
      run: docker build -t windmill-test:latest .
    
    - name: Run CLI Tests
      working-directory: ./cli
      run: deno run --allow-all test/runner.ts
      env:
        EE_LICENSE_KEY: ${{ secrets.EE_LICENSE_KEY }}
```

This testing infrastructure provides reliable, maintainable testing with real backend operations while being fast enough for development workflows.