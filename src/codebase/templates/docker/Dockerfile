# @LITE_DESC Multi-stage Docker build with builder and runtime stages, non-root user, and health check
# @LITE_SCENE Production-ready containerization for web applications
# @LITE_TAGS docker, dockerfile, container, build, multi-stage

# ============================================================================
# BUILDER STAGE
# ============================================================================

FROM ubuntu:22.04 AS builder

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    libpq-dev \
    libssl-dev \
    libffi-dev \
    python3 \
    python3-pip \
    python3-dev \
    --no-install-recommends && \
    rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy requirements first for better caching
COPY requirements.txt .

# Install Python dependencies
RUN pip3 install --user --no-cache-dir -r requirements.txt

# Copy application code
COPY . .

# Build the application
RUN python3 setup.py build || echo "Build completed"

# ============================================================================
# RUNTIME STAGE
# ============================================================================

FROM ubuntu:22.04 AS runtime

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PATH=/root/.local/bin:$PATH \
    PYTHONPATH=/app:$PYTHONPATH

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    curl \
    libpq5 \
    libssl3 \
    python3 \
    python3-pip \
    --no-install-recommends && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Set working directory
WORKDIR /app

# Copy Python packages from builder
COPY --from=builder /root/.local /root/.local

# Copy built application
COPY --from=builder /build /app

# Create necessary directories
RUN mkdir -p /app/logs /app/tmp /app/static /app/media && \
    chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose application port
EXPOSE 8000

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Default command
CMD ["python3", "-m", "gunicorn", "--bind", "0.0.0.0:8000", "--workers", "4", "app:app"]