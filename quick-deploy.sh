#!/bin/bash
# One-command deploy and run on KDE VM
# Usage: ./quick-deploy.sh

set -e

KDE_VM="192.168.10.205"

echo "Quick Deploy to KDE VM..."
echo ""

# Build and deploy
./test-kde.sh deploy

echo ""
echo "========================================="
echo "Deployment complete!"
echo ""
echo "Connecting to KDE VM and starting server..."
echo "========================================="
echo ""

# SSH and run
ssh -t greg@${KDE_VM} "cd lamco-rdp-server-test && ./run-server.sh"
