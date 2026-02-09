#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🎛️  Windmill Ephemeral Backend Manager - Installation Script"
echo "=========================================================="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}❌ This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Get the actual user who ran sudo (if applicable)
ACTUAL_USER="${SUDO_USER:-$(whoami)}"

# Configuration
SERVICE_USER="sandbox"
SERVICE_NAME="windmill-ephemeral-manager"
REPO_DIR="/home/$SERVICE_USER/ephemeral-backend/windmill"
ENV_DIR="/etc/$SERVICE_NAME"
ENV_FILE="$ENV_DIR/.env"

echo "📋 Configuration:"
echo "  Service user: $SERVICE_USER"
echo "  Repository directory: $REPO_DIR"
echo "  Environment file: $ENV_FILE"
echo ""

# Check if sandbox user exists
if ! id "$SERVICE_USER" &>/dev/null; then
    echo -e "${RED}❌ User '$SERVICE_USER' does not exist${NC}"
    echo "   Create the user first: sudo adduser --system --group $SERVICE_USER"
    exit 1
fi

# Check if repository directory exists
if [ ! -d "$REPO_DIR" ]; then
    echo -e "${RED}❌ Repository directory does not exist: $REPO_DIR${NC}"
    echo "   Clone the repository first as user $SERVICE_USER"
    exit 1
fi

# Check if bun is installed for the service user
if ! sudo -u "$SERVICE_USER" bash -c "command -v bun" &>/dev/null; then
    echo -e "${RED}❌ Bun is not installed for user $SERVICE_USER${NC}"
    echo "   Install bun: curl -fsSL https://bun.sh/install | bash"
    exit 1
fi

# Check if bubblewrap is installed
if ! command -v bwrap &>/dev/null; then
    echo -e "${YELLOW}⚠️  bubblewrap is not installed${NC}"
    echo "   Installing bubblewrap..."
    apt-get update -qq
    apt-get install -y -qq bubblewrap
    echo -e "${GREEN}✓ bubblewrap installed${NC}"
fi

# Check if cloudflared is installed
if ! command -v cloudflared &>/dev/null; then
    echo -e "${YELLOW}⚠️  cloudflared is not installed${NC}"
    echo "   Installing cloudflared..."

    # Detect architecture
    ARCH=$(uname -m)
    if [ "$ARCH" = "x86_64" ]; then
        wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
        dpkg -i cloudflared-linux-amd64.deb
        rm cloudflared-linux-amd64.deb
    elif [ "$ARCH" = "aarch64" ]; then
        wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-arm64.deb
        dpkg -i cloudflared-linux-arm64.deb
        rm cloudflared-linux-arm64.deb
    else
        echo -e "${RED}❌ Unsupported architecture: $ARCH${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ cloudflared installed${NC}"
fi

# Check if docker is installed and service user has access
if ! command -v docker &>/dev/null; then
    echo -e "${RED}❌ Docker is not installed${NC}"
    echo "   Install docker: https://docs.docker.com/engine/install/"
    exit 1
fi

if ! sudo -u "$SERVICE_USER" docker ps &>/dev/null; then
    echo -e "${YELLOW}⚠️  User $SERVICE_USER cannot access Docker${NC}"
    echo "   Adding $SERVICE_USER to docker group..."
    usermod -aG docker "$SERVICE_USER"
    echo -e "${GREEN}✓ User added to docker group (may require logout/login)${NC}"
fi

# Create environment directory
echo ""
echo "📁 Setting up environment..."
mkdir -p "$ENV_DIR"
chmod 755 "$ENV_DIR"

# Copy or create environment file
if [ ! -f "$ENV_FILE" ]; then
    if [ -f "$REPO_DIR/ephemeral-backends/.env.template" ]; then
        cp "$REPO_DIR/ephemeral-backends/.env.template" "$ENV_FILE"
        echo -e "${YELLOW}⚠️  Environment file created from template: $ENV_FILE${NC}"
        echo "   ${RED}IMPORTANT: Edit this file and fill in the required values!${NC}"
    else
        echo -e "${RED}❌ Template file not found: $REPO_DIR/ephemeral-backends/.env.template${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✓ Environment file already exists: $ENV_FILE${NC}"
fi

chmod 600 "$ENV_FILE"

# Install systemd service
echo ""
echo "🔧 Installing systemd service..."
cp "$REPO_DIR/ephemeral-backends/windmill-ephemeral-manager.service" "/etc/systemd/system/$SERVICE_NAME.service"
chmod 644 "/etc/systemd/system/$SERVICE_NAME.service"
echo -e "${GREEN}✓ Service file installed${NC}"

# Reload systemd
echo ""
echo "🔄 Reloading systemd..."
systemctl daemon-reload
echo -e "${GREEN}✓ Systemd reloaded${NC}"

# Enable service
echo ""
echo "✅ Enabling service..."
systemctl enable "$SERVICE_NAME.service"
echo -e "${GREEN}✓ Service enabled (will start on boot)${NC}"

echo ""
echo "=========================================================="
echo -e "${GREEN}✅ Installation complete!${NC}"
echo ""
echo "📝 Next steps:"
echo "   1. Edit the environment file: sudo nano $ENV_FILE"
echo "   2. Fill in the required values:"
echo "      - MANAGER_AUTH_TOKEN (generate with: openssl rand -hex 32)"
echo "      - GITHUB_TOKEN (GitHub personal access token)"
echo "      - GIT_EE_DEPLOY_KEY_FILE (path to SSH deploy key)"
echo "   3. Ensure the SSH deploy key exists and is readable by $SERVICE_USER"
echo "   4. Add MANAGER_AUTH_TOKEN to GitHub repository secrets"
echo "   5. Start the service: sudo systemctl start $SERVICE_NAME"
echo "   6. Check status: sudo systemctl status $SERVICE_NAME"
echo "   7. View logs: sudo journalctl -u $SERVICE_NAME -f"
echo ""
echo "🔍 Useful commands:"
echo "   sudo systemctl start $SERVICE_NAME     # Start the service"
echo "   sudo systemctl stop $SERVICE_NAME      # Stop the service"
echo "   sudo systemctl restart $SERVICE_NAME   # Restart the service"
echo "   sudo systemctl status $SERVICE_NAME    # Check service status"
echo "   sudo journalctl -u $SERVICE_NAME -f   # Follow logs"
echo "   sudo journalctl -u $SERVICE_NAME -n 100  # Last 100 log lines"
echo ""
