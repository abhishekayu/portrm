#!/bin/sh
# portrm installer
# Usage: curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh
set -e

REPO="abhishekayu/portrm"
BINARY_NAME="ptrm"

# Pick install dir: user override > writable /usr/local/bin > ~/.local/bin
if [ -n "${PTRM_INSTALL_DIR:-}" ]; then
    INSTALL_DIR="$PTRM_INSTALL_DIR"
elif [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

# ── Colors ─────────────────────────────────────────────────────────────

setup_colors() {
    if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
        ESC=$(printf '\033')
        RED="${ESC}[0;31m"
        GREEN="${ESC}[0;32m"
        YELLOW="${ESC}[0;33m"
        BLUE="${ESC}[0;34m"
        MAGENTA="${ESC}[0;35m"
        CYAN="${ESC}[0;36m"
        WHITE="${ESC}[1;37m"
        BOLD="${ESC}[1m"
        DIM="${ESC}[2m"
        RESET="${ESC}[0m"
    else
        RED='' GREEN='' YELLOW='' BLUE='' MAGENTA='' CYAN='' WHITE='' BOLD='' DIM='' RESET=''
    fi
}

# ── Output helpers ─────────────────────────────────────────────────────

banner() {
    # Rainbow gradient top to bottom
    R1="${RED}"
    R2="${YELLOW}"
    R3="${GREEN}"
    R4="${CYAN}"
    R5="${BLUE}"
    R6="${MAGENTA}"
    # Face features
    EYE="${WHITE}${BOLD}●${RESET}"
    NOSE="${YELLOW}${BOLD}○${RESET}"
    W="${WHITE}"
    echo ""
    echo "            ${R1}.ooooooo.${RESET}"
    echo "        ${R1}.ooooooooooooooo.${RESET}"
    echo "      ${R1}.ooooo     ooooooo.${RESET}"
    echo "     ${R1}ooooo           ooooo${RESET}"
    echo "    ${R1}oooo${RESET}    ${EYE}   ${EYE}     ${R1}oooo${RESET}"
    echo "   ${R1}oooo${RESET}       ${NOSE}        ${R1}oooo${RESET}"
    echo "   ${R1}oooo${RESET}    ${EYE}   ${EYE}       ${R1}oooo${RESET}"
    echo "    ${R1}oooo             oooo${RESET}"
    echo "     ${R1}'oooo         oooo'${RESET}"
    echo "       ${R1}'ooooooooooooo'${RESET}"
    echo "            ${R1}'ooo'${RESET}"
    echo ""
    echo "        ${R3}${BOLD}P${R3}O${R3}R${R3}T${R3}R${R3}M${RESET}"
    echo ""
    echo "  ${DIM}Fix port conflicts. Debug processes. Recover instantly.${RESET}"
    echo "  ${DIM}------------------------------------------------------${RESET}"
    echo ""
}

info()    { echo "  ${CYAN}${BOLD}::${RESET} $1"; }
success() { echo "  ${GREEN}${BOLD}OK${RESET} $1"; }
warn()    { echo "  ${YELLOW}${BOLD}!!${RESET} $1"; }
fail()    { echo "  ${RED}${BOLD}FAIL${RESET} $1"; exit 1; }
step()    { printf "  ${DIM}>${RESET} %-45s" "$1"; }
done_()   { echo "${GREEN}${BOLD}done${RESET}"; }

# ── Detect platform ───────────────────────────────────────────────────

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  OS_NAME="linux";  OS_LABEL="Linux" ;;
        Darwin) OS_NAME="darwin"; OS_LABEL="macOS" ;;
        MINGW*|MSYS*|CYGWIN*)
            fail "Windows is not supported via this installer. Use: cargo install portrm"
            ;;
        *)
            fail "Unsupported OS: $OS"
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH_NAME="amd64";  ARCH_LABEL="x86_64 (amd64)" ;;
        aarch64|arm64)   ARCH_NAME="arm64"; ARCH_LABEL="ARM64" ;;
        *)
            fail "Unsupported architecture: $ARCH"
            ;;
    esac

    TARGET="portrm-${OS_NAME}-${ARCH_NAME}"
    success "Detected ${WHITE}${OS_LABEL} ${ARCH_LABEL}${RESET}"
}

# ── Get latest version ────────────────────────────────────────────────

get_latest_version() {
    step "Fetching latest release..."
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    else
        fail "curl or wget is required"
    fi

    if [ -z "$VERSION" ]; then
        fail "Could not determine latest version"
    fi

    done_
    success "Latest version: ${WHITE}v${VERSION}${RESET}"
}

# ── Download and install ──────────────────────────────────────────────

