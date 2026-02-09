#!/bin/bash
#
# Build script for TeaLeaf .NET library with native dependencies.
#
# Usage:
#   ./build.sh [OPTIONS]
#
# Options:
#   -c, --configuration <Debug|Release>  Build configuration (default: Release)
#   -r, --rids <rids>                    Comma-separated RIDs to build (default: native platform)
#                                        Use 'all' to build all supported platforms
#   -t, --test                           Run tests after building
#   -p, --pack                           Create NuGet package
#   -s, --skip-rust                      Skip Rust compilation
#   -h, --help                           Show this help
#
# Examples:
#   ./build.sh                           # Build for native platform
#   ./build.sh -r all -p                 # Build all platforms and package
#   ./build.sh -c Debug -t               # Debug build with tests
#   ./build.sh -r linux-x64,linux-arm64  # Build specific platforms

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# Script directory and project paths
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOTNET_DIR="$SCRIPT_DIR"
TEALEAF_DIR="$DOTNET_DIR/TeaLeaf"
RUNTIMES_DIR="$TEALEAF_DIR/runtimes"

# Default values
CONFIGURATION="Release"
TARGET_RIDS=""
RUN_TESTS=false
CREATE_PACK=false
SKIP_RUST=false

# Rust target mappings
declare -A RUST_TARGETS
RUST_TARGETS["win-x64"]="x86_64-pc-windows-msvc:tealeaf_ffi.dll"
RUST_TARGETS["win-x86"]="i686-pc-windows-msvc:tealeaf_ffi.dll"
RUST_TARGETS["win-arm64"]="aarch64-pc-windows-msvc:tealeaf_ffi.dll"
RUST_TARGETS["linux-x64"]="x86_64-unknown-linux-gnu:libtealeaf_ffi.so"
RUST_TARGETS["linux-arm64"]="aarch64-unknown-linux-gnu:libtealeaf_ffi.so"
RUST_TARGETS["linux-musl-x64"]="x86_64-unknown-linux-musl:libtealeaf_ffi.so"
RUST_TARGETS["linux-musl-arm64"]="aarch64-unknown-linux-musl:libtealeaf_ffi.so"
RUST_TARGETS["osx-x64"]="x86_64-apple-darwin:libtealeaf_ffi.dylib"
RUST_TARGETS["osx-arm64"]="aarch64-apple-darwin:libtealeaf_ffi.dylib"

ALL_RIDS="linux-arm64 linux-musl-arm64 linux-musl-x64 linux-x64 osx-arm64 osx-x64 win-arm64 win-x64 win-x86"

# Functions
write_step() {
    echo -e "\n${GREEN}=== $1 ===${NC}"
}

write_info() {
    echo -e "  ${GRAY}$1${NC}"
}

show_help() {
    head -25 "$0" | tail -23 | sed 's/^# //' | sed 's/^#//'
    exit 0
}

detect_native_rid() {
    local os=""
    local arch=""

    # Detect OS
    case "$(uname -s)" in
        Darwin*)  os="osx" ;;
        Linux*)
            # Check for musl
            if ldd --version 2>&1 | grep -q musl; then
                os="linux-musl"
            else
                os="linux"
            fi
            ;;
        MINGW*|MSYS*|CYGWIN*)  os="win" ;;
        *)  os="linux" ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)  arch="x64" ;;
        i686|i386)     arch="x86" ;;
        aarch64|arm64) arch="arm64" ;;
        *)             arch="x64" ;;
    esac

    echo "${os}-${arch}"
}

install_rust_target() {
    local target="$1"

    if ! rustup target list --installed 2>/dev/null | grep -q "^${target}$"; then
        echo -e "  ${YELLOW}Installing Rust target: ${target}${NC}"
        if ! rustup target add "$target" 2>/dev/null; then
            return 1
        fi
    fi
    return 0
}

build_rust_library() {
    local rid="$1"
    local config="$2"

    local target_info="${RUST_TARGETS[$rid]}"
    if [ -z "$target_info" ]; then
        echo -e "  ${YELLOW}Warning: Unknown RID: $rid${NC}"
        return 1
    fi

    local rust_target="${target_info%%:*}"
    local library_name="${target_info##*:}"

    echo -e "  ${CYAN}Building: $rid ($rust_target)${NC}"

    # Install target if needed
    if ! install_rust_target "$rust_target"; then
        echo -e "    ${YELLOW}Failed to install target $rust_target. Skipping.${NC}"
        return 1
    fi

    # Build
    local cargo_args="build --package tealeaf-ffi --target $rust_target"
    if [ "$config" = "Release" ]; then
        cargo_args="$cargo_args --release"
    fi

    pushd "$REPO_ROOT" > /dev/null
    if ! cargo $cargo_args 2>&1; then
        echo -e "    ${RED}Cargo build failed for $rust_target${NC}"
        popd > /dev/null
        return 1
    fi
    popd > /dev/null

    # Copy output
    local profile="debug"
    if [ "$config" = "Release" ]; then
        profile="release"
    fi

    local source_path="$REPO_ROOT/target/$rust_target/$profile/$library_name"
    local dest_dir="$RUNTIMES_DIR/$rid/native"
    local dest_path="$dest_dir/$library_name"

    if [ -f "$source_path" ]; then
        mkdir -p "$dest_dir"
        cp "$source_path" "$dest_path"
        write_info "Copied: $dest_path"
        return 0
    else
        echo -e "    ${YELLOW}Library not found at: $source_path${NC}"
        return 1
    fi
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -c|--configuration)
            CONFIGURATION="$2"
            shift 2
            ;;
        -r|--rids)
            TARGET_RIDS="$2"
            shift 2
            ;;
        -t|--test)
            RUN_TESTS=true
            shift
            ;;
        -p|--pack)
            CREATE_PACK=true
            shift
            ;;
        -s|--skip-rust)
            SKIP_RUST=true
            shift
            ;;
        -h|--help)
            show_help
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

