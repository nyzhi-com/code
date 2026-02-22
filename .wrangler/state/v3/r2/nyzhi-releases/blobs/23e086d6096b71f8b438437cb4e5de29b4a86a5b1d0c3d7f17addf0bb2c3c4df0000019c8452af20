#!/bin/sh
# nyzhi code installer
# Usage: curl -fsSL https://get.nyzhi.com | sh
#
# This script ONLY touches:
#   - $NYZHI_HOME/bin/nyzhi  (the binary)
#   - Shell profile (append PATH if needed)
#
# It NEVER touches:
#   - ~/.config/nyzhi/        (user configuration)
#   - ~/.local/share/nyzhi/   (sessions, history, analytics)
#   - OS keyring              (OAuth tokens)
#   - .nyzhi/ project dirs    (project-level config)
#
set -eu

RELEASE_URL="${NYZHI_RELEASE_URL:-https://get.nyzhi.com}"
NYZHI_HOME="${NYZHI_HOME:-$HOME/.nyzhi}"
INSTALL_DIR="${NYZHI_HOME}/bin"

main() {
  check_deps
  detect_platform
  fetch_version_info
  check_existing_install
  download_binary
  verify_checksum
  backup_existing
  install_binary
  verify_install
  setup_path
  print_success
}

check_deps() {
  for cmd in curl tar uname; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      err "Required command not found: $cmd"
    fi
  done
  if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    warn "Neither sha256sum nor shasum found — skipping checksum verification"
    SKIP_CHECKSUM=1
  else
    SKIP_CHECKSUM=0
  fi
}

detect_platform() {
  OS="$(uname -s)"
  case "$OS" in
    Linux)  OS="linux" ;;
    Darwin) OS="darwin" ;;
    *)      err "Unsupported operating system: $OS" ;;
  esac

  ARCH="$(uname -m)"
  case "$ARCH" in
    x86_64|amd64)   ARCH="x86_64" ;;
    aarch64|arm64)   ARCH="aarch64" ;;
    *)               err "Unsupported architecture: $ARCH" ;;
  esac

  info "Detected platform: ${OS}/${ARCH}"
}

fetch_version_info() {
  info "Fetching latest version info..."
  VERSION_JSON="$(curl -fsSL "${RELEASE_URL}/version")" || err "Failed to fetch version info"
  VERSION="$(printf '%s' "$VERSION_JSON" | parse_json_field "version")"
  CHECKSUM="$(printf '%s' "$VERSION_JSON" | parse_json_field "${OS}-${ARCH}")"

  if [ -z "$VERSION" ]; then
    err "Could not determine latest version"
  fi
  info "Latest version: v${VERSION}"
}

check_existing_install() {
  EXISTING_BIN="${INSTALL_DIR}/nyzhi"
  EXISTING_VERSION=""

  if [ -f "$EXISTING_BIN" ]; then
    EXISTING_VERSION="$("$EXISTING_BIN" --version 2>/dev/null | sed 's/[^0-9.]//g' || true)"
    if [ -n "$EXISTING_VERSION" ]; then
      info "Existing installation found: v${EXISTING_VERSION}"
      if [ "$EXISTING_VERSION" = "$VERSION" ]; then
        info "Already up to date (v${VERSION}). Nothing to do."
        exit 0
      fi
    else
      info "Existing installation found (unknown version)"
    fi
  fi
}

download_binary() {
  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR"' EXIT
  TARBALL="${TMPDIR}/nyzhi.tar.gz"

  info "Downloading nyzhi v${VERSION} for ${OS}/${ARCH}..."
  curl -fsSL "${RELEASE_URL}/download/${OS}/${ARCH}?version=${VERSION}" -o "$TARBALL" \
    || err "Download failed"
}

verify_checksum() {
  if [ "$SKIP_CHECKSUM" = "1" ] || [ -z "$CHECKSUM" ]; then
    warn "Skipping checksum verification"
    return
  fi

  info "Verifying checksum..."
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL="$(sha256sum "$TARBALL" | cut -d' ' -f1)"
  else
    ACTUAL="$(shasum -a 256 "$TARBALL" | cut -d' ' -f1)"
  fi

  if [ "$ACTUAL" != "$CHECKSUM" ]; then
    err "Checksum mismatch!\n  Expected: ${CHECKSUM}\n  Actual:   ${ACTUAL}"
  fi
  info "Checksum verified"
}

backup_existing() {
  EXISTING_BIN="${INSTALL_DIR}/nyzhi"
  if [ ! -f "$EXISTING_BIN" ]; then
    return
  fi

  BACKUP_DIR="${NYZHI_HOME}/backups"
  mkdir -p "$BACKUP_DIR"

  TIMESTAMP="$(date +%s)"
  BACKUP_NAME="nyzhi-v${EXISTING_VERSION:-unknown}-${TIMESTAMP}"
  BACKUP_PATH="${BACKUP_DIR}/${BACKUP_NAME}"

  cp "$EXISTING_BIN" "$BACKUP_PATH"
  chmod +x "$BACKUP_PATH"
  info "Backed up existing binary to ${BACKUP_PATH}"

  # Keep only the 3 newest backups
  BACKUP_COUNT="$(ls -1 "$BACKUP_DIR" | wc -l | tr -d ' ')"
  if [ "$BACKUP_COUNT" -gt 3 ]; then
    ls -1t "$BACKUP_DIR" | tail -n +"4" | while read -r OLD; do
      rm -f "${BACKUP_DIR}/${OLD}"
    done
  fi
}

