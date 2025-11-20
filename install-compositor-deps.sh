#!/bin/bash
# Install Lamco compositor dependencies

echo "Installing compositor system dependencies..."

sudo apt-get update
sudo apt-get install -y \
    xvfb \
    libxkbcommon-dev \
    libgbm-dev \
    libegl1-mesa-dev \
    libgl1-mesa-dev \
    libwayland-dev

echo "✅ Compositor dependencies installed"
echo ""
echo "To test with Xvfb:"
echo "  Xvfb :99 -screen 0 1920x1080x24 &"
echo "  DISPLAY=:99 ./wrd-server --mode compositor"
