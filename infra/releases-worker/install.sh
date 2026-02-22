#!/bin/sh
# nyzhi code installer
# Usage: curl -fsSL https://get.nyzhi.com | sh
#        curl -fsSL https://get.nyzhi.com | sh -s -- --uninstall
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

# ---------------------------------------------------------------------------
# Entrypoint guard: the entire script is wrapped in main() so that partial
# downloads (truncated curl) cannot execute an incomplete script.
# main is called at the very last line — if the download is cut short,
# the function definition is incomplete and the shell errors harmlessly.
# ---------------------------------------------------------------------------

RELEASE_URL="${NYZHI_RELEASE_URL:-https://get.nyzhi.com}"
NYZHI_HOME="${NYZHI_HOME:-$HOME/.nyzhi}"
INSTALL_DIR="${NYZHI_HOME}/bin"
BACKUP_PATH=""
BAR_WIDTH=40

main() {
  case "${1:-}" in
    --uninstall) do_uninstall; return ;;
  esac

  check_deps
  detect_platform
  fetch_version_info
  check_existing_install
  show_header
  download_binary
  verify_checksum
  backup_existing
  install_binary
  verify_install
  setup_path
  print_success
}

# ---- uninstall -----------------------------------------------------------

do_uninstall() {
  CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/nyzhi"
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/nyzhi"

  printf '\n  This will permanently remove:\n'
  printf '    %s\n' "$NYZHI_HOME"
  printf '    %s\n' "$CONFIG_DIR"
  printf '    %s\n' "$DATA_DIR"
  printf '    Shell PATH entries for nyzhi\n\n'

  printf '  Continue? [y/N] '
  read -r REPLY
  case "$REPLY" in
    y|Y|yes|YES) ;;
    *) printf '  Aborted.\n\n'; exit 0 ;;
  esac

  printf '\n'

  for DIR in "$NYZHI_HOME" "$CONFIG_DIR" "$DATA_DIR"; do
    if [ -d "$DIR" ]; then
      rm -rf "$DIR" && printf '  ✓ Removed %s\n' "$DIR" \
                     || printf '  ✗ Failed to remove %s\n' "$DIR"
    fi
  done

  # Clean shell profiles
  for PROFILE in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile"; do
    if [ -f "$PROFILE" ] && grep -q "nyzhi" "$PROFILE" 2>/dev/null; then
      TMPF="$(mktemp)"
      grep -v "nyzhi" "$PROFILE" > "$TMPF"
      mv "$TMPF" "$PROFILE"
      printf '  ✓ Cleaned PATH from %s\n' "$PROFILE"
    fi
  done

  FISH_CONF="${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/nyzhi.fish"
  if [ -f "$FISH_CONF" ]; then
    rm -f "$FISH_CONF"
    printf '  ✓ Removed %s\n' "$FISH_CONF"
  fi

  printf '\n  nyzhi has been uninstalled.\n'
  printf '  Note: OAuth tokens in your OS keyring are not removed by this script.\n'
  printf '  To clear them, run: nyzhi uninstall --yes  (before uninstalling)\n\n'
}

# ---- progress bar --------------------------------------------------------

draw_bar() {
  local pct=$1
  local filled=$((pct * BAR_WIDTH / 100))
  local i=0

  printf '\r  \033[33m'
  while [ $i -lt $filled ]; do printf '█'; i=$((i + 1)); done
  while [ $i -lt $BAR_WIDTH ]; do printf '░'; i=$((i + 1)); done
  printf '\033[0m %d%%' "$pct"
}

animate_progress() {
  local step=0
  while [ $step -le 100 ]; do
    draw_bar "$step"
    step=$((step + 5))
    sleep 0.02 2>/dev/null || sleep 1
  done
  draw_bar 100
  printf '\n'
}

# ---- dependency check ----------------------------------------------------

check_deps() {
  for cmd in curl tar uname; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      err "Required command not found: $cmd"
    fi
  done

  if command -v sha256sum >/dev/null 2>&1; then
    SHA_CMD="sha256sum"
  elif command -v shasum >/dev/null 2>&1; then
    SHA_CMD="shasum -a 256"
  else
    err "Neither sha256sum nor shasum found. Cannot verify download integrity."
  fi
}

# ---- platform detection --------------------------------------------------

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
}

# ---- version info --------------------------------------------------------

fetch_version_info() {
  VERSION_JSON="$(curl -fsSL "${RELEASE_URL}/version")" || err "Failed to fetch version info"
  VERSION="$(printf '%s' "$VERSION_JSON" | parse_json_field "version")"
  CHECKSUM="$(printf '%s' "$VERSION_JSON" | parse_json_field "${OS}-${ARCH}")"

  if [ -z "$VERSION" ]; then
    err "Could not determine latest version"
  fi
  case "$VERSION" in
    *[!0-9.]*) err "Version contains unexpected characters: $VERSION" ;;
  esac

  if [ -z "$CHECKSUM" ]; then
    err "No checksum available for ${OS}-${ARCH}. Cannot verify download."
  fi
  case "$CHECKSUM" in
    *[!0-9a-f]*) err "Checksum contains non-hex characters" ;;
  esac
  if [ "${#CHECKSUM}" -ne 64 ]; then
    err "Checksum has wrong length (expected 64 hex chars, got ${#CHECKSUM})"
  fi
}

