#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# main.sh
# vecnode Tools CLI (intended to run inside the Alpine Docker container).
# ---------------------------------------------------------------------------

check_tools_dependencies() {
  local tools=("pandoc" "python3" "yt-dlp")
  local missing=0

  echo ""
  echo "# ============================"
  echo "# Tools Dependencies"
  echo "# ============================"
  echo ""

  for tool in "${tools[@]}"; do
    if command -v "$tool" >/dev/null 2>&1; then
      case "$tool" in
        pandoc)
          echo "[OK] $tool: $(pandoc --version | head -n1)"
          ;;
        python3)
          echo "[OK] $tool: $(python3 --version 2>&1)"
          ;;
        yt-dlp)
          echo "[OK] $tool: $(yt-dlp --version 2>&1)"
          ;;
      esac
    else
      echo "[ERROR] Missing dependency: $tool"
      missing=1
    fi
  done

  echo ""
  if [[ "$missing" -eq 0 ]]; then
    echo "[INFO] All Tools CLI dependencies are installed."
  else
    echo "[WARNING] Some Tools CLI dependencies are missing."
  fi
}

clear
echo ""
echo "# ============================"
echo "# vecnode Tools CLI"
echo "# ============================"
echo ""

while true; do
  echo "What would you like to do?"
  echo "  1 = Check Tools Dependencies"
  echo "  2 = Python REPL"
  echo "  3 = Pandoc Version"
  echo "  4 = yt-dlp Version"
  echo "  5 = Shell"
  echo "  6 = Quit"
  echo ""
  read -r -p "Enter your choice (1, 2, 3, 4, 5, or 6): " TOOLS_CHOICE
  clear

  if [[ "$TOOLS_CHOICE" == "1" ]]; then
    check_tools_dependencies
    echo ""
    continue
  fi

  if [[ "$TOOLS_CHOICE" == "2" ]]; then
    echo ""
    echo "[INFO] Starting Python REPL. Type exit() to return."
    python3 || true
    echo ""
    continue
  fi

  if [[ "$TOOLS_CHOICE" == "3" ]]; then
    echo ""
    pandoc --version | head -n1 || true
    echo ""
    continue
  fi

  if [[ "$TOOLS_CHOICE" == "4" ]]; then
    echo ""
    yt-dlp --version || true
    echo ""
    continue
  fi

  if [[ "$TOOLS_CHOICE" == "5" ]]; then
    echo ""
    echo "[INFO] Starting interactive shell. Type exit to return to Tools CLI."
    bash || true
    echo ""
    continue
  fi

  if [[ "$TOOLS_CHOICE" == "6" ]]; then
    echo ""
    echo "[INFO] Exiting Tools CLI."
    exit 0
  fi

  echo "[ERROR] Invalid choice. Please enter 1, 2, 3, 4, 5, or 6."
  echo ""
done
