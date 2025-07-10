#!/bin/bash

# Grammar validation script for Sutra Engine
# Can be run manually or as a pre-commit hook

set -e

echo "🔍 Running grammar validation..."

# Change to project root if not already there
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Please run this script from the project root directory"
    exit 1
fi

# Run the grammar validation tool
if cargo run --bin validate_grammar --quiet; then
    echo "✅ Grammar validation passed"
    exit 0
else
    echo "❌ Grammar validation failed"
    echo ""
    echo "Please fix the grammar issues before committing."
    echo "Run 'cargo run --bin validate_grammar' for detailed output."
    exit 1
fi