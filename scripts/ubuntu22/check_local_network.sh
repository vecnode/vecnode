#!/usr/bin/env bash
set -euo pipefail

echo "[Local Network Scan]"
echo ""

LOCAL_IP=""

if command -v ip >/dev/null 2>&1; then
  LOCAL_IP="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for (i=1; i<=NF; i++) if ($i=="src") {print $(i+1); exit}}')"
fi

if [[ -z "$LOCAL_IP" ]]; then
  LOCAL_IP="$(hostname -I 2>/dev/null | awk '{print $1}')"
fi

if [[ -z "$LOCAL_IP" ]]; then
  echo "[ERROR] Could not determine local IP address."
  exit 1
fi

echo "[INFO] Local IP: $LOCAL_IP"

IFS='.' read -r A B C _ <<<"$LOCAL_IP"
SUBNET="$A.$B.$C"

echo "[INFO] Scanning $SUBNET.1-254"
echo ""

FOUND=0
for i in $(seq 1 254); do
  if ping -c 1 -W 1 "$SUBNET.$i" >/dev/null 2>&1; then
    FOUND=$((FOUND + 1))
    echo "  $SUBNET.$i"
  fi
done

echo ""
if [[ "$FOUND" -eq 0 ]]; then
  echo "[INFO] No reachable hosts found on $SUBNET.0/24"
else
  echo "[INFO] $FOUND host(s) reachable on $SUBNET.0/24"
  echo ""
  if command -v arp >/dev/null 2>&1; then
    echo "[INFO] ARP cache for subnet (includes resolved hostnames):"
    arp -a 2>/dev/null | grep -F "($SUBNET." || true
  fi
fi

exit 0