#!/bin/bash

# Install git hooks for Sutra Engine development
# This sets up automatic grammar validation on commits

set -e

echo "🔧 Installing git hooks for Sutra Engine..."

# Ensure we're in the project root
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Please run this script from the project root directory"
    exit 1
fi

# Create .git/hooks directory if it doesn't exist
mkdir -p .git/hooks

# Create pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash

# Sutra Engine pre-commit hook
# Validates grammar.pest before allowing commits

echo "🔍 Pre-commit: Running grammar validation..."

# Check if grammar.pest was modified
if git diff --cached --name-only | grep -q "src/syntax/grammar.pest"; then
    echo "📝 Grammar file modified, running validation..."

    if ! ./scripts/check_grammar.sh; then
        echo "❌ Commit blocked: Grammar validation failed"
        echo "   Fix the grammar issues and try committing again."
        exit 1
    fi
else
    echo "📋 Grammar file not modified, skipping validation"
fi

echo "✅ Pre-commit checks passed"
EOF

# Make the hook executable
chmod +x .git/hooks/pre-commit

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  • pre-commit: Grammar validation (when grammar.pest is modified)"
echo ""
echo "You can also run grammar validation manually:"
echo "  ./scripts/check_grammar.sh"
echo "  cargo run --bin validate_grammar"