# Validate configuration
if [[ "$CONFIGURATION" != "Debug" && "$CONFIGURATION" != "Release" ]]; then
    echo -e "${RED}Invalid configuration: $CONFIGURATION. Use Debug or Release.${NC}"
    exit 1
fi

# Determine which RIDs to build
if [ "$TARGET_RIDS" = "all" ]; then
    RIDS_TO_BUILD=($ALL_RIDS)
elif [ -n "$TARGET_RIDS" ]; then
    IFS=',' read -ra RIDS_TO_BUILD <<< "$TARGET_RIDS"
else
    # Default: build only native platform
    RIDS_TO_BUILD=($(detect_native_rid))
fi

# Main build process
write_step "TeaLeaf .NET Build Script"
echo "Configuration: $CONFIGURATION"
echo "Target RIDs: ${RIDS_TO_BUILD[*]}"

# Check prerequisites
write_step "Checking Prerequisites"

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Rust/Cargo not found. Please install from https://rustup.rs${NC}"
    exit 1
fi
echo "  Cargo: $(cargo --version)"

if ! command -v dotnet &> /dev/null; then
    echo -e "${RED}.NET SDK not found. Please install from https://dot.net${NC}"
    exit 1
fi
echo "  .NET: $(dotnet --version)"

# Build Rust libraries
if [ "$SKIP_RUST" = false ]; then
    write_step "Building Rust Native Libraries"

    success_count=0
    failed_rids=()

    for rid in "${RIDS_TO_BUILD[@]}"; do
        if build_rust_library "$rid" "$CONFIGURATION"; then
            ((success_count++))
        else
            failed_rids+=("$rid")
        fi
    done

    echo ""
    if [ $success_count -gt 0 ]; then
        echo -e "${GREEN}Built $success_count of ${#RIDS_TO_BUILD[@]} native libraries${NC}"
    fi
    if [ ${#failed_rids[@]} -gt 0 ]; then
        echo -e "${YELLOW}Failed to build: ${failed_rids[*]}${NC}"
    fi
    if [ $success_count -eq 0 ]; then
        echo -e "${RED}No native libraries were built successfully${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}Skipping Rust build (using existing libraries)${NC}"
fi

# Build .NET project
write_step "Building .NET Project"

dotnet build "$TEALEAF_DIR" -c "$CONFIGURATION"

# Run tests if requested
if [ "$RUN_TESTS" = true ]; then
    write_step "Running Tests"

    TEST_PROJECT="$DOTNET_DIR/TeaLeaf.Tests"
    if [ -d "$TEST_PROJECT" ]; then
        # Find native library to copy to test output
        NATIVE_LIB=$(find "$RUNTIMES_DIR" -name "libtealeaf_ffi.*" -o -name "tealeaf_ffi.dll" 2>/dev/null | head -1)
        if [ -n "$NATIVE_LIB" ]; then
            LIB_NAME=$(basename "$NATIVE_LIB")
            for framework in net8.0 net10.0; do
                TEST_BIN_DIR="$TEST_PROJECT/bin/$CONFIGURATION/$framework"
                if [ -d "$TEST_BIN_DIR" ]; then
                    cp "$NATIVE_LIB" "$TEST_BIN_DIR/"
                fi
            done
        fi

        dotnet test "$TEST_PROJECT" -c "$CONFIGURATION" --no-build
    else
        echo -e "${YELLOW}Warning: Test project not found at: $TEST_PROJECT${NC}"
    fi
fi

# Create NuGet package
if [ "$CREATE_PACK" = true ]; then
    write_step "Creating NuGet Package"

    ARTIFACTS_DIR="$DOTNET_DIR/artifacts"
    mkdir -p "$ARTIFACTS_DIR"

    dotnet pack "$TEALEAF_DIR" -c "$CONFIGURATION" --no-build -o "$ARTIFACTS_DIR"

    PACKAGE=$(ls -t "$ARTIFACTS_DIR"/*.nupkg 2>/dev/null | head -1)
    if [ -n "$PACKAGE" ]; then
        echo -e "\n${GREEN}Package created: $(basename "$PACKAGE")${NC}"
        write_info "Location: $PACKAGE"
    fi
fi

write_step "Build Complete"

# Show summary of native libraries
echo -e "\nNative libraries included:"
if [ -d "$RUNTIMES_DIR" ]; then
    find "$RUNTIMES_DIR" -type f | while read -r lib; do
        relative_path="${lib#$TEALEAF_DIR/}"
        write_info "$relative_path"
    done
else
    echo -e "${YELLOW}No native libraries found in runtimes folder${NC}"
fi