download_and_install() {
    URL="https://github.com/${REPO}/releases/download/v${VERSION}/${TARGET}.tar.gz"
    CHECKSUM_URL="${URL}.sha256"

    TMPDIR="$(mktemp -d)"
    trap 'rm -rf "$TMPDIR"' EXIT

    # Download
    step "Downloading ${DIM}${TARGET}.tar.gz${RESET}..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$URL" -o "${TMPDIR}/ptrm.tar.gz"
        curl -fsSL "$CHECKSUM_URL" -o "${TMPDIR}/ptrm.tar.gz.sha256"
    else
        wget -q "$URL" -O "${TMPDIR}/ptrm.tar.gz"
        wget -q "$CHECKSUM_URL" -O "${TMPDIR}/ptrm.tar.gz.sha256"
    fi
    done_

    # Verify checksum
    step "Verifying SHA256 checksum..."
    EXPECTED=$(cat "${TMPDIR}/ptrm.tar.gz.sha256" | awk '{print $1}')
    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL=$(sha256sum "${TMPDIR}/ptrm.tar.gz" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL=$(shasum -a 256 "${TMPDIR}/ptrm.tar.gz" | awk '{print $1}')
    else
        warn "Cannot verify checksum (no sha256sum or shasum found)"
        ACTUAL="$EXPECTED"
    fi

    if [ "$EXPECTED" != "$ACTUAL" ]; then
        echo "${RED}FAILED${RESET}"
        fail "Checksum mismatch! Expected ${EXPECTED}, got ${ACTUAL}"
    fi
    done_

    # Extract
    step "Extracting binary..."
    tar xzf "${TMPDIR}/ptrm.tar.gz" -C "${TMPDIR}"
    done_

    # Install
    step "Installing to ${CYAN}${INSTALL_DIR}${RESET}..."
    mv "${TMPDIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    done_

    # Sizes
    DOWNLOAD_SIZE=$(wc -c < "${TMPDIR}/ptrm.tar.gz" | tr -d ' ')
    BINARY_SIZE=$(wc -c < "${INSTALL_DIR}/${BINARY_NAME}" | tr -d ' ')
    if [ "$DOWNLOAD_SIZE" -ge 1048576 ]; then
        DOWNLOAD_HUMAN="$(echo "$DOWNLOAD_SIZE" | awk '{printf "%.1f MB", $1/1048576}')"
    else
        DOWNLOAD_HUMAN="$(echo "$DOWNLOAD_SIZE" | awk '{printf "%.0f KB", $1/1024}')"
    fi
    if [ "$BINARY_SIZE" -ge 1048576 ]; then
        BINARY_HUMAN="$(echo "$BINARY_SIZE" | awk '{printf "%.1f MB", $1/1048576}')"
    else
        BINARY_HUMAN="$(echo "$BINARY_SIZE" | awk '{printf "%.0f KB", $1/1024}')"
    fi

    # Success
    echo ""
    echo "  ${GREEN}${BOLD}------------------------------------------------${RESET}"
    echo "  ${GREEN}${BOLD}  portrm v${VERSION} installed successfully!${RESET}"
    echo "  ${GREEN}${BOLD}------------------------------------------------${RESET}"
    echo ""
    echo "  ${DIM}Download:${RESET}  ${DOWNLOAD_HUMAN}"
    echo "  ${DIM}Binary:${RESET}    ${BINARY_HUMAN}"
    echo "  ${DIM}Location:${RESET}  ${INSTALL_DIR}/${BINARY_NAME}"
    echo "  ${DIM}Platform:${RESET}  ${OS_LABEL} ${ARCH_LABEL}"
    echo ""

    if command -v ptrm >/dev/null 2>&1; then
        echo "  ${BOLD}Get started:${RESET}"
        echo ""
        echo "    ${CYAN}\$ ptrm scan${RESET}        ${DIM}# see all listening ports${RESET}"
        echo "    ${CYAN}\$ ptrm fix 3000${RESET}    ${DIM}# resolve port conflict${RESET}"
        echo "    ${CYAN}\$ ptrm doctor${RESET}      ${DIM}# auto-diagnose issues${RESET}"
        echo "    ${CYAN}\$ ptrm --help${RESET}      ${DIM}# full command reference${RESET}"
    else
        warn "Add ${CYAN}${INSTALL_DIR}${RESET} to your PATH, then run ${CYAN}ptrm --help${RESET}"
    fi
    echo ""
}

# ── Main ──────────────────────────────────────────────────────────────

setup_colors
banner
detect_platform
get_latest_version
echo ""
info "Installing portrm for ${WHITE}${OS_LABEL} ${ARCH_LABEL}${RESET}"
echo ""
download_and_install
