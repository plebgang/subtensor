#!/bin/bash

# Subtensor WASM Runtime Override Example Script
# This script demonstrates how to build and run Subtensor with custom WASM runtime overrides

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WASM_OVERRIDE_DIR="./wasm-overrides"
NODE_BINARY="./target/release/node-subtensor"
RUNTIME_PACKAGE="node-subtensor-runtime"

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if binary exists
check_binary() {
    if [ ! -f "$NODE_BINARY" ]; then
        print_error "Node binary not found at $NODE_BINARY"
        print_info "Please build the node first: cargo build --release"
        exit 1
    fi
}

# Function to build runtime
build_runtime() {
    print_info "Building runtime package: $RUNTIME_PACKAGE"
    cargo build --release -p "$RUNTIME_PACKAGE" --features metadata-hash
    
    if [ $? -eq 0 ]; then
        print_success "Runtime built successfully"
    else
        print_error "Failed to build runtime"
        exit 1
    fi
}

# Function to setup WASM override directory
setup_wasm_override() {
    print_info "Setting up WASM override directory: $WASM_OVERRIDE_DIR"
    
    # Create override directory
    mkdir -p "$WASM_OVERRIDE_DIR"
    
    # Find and copy WASM runtime file
    WASM_FILE="./target/release/wbuild/$RUNTIME_PACKAGE/node_subtensor_runtime.compact.wasm"
    
    if [ -f "$WASM_FILE" ]; then
        cp "$WASM_FILE" "$WASM_OVERRIDE_DIR/"
        print_success "WASM runtime copied to override directory"
        
        # Show file info
        WASM_SIZE=$(du -h "$WASM_OVERRIDE_DIR/node_subtensor_runtime.compact.wasm" | cut -f1)
        print_info "WASM file size: $WASM_SIZE"
    else
        print_error "WASM runtime file not found at $WASM_FILE"
        print_info "Make sure the runtime was built successfully"
        exit 1
    fi
}

# Function to run node with different configurations
run_node() {
    local chain_type="$1"
    local additional_args="$2"
    
    print_info "Starting Subtensor node with WASM runtime overrides"
    print_info "Chain: $chain_type"
    print_info "Override directory: $WASM_OVERRIDE_DIR"
    
    case "$chain_type" in
        "local")
            print_info "Running local development node..."
            "$NODE_BINARY" \
                --wasm-runtime-overrides "$WASM_OVERRIDE_DIR" \
                --chain local \
                --alice \
                --tmp \
                --rpc-cors all \
                --rpc-methods unsafe \
                --rpc-external \
                $additional_args
            ;;
        "test")
            print_info "Running against test network..."
            "$NODE_BINARY" \
                --wasm-runtime-overrides "$WASM_OVERRIDE_DIR" \
                --chain test_finney \
                --sync warp \
                --tmp \
                # --bootnodes /dns/bootnode.test.finney.opentensor.ai/tcp/30333/ws/p2p/12D3KooWPM4mLcKJGtyVtkggqdG84zWrd7Rij6PGQDoijh1X86Vr \
                $additional_args
            ;;
        "finney")
            print_warning "Running against Finney mainnet - use with caution!"
            read -p "Are you sure you want to connect to mainnet? (y/N): " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                "$NODE_BINARY" \
                    --wasm-runtime-overrides "$WASM_OVERRIDE_DIR" \
                    --chain finney \
                    --sync warp \
                    --tmp \
                    # --bootnodes /dns/bootnode.finney.chain.opentensor.ai/tcp/30333/ws/p2p/12D3KooWRwbMb85RWnT8DSXSYMWQtuDwh4LJzndoRrTDotTR5gDC \
                    $additional_args
            else
                print_info "Cancelled mainnet connection"
                exit 0
            fi
            ;;
        *)
            print_error "Unknown chain type: $chain_type"
            print_info "Supported chains: local, test, finney"
            exit 1
            ;;
    esac
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS] <CHAIN_TYPE>"
    echo ""
    echo "CHAIN_TYPE:"
    echo "  local    - Run local development node"
    echo "  test     - Connect to test network"
    echo "  finney   - Connect to Finney mainnet"
    echo ""
    echo "OPTIONS:"
    echo "  --build-only     Build runtime and setup overrides without running"
    echo "  --no-build       Skip runtime build (use existing WASM)"
    echo "  --clean          Clean override directory before setup"
    echo "  --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 local                    # Build and run local node"
    echo "  $0 --build-only local       # Only build and setup overrides"
    echo "  $0 --no-build test          # Run test node without rebuilding"
    echo "  $0 --clean local            # Clean and rebuild for local node"
}

# Parse command line arguments
BUILD_ONLY=false
NO_BUILD=false
CLEAN=false
CHAIN_TYPE=""
ADDITIONAL_ARGS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --build-only)
            BUILD_ONLY=true
            shift
            ;;
        --no-build)
            NO_BUILD=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        --*)
            ADDITIONAL_ARGS="$ADDITIONAL_ARGS $1"
            shift
            ;;
        *)
            if [ -z "$CHAIN_TYPE" ]; then
                CHAIN_TYPE="$1"
            else
                ADDITIONAL_ARGS="$ADDITIONAL_ARGS $1"
            fi
            shift
            ;;
    esac
done

# Validate chain type
if [ -z "$CHAIN_TYPE" ]; then
    print_error "Chain type is required"
    show_usage
    exit 1
fi

# Main execution
print_info "Subtensor WASM Runtime Override Script"
print_info "====================================="

# Check if node binary exists
check_binary

# Clean override directory if requested
if [ "$CLEAN" = true ]; then
    print_info "Cleaning override directory"
    rm -rf "$WASM_OVERRIDE_DIR"
fi

# Build runtime if not skipped
if [ "$NO_BUILD" = false ]; then
    build_runtime
fi

# Setup WASM override
setup_wasm_override

# Run node if not build-only
if [ "$BUILD_ONLY" = false ]; then
    run_node "$CHAIN_TYPE" "$ADDITIONAL_ARGS"
else
    print_success "Build and setup completed. Override directory ready at: $WASM_OVERRIDE_DIR"
    print_info "To run the node manually:"
    print_info "$NODE_BINARY --wasm-runtime-overrides $WASM_OVERRIDE_DIR --chain $CHAIN_TYPE [other options]"
fi