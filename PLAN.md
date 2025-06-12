# PLAN.md

## Project Goals

- **Desktop GUI:** Build a cross-platform desktop application for managing and importing environment configurations.
- **Background Service:** Allow the server (vaultd) to run as a background process/daemon, locally or on a remote host (e.g., Azure).
- **Flexible Client:** Enable the client (vaultwrap) to connect to either a local or remote server, callable from anywhere (add to PATH).
- **Networked Mode:** Retain and enhance the ability to fetch environment secrets over the network, supporting secure remote access.

---

## Architecture Overview

- **vaultd (Server/Daemon):**
  - Runs as a background service.
  - Can be started locally or deployed to a remote server/cloud.
  - Exposes a secure API (REST/gRPC) for environment management.

- **vaultwrap (Client):**
  - CLI tool callable from anywhere (installable via PATH).
  - Communicates with vaultd (local or remote).
  - Optionally, a GUI frontend (see below).

- **Desktop GUI:**
  - Cross-platform (consider Tauri, Electron, or native frameworks).
  - Allows users to manage, import, and export environments.
  - Connects to vaultd via API.

- **Shared Library:**
  - Common logic for environment management, serialization, etc.

---

## Implementation Steps

### A. Refactor for Networked Operation
- Define a clear API for vaultd (REST/gRPC).
- Update vaultwrap to support both local and remote server addresses.
- Add authentication and encryption for remote access.

### B. Background Service Support
- Provide scripts/instructions for running vaultd as a background service (systemd, launchd, Windows service).
- Add CLI commands to start/stop/status the service.

### C. CLI Installation
- Package vaultwrap as a binary and provide install scripts (e.g., Homebrew, shell script, or cargo install).
- Ensure it can be called from anywhere via PATH.

### D. Desktop GUI
- Choose a framework (Tauri recommended for Rust).
- Build a GUI that interacts with the vaultd API.
- Support importing/exporting environment configs.

### E. Cloud/Remote Hosting
- Document how to deploy vaultd to Azure (or other cloud providers).
- Add configuration for remote server URLs and credentials.

### F. Backward Compatibility
- Ensure local-only mode still works for users who don't want networked features.

---

## Security Considerations

- Use TLS for all network communication.
- Implement authentication (API keys, OAuth, etc.).
- Encrypt secrets at rest and in transit.

---

## User Experience

- Simple onboarding: local mode by default, with easy upgrade to networked/cloud mode.
- Intuitive GUI for non-CLI users.
- Clear documentation for all modes of operation.

---

## Next Steps

1. Define the vaultd API (REST/gRPC spec).
2. Refactor vaultwrap to use the API.
3. Prototype the GUI (Tauri/Electron).
4. Add service management scripts.
5. Write deployment and installation docs. 