install_binary() {
  mkdir -p "$INSTALL_DIR"
  tar -xzf "$TARBALL" -C "$TMPDIR"

  EXTRACTED="${TMPDIR}/nyzhi"
  if [ ! -f "$EXTRACTED" ]; then
    EXTRACTED="$(find "$TMPDIR" -name nyzhi -type f | head -1)"
  fi
  if [ -z "$EXTRACTED" ] || [ ! -f "$EXTRACTED" ]; then
    err "Could not find nyzhi binary in archive"
  fi

  chmod +x "$EXTRACTED"
  mv "$EXTRACTED" "${INSTALL_DIR}/nyzhi"
  info "Installed to ${INSTALL_DIR}/nyzhi"
}

verify_install() {
  NEW_BIN="${INSTALL_DIR}/nyzhi"
  if [ ! -x "$NEW_BIN" ]; then
    err "Installation failed: binary not executable"
  fi

  INSTALLED_VERSION="$("$NEW_BIN" --version 2>/dev/null || true)"
  if [ -z "$INSTALLED_VERSION" ]; then
    warn "Could not verify new binary (--version failed)"
    # Attempt rollback if we have a backup
    if [ -n "${BACKUP_PATH:-}" ] && [ -f "${BACKUP_PATH:-}" ]; then
      warn "Rolling back to previous version..."
      cp "$BACKUP_PATH" "$NEW_BIN"
      chmod +x "$NEW_BIN"
      err "New binary is broken. Rolled back to previous version.\n  Broken binary download may be corrupt — try again later."
    fi
  else
    info "Verified: ${INSTALLED_VERSION}"
  fi

  # Verify user data was not touched
  CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/nyzhi"
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/nyzhi"

  if [ -d "$CONFIG_DIR" ]; then
    info "Config preserved: ${CONFIG_DIR}"
  fi
  if [ -d "$DATA_DIR" ]; then
    info "Data preserved: ${DATA_DIR}"
  fi
}

setup_path() {
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) return ;;
  esac

  EXPORT_LINE="export PATH=\"\${NYZHI_HOME:-\$HOME/.nyzhi}/bin:\$PATH\""

  SHELL_NAME="$(basename "${SHELL:-/bin/sh}")"
  case "$SHELL_NAME" in
    zsh)  PROFILE="$HOME/.zshrc" ;;
    bash)
      if [ -f "$HOME/.bashrc" ]; then PROFILE="$HOME/.bashrc"
      elif [ -f "$HOME/.bash_profile" ]; then PROFILE="$HOME/.bash_profile"
      else PROFILE="$HOME/.profile"
      fi ;;
    fish)
      FISH_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/fish"
      mkdir -p "$FISH_DIR/conf.d"
      printf 'set -gx PATH "%s" $PATH\n' "$INSTALL_DIR" > "$FISH_DIR/conf.d/nyzhi.fish"
      info "Added nyzhi to fish PATH via conf.d/nyzhi.fish"
      return ;;
    *)    PROFILE="$HOME/.profile" ;;
  esac

  if [ -f "$PROFILE" ] && grep -q "nyzhi" "$PROFILE" 2>/dev/null; then
    return
  fi

  printf '\n# nyzhi\n%s\n' "$EXPORT_LINE" >> "$PROFILE"
  info "Added nyzhi to PATH in $PROFILE"
}

print_success() {
  printf '\n'
  if [ -n "${EXISTING_VERSION:-}" ]; then
    printf '  \033[1;32m✓\033[0m nyzhi updated: v%s → v%s\n' "$EXISTING_VERSION" "$VERSION"
  else
    printf '  \033[1;32m✓\033[0m nyzhi v%s installed successfully!\n' "$VERSION"
  fi
  printf '\n'
  printf '  \033[2mConfig:   %s\033[0m\n' "${XDG_CONFIG_HOME:-$HOME/.config}/nyzhi/"
  printf '  \033[2mData:     %s\033[0m\n' "${XDG_DATA_HOME:-$HOME/.local/share}/nyzhi/"
  printf '  \033[2mBinary:   %s\033[0m\n' "${INSTALL_DIR}/nyzhi"
  printf '\n'
  if [ -z "${EXISTING_VERSION:-}" ]; then
    printf '  To get started, open a new terminal and run:\n'
    printf '\n'
    printf '    \033[1mnyzhi\033[0m\n'
    printf '\n'
    SHELL_NAME="$(basename "${SHELL:-/bin/sh}")"
    printf '  Or restart your shell:\n'
    printf '\n'
    printf '    \033[2mexec %s\033[0m\n' "$SHELL_NAME"
    printf '\n'
  else
    printf '  Restart nyzhi to use the new version.\n'
    printf '\n'
  fi
}

parse_json_field() {
  FIELD="$1"
  sed -n 's/.*"'"$FIELD"'"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
}

info() { printf '  \033[1;34m→\033[0m %s\n' "$*"; }
warn() { printf '  \033[1;33m⚠\033[0m %s\n' "$*"; }
err()  { printf '  \033[1;31m✗\033[0m %s\n' "$*" >&2; exit 1; }

main "$@"
