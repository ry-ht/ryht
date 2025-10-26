#!/usr/bin/env bash
# Build script that compiles all binaries and copies them to dist/
set -e

# Set PATH to include cargo
export PATH="/Users/taaliman/.cargo/bin:/usr/local/bin:/usr/bin:/bin"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Building Cortex Workspace ===${NC}"

# Determine build profile (default to release)
PROFILE="${1:-release}"

if [ "$PROFILE" = "release" ]; then
    BUILD_CMD="cargo build --release"
    TARGET_DIR="target/release"
else
    BUILD_CMD="cargo build"
    TARGET_DIR="target/debug"
fi

echo -e "${BLUE}Build profile: ${PROFILE}${NC}"
echo -e "${BLUE}Target directory: ${TARGET_DIR}${NC}"

# Build all binaries
echo -e "\n${BLUE}Building binaries...${NC}"
$BUILD_CMD

# Create dist directory if it doesn't exist
mkdir -p dist

# Copy binaries to dist
echo -e "\n${BLUE}Copying binaries to dist/...${NC}"

BINARIES=("cortex" "axon")

for binary in "${BINARIES[@]}"; do
    if [ -f "${TARGET_DIR}/${binary}" ]; then
        cp "${TARGET_DIR}/${binary}" dist/
        echo -e "${GREEN}✓ Copied ${binary}${NC}"
    else
        echo -e "⚠ Binary ${binary} not found at ${TARGET_DIR}/${binary}"
    fi
done

# Check for dashboard binary (might have different name)
if [ -f "${TARGET_DIR}/dashboard" ]; then
    cp "${TARGET_DIR}/dashboard" dist/
    echo -e "${GREEN}✓ Copied dashboard${NC}"
elif [ -f "${TARGET_DIR}/cortex-dashboard" ]; then
    cp "${TARGET_DIR}/cortex-dashboard" dist/dashboard
    echo -e "${GREEN}✓ Copied cortex-dashboard as dashboard${NC}"
fi

echo -e "\n${GREEN}=== Build Complete ===${NC}"
echo -e "Binaries available in: ${PWD}/dist/"
ls -lh dist/
