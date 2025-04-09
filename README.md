# VaultWrap 🔒

A secure, memory-hardened environment variable injection system for development teams. VaultWrap enables secure sharing of sensitive environment variables between team members while maintaining strict access control and memory safety.

## 🌟 Features

- **Just-In-Time Secret Injection**: Environment variables are injected only when needed and cleared immediately after use
- **Memory-Hardened**: Secrets never touch disk and are protected from memory inspection
- **Command-Bound Access**: Secrets are only available to specific approved commands
- **Centralized Management**: Single source of truth for environment variables
- **Modern AI Integration**: Works seamlessly with AI coding assistants and IDEs
- **Cross-Platform**: Works on macOS, Linux, and Windows

## 🏗️ Architecture

```
vault-system/
├── vaultd/             # Master secrets server
│   ├── src/           # Server implementation
│   ├── secrets.toml   # Encrypted secrets storage
│   └── config.toml    # Client permissions & commands
├── vaultwrap/         # Client CLI
│   ├── src/          # Client implementation
│   └── Cargo.toml    # Rust dependencies
└── shared/           # Shared components
    └── src/         # Common utilities
```

## 🚀 Quick Start

1. **Start the Master Server**:
```bash
cd vaultd
cargo run
```

2. **Run a Command with Secrets**:
```bash
cd vaultwrap
cargo run -- run python3 app.py
```

## 🔒 Security Features

- **Memory Protection**: Secrets are stored in protected memory regions
- **Tamper Detection**: Monitors for unauthorized memory access attempts
- **Process Isolation**: Secrets are only injected into approved processes
- **Automatic Cleanup**: Secrets are wiped when processes terminate
- **Encrypted Transport**: All communication is secured via TLS

## 💡 Use Cases

- **Team Development**: Securely share environment variables with team members
- **CI/CD Integration**: Safely inject secrets during build processes
- **Local Development**: Keep sensitive credentials off developer machines
- **AI Assistant Integration**: Enable AI tools to work with environment variables safely

## 🛠️ Technical Details

- Built in Rust for memory safety and performance
- Uses modern encryption standards (AES-256)
- Implements secure process spawning and monitoring
- Supports both local and LAN-based deployments

## 📚 Documentation

For detailed documentation, please refer to the [docs](./docs) directory.

## 🤝 Contributing

Contributions are welcome! Please see our [contributing guidelines](./CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
