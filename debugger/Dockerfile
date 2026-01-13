# Dockerfile for Windmill DAP Debug Service
#
# This containerizes the unified debug service with support for:
# - TypeScript/Bun debugging
# - Python debugging
# - Automatic dependency installation via windmill CLI
# - Optional nsjail sandboxing
#
# Build:
#   docker build -t windmill-debugger .
#
# Run:
#   docker run -p 5679:5679 windmill-debugger
#
# With nsjail enabled:
#   docker run -p 5679:5679 --privileged windmill-debugger --nsjail

# Stage 1: Get nsjail and windmill from the official windmill image
FROM ghcr.io/windmill-labs/windmill-ee-nsjail:main AS windmill-source

# Stage 2: Build the debug service
FROM oven/bun:1 AS runtime

# Install Python and required system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-venv \
    libprotobuf-dev \
    libnl-route-3-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies for the debug server
RUN pip3 install --break-system-packages websockets debugpy

# Copy nsjail binary and its dependencies from windmill image
COPY --from=windmill-source /bin/nsjail /bin/nsjail
COPY --from=windmill-source /etc/nsjail/ /etc/nsjail/

# Copy windmill binary for prepare-deps functionality
COPY --from=windmill-source /usr/src/app/windmill /usr/local/bin/windmill

# Create app directory
WORKDIR /app

# Copy the debug service files
COPY dap_debug_service.ts .
COPY dap_websocket_server_bun.ts .
COPY dap_websocket_server.py .

# Expose the default port
EXPOSE 5679

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:5679/health || exit 1

# Default environment variables
ENV HOST=0.0.0.0
ENV PORT=5679

# Run the unified debug service with windmill path for autoinstall
ENTRYPOINT ["bun", "run", "dap_debug_service.ts", "--windmill", "/usr/local/bin/windmill"]
CMD ["--host", "0.0.0.0", "--port", "5679"]
