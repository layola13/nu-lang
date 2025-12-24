#!/bin/bash
# Setup Git hooks for Nu Language Compiler
# This script installs the pre-push hook

set -e

echo "🔧 Setting up Git hooks..."

# Get the git hooks directory
HOOKS_DIR=".git/hooks"

# Check if .git directory exists
if [ ! -d ".git" ]; then
    echo "❌ Not a git repository. Please run this from the project root."
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Copy pre-push hook
echo "📋 Installing pre-push hook..."
cp .githooks/pre-push "$HOOKS_DIR/pre-push"
chmod +x "$HOOKS_DIR/pre-push"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The pre-push hook will now:"
echo "  1. Check code formatting (cargo fmt)"
echo "  2. Run clippy linter"
echo "  3. Run all tests"
echo ""
echo "To bypass the hook (not recommended), use:"
echo "  git push --no-verify"