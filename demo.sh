#!/bin/bash

echo "=== VaultWrap Demo: Python venv-like Environment Variable Management ==="
echo ""

echo "1. Current shell state:"
echo "   PS1: $PS1"
echo "   DATABASE_URL: ${DATABASE_URL:-<not set>}"
echo "   API_KEY: ${API_KEY:-<not set>}"
echo ""

echo "2. Normal Mode - Activating 'Sandbox' environment (like 'source venv/bin/activate'):"
echo "   $ vaultwrap disable  # Ensure runtime injection is off"
echo "   $ eval \"\$(vaultwrap set Sandbox)\""
echo ""

# Note: In a real demo, you'd run this, but for the script we'll just show the output
cd vaultwrap
echo "   Output:"
cargo run -- disable 2>/dev/null
cargo run -- set Sandbox 2>/dev/null | sed 's/^/   /'
echo ""

echo "3. After activation, your shell would have:"
echo "   PS1: (Sandbox) $PS1"
echo "   DATABASE_URL: postgres://user:pass@localhost:5432/sandbox"
echo "   API_KEY: test-api-key-12345"
echo ""

echo "4. Runtime Injection Mode - More secure, variables only during command execution:"
echo "   $ vaultwrap add cargo        # Add commands to inject into"
echo "   $ vaultwrap add python3"
echo "   $ vaultwrap enable           # Enable runtime injection"
echo "   $ eval \"\$(vaultwrap set Sandbox)\""
echo ""
echo "   Output:"
cargo run -- add cargo 2>/dev/null | sed 's/^/   /'
cargo run -- add python3 2>/dev/null | sed 's/^/   /'
cargo run -- enable 2>/dev/null | sed 's/^/   /'
cargo run -- set Sandbox 2>/dev/null | sed 's/^/   /'
echo ""

echo "5. In runtime injection mode:"
echo "   PS1: (Sandbox) $PS1          # Prompt shows environment"
echo "   DATABASE_URL: <not set>       # Variables NOT in shell"
echo "   cargo run: gets variables     # Only configured commands get vars"
echo "   python3: gets variables       # Only configured commands get vars"
echo ""

echo "6. Deactivating environment (like 'deactivate'):"
echo "   $ eval \"\$(vaultwrap drop)\""
echo ""
echo "   Output:"
cargo run -- drop 2>/dev/null | sed 's/^/   /'
echo ""

echo "7. After deactivation, your shell returns to normal:"
echo "   PS1: $PS1"
echo "   DATABASE_URL: <not set>"
echo "   API_KEY: <not set>"
echo ""

echo "=== Runtime Injection Management ==="
echo ""

echo "• Check status:"
echo "  $ vaultwrap status"
cargo run -- status 2>/dev/null | sed 's/^/  /'
echo ""

echo "• Manage injection commands:"
echo "  $ vaultwrap add node          # Add command"
echo "  $ vaultwrap remove cargo      # Remove command"
echo "  $ vaultwrap disable           # Disable injection"
echo ""

echo "=== Environment Management ==="
echo ""

echo "• List environments:"
echo "  $ vaultwrap env list"
cargo run -- env list 2>/dev/null | grep -v "^$" | sed 's/^/  /'
echo ""

echo "• Connection management:"
echo "  $ vaultwrap connect prod.company.com:4000 --save prod"
echo "  $ vaultwrap connections list"
echo ""

echo "=== Installation ==="
echo "To install globally:"
echo "  $ cd vaultwrap && cargo install --path ."
echo "  $ eval \"\$(vaultwrap set dev)\""
echo "  $ eval \"\$(vaultwrap drop)\""
echo "  $ vaultwrap add cargo && vaultwrap enable" 