# vaultwrap
.env injection during runtime


vault-system/
├── vaultd/             # Master secrets server
│   ├── src/
│   ├── secrets.toml    # Encrypted file (or load from Azure)
│   └── config.toml     # Allowed clients & commands
├── vaultwrap/          # Client CLI
│   ├── src/
│   ├── Cargo.toml
│   └── README.md
└── shared/             # Shared structs, encryption logic
    └── src/