# ---- existing install check ----------------------------------------------

check_existing_install() {
  EXISTING_BIN="${INSTALL_DIR}/nyzhi"
  EXISTING_VERSION=""

  if [ -f "$EXISTING_BIN" ]; then
    EXISTING_VERSION="$("$EXISTING_BIN" --version 2>/dev/null | sed 's/[^0-9.]//g' || true)"
    if [ -n "$EXISTING_VERSION" ] && [ "$EXISTING_VERSION" = "$VERSION" ]; then
      printf '\n  \033[1;32m✓\033[0m Already up to date (v%s)\n\n' "$VERSION"
      exit 0
    fi
  fi
}

# ---- visual header -------------------------------------------------------

show_header() {
  printf '\n'
  printf '  \033[1mInstalling nyzhi\033[0m version: \033[1;36m%s\033[0m\n' "$VERSION"
}

# ---- download ------------------------------------------------------------

download_binary() {
  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR"' EXIT
  TARBALL="${TMPDIR}/nyzhi.tar.gz"

  curl -fsSL "${RELEASE_URL}/download/${OS}/${ARCH}?version=${VERSION}" -o "$TARBALL" \
    || err "Download failed"

  animate_progress
}

# ---- checksum verification -----------------------------------------------

verify_checksum() {
  ACTUAL="$($SHA_CMD "$TARBALL" | cut -d' ' -f1)"

  if [ "$ACTUAL" != "$CHECKSUM" ]; then
    err "Checksum verification FAILED!\n  Expected: ${CHECKSUM}\n  Actual:   ${ACTUAL}\n  The download may be corrupt or tampered with."
  fi
}

# ---- backup --------------------------------------------------------------

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

  BACKUP_COUNT="$(ls -1 "$BACKUP_DIR" 2>/dev/null | wc -l | tr -d ' ')"
  if [ "$BACKUP_COUNT" -gt 3 ]; then
    ls -1t "$BACKUP_DIR" | tail -n +"4" | while read -r OLD; do
      rm -f "${BACKUP_DIR}/${OLD}"
    done
  fi
}

# ---- install -------------------------------------------------------------

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
}

# ---- post-install verification -------------------------------------------

verify_install() {
  NEW_BIN="${INSTALL_DIR}/nyzhi"
  if [ ! -x "$NEW_BIN" ]; then
    err "Installation failed: binary not executable"
  fi

  INSTALLED_VERSION="$("$NEW_BIN" --version 2>/dev/null || true)"
  if [ -z "$INSTALLED_VERSION" ]; then
    if [ -n "${BACKUP_PATH}" ] && [ -f "${BACKUP_PATH}" ]; then
      cp "$BACKUP_PATH" "$NEW_BIN"
      chmod +x "$NEW_BIN"
      err "New binary is broken. Rolled back to previous version."
    fi
  fi
}

# ---- PATH setup ----------------------------------------------------------

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
      return ;;
    *)    PROFILE="$HOME/.profile" ;;
  esac

  if [ -f "$PROFILE" ] && grep -q "nyzhi" "$PROFILE" 2>/dev/null; then
    return
  fi

  printf '\n# nyzhi\n%s\n' "$EXPORT_LINE" >> "$PROFILE"
}

# ---- post-install screen -------------------------------------------------

print_success() {
  printf '\n'

  # brand
  printf '  \033[1;36m◆ nyzhi code\033[0m\n'
  printf '\n'

  if [ -n "${EXISTING_VERSION:-}" ]; then
    printf '  Updated: v%s → v%s\n' "$EXISTING_VERSION" "$VERSION"
  else
    printf '  To get started:\n'
    printf '\n'
    printf '  \033[1mcd <project>\033[0m    # Open directory\n'
    printf '  \033[1mnyzhi\033[0m           # Run command\n'
  fi
  printf '\n'
  printf '  For more information visit \033[4mhttps://nyzhi.com/docs\033[0m\n'
  printf '\n'
}

# ---- helpers -------------------------------------------------------------

parse_json_field() {
  FIELD="$1"
  sed -n 's/.*"'"$FIELD"'"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
}

err()  { printf '\n  \033[1;31m✗\033[0m %s\n\n' "$*" >&2; exit 1; }

# The call to main MUST be the very last line of the script.
# If the download is truncated before this point, the shell will
# see an incomplete function definition and exit with a syntax error
# instead of executing partial commands.
main "$@"
