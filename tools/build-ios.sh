#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

NSH_BIN="${NSH_BIN:-nsh}"
MODE="${1:-simulator}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "ERROR: iOS builds require macOS with Xcode (nsh 0.5.1 ios component is published for macOS on Apple silicon only)." >&2
  exit 1
fi

if [[ "$(uname -m)" != "arm64" ]]; then
  echo "ERROR: this nsh version publishes the ios component for Apple silicon Macs only (uname -m must be arm64)." >&2
  exit 1
fi

if ! command -v xcrun >/dev/null 2>&1; then
  echo "ERROR: Xcode command line tools not found. Run: xcode-select --install" >&2
  exit 1
fi

if ! command -v "$NSH_BIN" >/dev/null 2>&1 && [[ ! -x "$NSH_BIN" ]]; then
  echo "ERROR: nsh not found. Install it from https://nsh.dev/docs/installation or set NSH_BIN=/path/to/nsh" >&2
  exit 1
fi

rustup target add aarch64-apple-ios-sim aarch64-apple-ios
"$NSH_BIN" component add ios

case "$MODE" in
  simulator|sim)
    # ad-hoc signed .app, boots a simulator and launches the app.
    "$NSH_BIN" package --target ios-simulator --run
    ;;
  device|ipa)
    SIGN="${2:-}"
    PROFILE="${3:-}"
    if [[ -z "$SIGN" || -z "$PROFILE" ]]; then
      echo "Usage: $0 device \"Apple Distribution: Team (TEAMID)\" AppStore.mobileprovision" >&2
      echo "List identities with: security find-identity -v -p codesigning" >&2
      echo "Without --sign/--profile nsh writes an unsigned .app that cannot be installed on a device." >&2
      exit 2
    fi
    "$NSH_BIN" package --target ios --sign "$SIGN" --profile "$PROFILE"
    ;;
  test)
    "$NSH_BIN" test
    ;;
  *)
    echo "Usage: $0 [simulator|device|test]" >&2
    exit 2
    ;;
esac

echo "Done. Output directory: $ROOT/.nsh/out/"
