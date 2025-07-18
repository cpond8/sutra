#!/bin/bash

# Sutra Grammar Validation Script
# Validates the grammar.pest file using the built-in Rust validation system

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Check if we're in the right directory (should have Cargo.toml)
if [ ! -f "Cargo.toml" ]; then
    print_status $RED "Error: This script must be run from the sutra project root directory"
    exit 1
fi

# Check if grammar.pest exists
if [ ! -f "src/syntax/grammar.pest" ]; then
    print_status $RED "Error: Grammar file not found at src/syntax/grammar.pest"
    exit 1
fi

print_status $GREEN "🔍 Validating grammar.pest..."

# Build the project first to ensure we have the latest binary
print_status $YELLOW "Building project..."
if ! cargo build --quiet; then
    print_status $RED "Error: Failed to build project"
    exit 1
fi

# Run the grammar validation using the CLI
print_status $YELLOW "Running grammar validation..."
if cargo run --quiet -- validate-grammar; then
    print_status $GREEN "✅ Grammar validation passed"
    exit 0
else
    print_status $RED "❌ Grammar validation failed"
    exit 1
fi