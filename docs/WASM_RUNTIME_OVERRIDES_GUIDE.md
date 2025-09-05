# WASM Runtime Overrides Guide

This guide explains how to use the `--wasm-runtime-overrides` option in Subtensor to run custom WASM runtimes locally.

## Overview

The `--wasm-runtime-overrides` option allows you to override on-chain runtimes with local WASM runtime files when the version numbers match. This is particularly useful for:

- Testing runtime upgrades locally before deployment
- Running custom runtime modifications for development
- Debugging runtime issues with modified code
- Testing runtime changes against live chain state

## How It Works

When you specify a directory with `--wasm-runtime-overrides <PATH>`, the node will:

1. Look for WASM runtime files in the specified directory
2. Compare the runtime version of local WASM files with the on-chain runtime version
3. If versions match, use the local WASM runtime instead of the on-chain version
4. If no matching version is found locally, fall back to the on-chain runtime

## Usage

### Basic Syntax

```bash
./target/release/node-subtensor --wasm-runtime-overrides <PATH_TO_WASM_DIRECTORY> [other options]
```

### Example Usage

1. **Create a directory for your WASM overrides:**
   ```bash
   mkdir -p ./wasm-overrides
   ```

2. **Build your custom runtime:**
   ```bash
   cargo build --release -p node-subtensor-runtime
   ```

3. **Copy the WASM runtime to your overrides directory:**
   ```bash
   cp ./target/release/wbuild/node-subtensor-runtime/node_subtensor_runtime.compact.wasm ./wasm-overrides/
   ```

4. **Run the node with runtime overrides:**
   ```bash
   ./target/release/node-subtensor \
     --wasm-runtime-overrides ./wasm-overrides \
     --chain local \
     --alice \
     --tmp
   ```

### Advanced Example: Testing Against Live Chain

```bash
# Run against testnet with custom runtime
./target/release/node-subtensor \
  --wasm-runtime-overrides ./wasm-overrides \
  --chain test_finney \
  --sync warp \
  --tmp
```

## Runtime Version Matching

The runtime override system matches based on the runtime version defined in your runtime's `lib.rs`:

```rust
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: Cow::Borrowed("custom-subtensor"),
    impl_name: Cow::Borrowed("custom-subtensor"),
    authoring_version: 1,
    spec_version: 123,  // This must match the on-chain version
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};
```

**Important:** The `spec_version` in your local WASM must match the on-chain `spec_version` for the override to take effect.

## File Naming Convention

The WASM files in your overrides directory should follow this naming pattern:
- `node_subtensor_runtime.compact.wasm` (standard build output)
- Or any `.wasm` file that contains the correct runtime version

## Verification

To verify that your runtime override is being used:

1. **Check the logs:** Look for messages indicating runtime override usage
2. **Monitor runtime version:** Use RPC calls to verify the runtime version being used
3. **Test custom functionality:** If you've added custom features, test them to confirm your runtime is active

## Common Use Cases

### 1. Testing Runtime Upgrades

```bash
# Build runtime with new version
cargo build --release -p node-subtensor-runtime

# Copy to overrides
cp ./target/release/wbuild/node-subtensor-runtime/node_subtensor_runtime.compact.wasm ./wasm-overrides/

# Test against live chain state
./target/release/node-subtensor \
  --wasm-runtime-overrides ./wasm-overrides \
  --chain finney \
  --sync warp
```

### 2. Development with Custom Pallets

```bash
# After modifying pallets, rebuild runtime
cargo build --release -p node-subtensor-runtime

# Update override
cp ./target/release/wbuild/node-subtensor-runtime/node_subtensor_runtime.compact.wasm ./wasm-overrides/

# Run local development node
./target/release/node-subtensor \
  --wasm-runtime-overrides ./wasm-overrides \
  --chain local \
  --alice \
  --tmp
```

### 3. Debugging Runtime Issues

```bash
# Build runtime with debug info or additional logging
RUST_LOG=debug cargo build --release -p node-subtensor-runtime

# Use override for debugging
./target/release/node-subtensor \
  --wasm-runtime-overrides ./wasm-overrides \
  --chain local \
  -lruntime=debug
```

## Important Notes

1. **Version Matching:** Runtime overrides only work when versions match exactly
2. **Performance:** Local WASM files may have different performance characteristics
3. **Security:** Only use trusted WASM files for overrides
4. **State Compatibility:** Ensure your runtime changes are compatible with existing chain state
5. **Testing:** Always test thoroughly in a development environment before using on live networks

## Troubleshooting

### Runtime Override Not Working
- Check that the runtime version matches exactly
- Verify the WASM file is in the correct directory
- Ensure the file is named correctly
- Check node logs for override confirmation messages

### Peer Connection Issues
- **Problem:** Node stuck "Waiting for peers to be connected"
- **Solution:** Ensure bootnodes are specified for network discovery
- **Finney mainnet:** Use `--bootnodes /dns/bootnode.finney.chain.opentensor.ai/tcp/30333/ws/p2p/12D3KooWRwbMb85RWnT8DSXSYMWQtuDwh4LJzndoRrTDotTR5gDC`
- **Test network:** Use `--bootnodes /dns/bootnode.test.finney.opentensor.ai/tcp/30333/ws/p2p/12D3KooWPM4mLcKJGtyVtkggqdG84zWrd7Rij6PGQDoijh1X86Vr`

### Build Failures
- Ensure all dependencies are installed
- Check that the runtime package name is correct
- Verify Rust toolchain compatibility

### Performance Issues
- Monitor resource usage when using overrides
- Consider the impact of debug builds vs release builds
- Test with different node configurations

## Conclusion

The `--wasm-runtime-overrides` option is a powerful tool for Subtensor development and testing. It allows you to safely test runtime changes against live chain state without affecting the actual network, making it invaluable for development, debugging, and upgrade testing workflows.