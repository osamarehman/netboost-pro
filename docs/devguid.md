# NetBoost Pro - Development Setup Guide

## Prerequisites

### System Requirements
- **Windows 11** (primary target platform)
- **Node.js 18+** for frontend development
- **Rust 1.70+** for backend development
- **Visual Studio 2022** with C++ tools (for Windows development)
- **Administrator privileges** (required for TUN/TAP interface creation)

### Required Tools

1. **Install Rust**
   ```bash
   # Download and install from https://rustup.rs/
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Add to PATH and reload
   source ~/.cargo/env
   ```

2. **Install Node.js**
   ```bash
   # Download from https://nodejs.org/ or use package manager
   # Verify installation
   node --version
   npm --version
   ```

3. **Install Tauri CLI**
   ```bash
   cargo install tauri-cli
   ```

4. **Install Visual Studio Build Tools**
   - Download Visual Studio 2022 Community
   - Select "Desktop development with C++" workload
   - Include Windows 11 SDK

## Project Setup

### 1. Clone and Initialize

```bash
git clone <your-repo-url>
cd netboost-pro

# Install frontend dependencies
npm install

# Install additional React dependencies
npm install react react-dom @types/react @types/react-dom
npm install -D @vitejs/plugin-react tailwindcss autoprefixer postcss
```

### 2. Configure Development Environment

```bash
# Create necessary directories
mkdir -p src/components src/utils src/types

# Set environment variables (optional)
export RUST_LOG=debug
export TAURI_DEBUG=true
```

### 3. Build and Run

```bash
# Development mode (hot reload)
npm run tauri dev

# Or build for production
npm run tauri build
```

## File Structure Overview

```
netboost-pro/
├── src/                          # Frontend source
│   ├── components/              
│   │   └── NetBoostDashboard.tsx
│   ├── main.tsx                 # React entry point
│   └── styles.css              # Global styles
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── bin/
│   │   │   └── cli.rs          # CLI interface
│   │   ├── interface_manager.rs # Network interface discovery
│   │   ├── packet_router.rs    # Intelligent packet routing
│   │   ├── performance_monitor.rs # Performance tracking
│   │   ├── virtual_adapter.rs  # TUN interface management
│   │   ├── lib.rs             # Library entry + Tauri commands
│   │   └── main.rs            # GUI entry point
│   ├── Cargo.toml             # Rust dependencies
│   ├── tauri.conf.json        # Tauri configuration
│   └── build.rs               # Build script
├── package.json               # Node.js dependencies
├── vite.config.ts            # Vite configuration
├── tailwind.config.js        # Tailwind CSS config
├── postcss.config.js         # PostCSS config
└── tsconfig.json             # TypeScript config
```

## Development Workflow

### 1. CLI Development & Testing

```bash
# Build CLI binary
cargo build --bin netboost-cli

# Test network interface discovery
./target/debug/netboost-cli --discover

# Test service start (requires admin privileges)
sudo ./target/debug/netboost-cli --start
```

### 2. GUI Development

```bash
# Start development server with hot reload
npm run tauri dev

# The GUI will automatically reload when you make changes to:
# - Frontend code (src/)
# - Rust code (src-tauri/src/)
```

### 3. Testing Different Scenarios

```bash
# Test with different load balancing modes
# Use the GUI dropdown or CLI commands

# Monitor performance
# Check the performance stats in the GUI dashboard

# Test interface switching
# Disable/enable network adapters while running
```

## Troubleshooting

### Common Issues

1. **TUN Interface Creation Fails**
   ```
   Error: Failed to create TUN interface
   ```
   - **Solution**: Run with administrator privileges
   - **Alternative**: Use Windows Subsystem for Linux (WSL)

2. **Build Fails on Windows**
   ```
   Error: linking with `link.exe` failed
   ```
   - **Solution**: Install Visual Studio Build Tools
   - **Check**: Ensure C++ tools are selected during installation

3. **Frontend Bundle Issues**
   ```
   Error: Could not resolve "@tauri-apps/api"
   ```
   - **Solution**: Reinstall dependencies
   ```bash
   rm -rf node_modules package-lock.json
   npm install
   ```

4. **Packet Routing Not Working**
   ```
   Error: Permission denied
   ```
   - **Solution**: Ensure admin privileges for network operations
   - **Check**: Windows Firewall settings

### Debug Mode

Enable detailed logging:

```bash
# Set environment variables
set RUST_LOG=netboost_pro=debug
set TAURI_DEBUG=true

# Run with verbose output
npm run tauri dev
```

## Current Implementation Status

### ✅ Completed (Phase 1 & 2)
- [x] Basic TUN interface creation
- [x] Network interface discovery
- [x] Packet router with load balancing algorithms
- [x] Performance monitoring system
- [x] React dashboard with real-time stats
- [x] Tauri integration and commands
- [x] CLI interface for testing

### 🚧 In Progress (Phase 2 Completion)
- [ ] Actual packet forwarding to physical interfaces
- [ ] Real network performance measurement
- [ ] Interface health monitoring
- [ ] Configuration persistence

### 📋 Next Steps (Phase 3: System Integration)

1. **Raw Socket Implementation**
   ```rust
   // Implement actual packet sending via pnet_datalink
   // Add Windows WinPcap/Npcap integration
   // Handle different interface types (Ethernet, WiFi, cellular)
   ```

2. **Windows Service Integration**
   ```rust
   // Implement Windows service wrapper
   // Add registry configuration
   // Handle service start/stop/restart
   ```

3. **Advanced Routing Features**
   ```rust
   // Add application-specific routing rules
   // Implement QoS prioritization
   // Add custom routing table management
   ```

4. **Security Implementation**
   ```rust
   // Add proper privilege separation
   // Implement secure IPC
   // Add Windows Firewall integration
   ```

## Performance Optimization

### Rust Backend
- Use `--release` builds for production
- Enable link-time optimization (LTO)
- Profile with `cargo flamegraph`

### Frontend
- Implement virtualization for large data sets
- Use React.memo for expensive components
- Optimize re-render cycles

## Deployment Preparation

### Code Signing
```bash
# Generate signing certificate
# Configure Tauri for code signing
# Set up automated signing pipeline
```

### Installer Creation
```bash
# Build installer
npm run tauri build

# Output will be in src-tauri/target/release/bundle/
```

### Testing Checklist
- [ ] Admin privilege handling
- [ ] Multiple network adapter scenarios
- [ ] High network load conditions
- [ ] Interface failure/recovery
- [ ] System restart persistence
- [ ] Antivirus compatibility

## Contributing

1. Follow the established code structure
2. Add tests for new functionality
3. Update documentation for API changes
4. Test on different Windows versions
5. Ensure compatibility with various network adapters

## Next Development Phase

The next major milestone is implementing **actual packet forwarding** and **Windows service integration**. This involves:

1. **Raw packet handling** using Windows APIs
2. **Service wrapper** for system integration
3. **Production-ready error handling**
4. **Comprehensive testing** across different network scenarios

Continue to **Phase 3: System Integration** when ready to move beyond the current proof-of-concept stage.