# WASM Runtime Override Example

This document provides a quick example of how to use the `--wasm-runtime-overrides` feature in Subtensor.

## Quick Start

### 1. Build the Runtime

```bash
# Build the runtime package
cargo build --release -p node-subtensor-runtime
```

### 2. Create Override Directory

```bash
# Create directory for WASM overrides
mkdir -p ./wasm-overrides

# Copy the built WASM runtime
cp ./target/release/wbuild/node-subtensor-runtime/node_subtensor_runtime.compact.wasm ./wasm-overrides/
```

### 3. Run with Override

```bash
# Run local development node with WASM override
./target/release/node-subtensor \
  --wasm-runtime-overrides ./wasm-overrides \
  --chain local \
  --alice \
  --tmp
```

## Using the Helper Script

We've provided a convenient script that automates this process:

```bash
# Make script executable (if not already)
chmod +x ./scripts/run_with_wasm_override.sh

# Run local development node with automatic build and override setup
./scripts/run_with_wasm_override.sh local

# Or just build and setup without running
./scripts/run_with_wasm_override.sh --build-only local
```

## Verification

To verify that your WASM override is working:

1. **Check the node logs** for any runtime-related messages
2. **Test custom functionality** if you've modified the runtime
3. **Monitor performance** to ensure the override is functioning correctly

```bash
# Check if the WASM file exists
ls -la ./wasm-overrides/node_subtensor_runtime.compact.wasm

# Check file size
du -h ./wasm-overrides/node_subtensor_runtime.compact.wasm
```

## Important Notes

- **Version Matching**: The runtime override only works when the local WASM runtime version matches the on-chain version
- **Development Use**: This feature is primarily intended for development and testing
- **State Compatibility**: Ensure your runtime changes are compatible with existing chain state

For more detailed information, see the [WASM Runtime Overrides Guide](./WASM_RUNTIME_OVERRIDES_GUIDE.md).