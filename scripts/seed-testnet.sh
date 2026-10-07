#!/usr/bin/env bash
# Seeds a deployed Inowo contract on testnet with three demo events that show
# the full lifecycle:
#
#   1. Active    — sponsorships and ticket sales, open for anyone to join
#   2. Ended     — funds released to a venue and a crew, each with a memo
#   3. Cancelled — the sponsor and ticket holder have claimed refunds
#
# Usage: scripts/seed-testnet.sh <contract-id> [organizer-key-alias]
#
# The organizer key (default: inowo-deployer) must hold at least 18.5 testnet
# USDC from https://faucet.circle.com. Demo accounts are created and funded
# with Friendbot on first run and reused afterwards. Each run creates three new
# events.
set -euo pipefail

CONTRACT="${1:?usage: seed-testnet.sh <contract-id> [organizer-key-alias]}"
ORGANIZER="${2:-inowo-deployer}"
USDC_SAC="${USDC_SAC:-CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA}"
USDC_ASSET="USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
NET=(--network testnet)

usdc() { echo $(( $1 * 100000 )); } # hundredths of a USDC -> stroops

addr() { stellar keys address "$1"; }

# Runs a CLI command quietly, printing its stderr only if it fails.
quiet() {
  local err; err=$(mktemp)
  if ! "$@" 2>"$err"; then
    cat "$err" >&2; rm -f "$err"; return 1
  fi
  rm -f "$err"
}

invoke() {
  local source="$1"; shift
  quiet stellar contract invoke --id "$CONTRACT" --source "$source" "${NET[@]}" -- "$@"
}

retry() {
  local attempt
  for attempt in 1 2 3 4 5; do
    "$@" && return 0
    sleep 3
  done
  return 1
}

horizon_account() { curl -fsS "https://horizon-testnet.stellar.org/accounts/$1" 2>/dev/null; }
account_exists() { horizon_account "$1" >/dev/null; }
has_usdc_trustline() { horizon_account "$1" | grep -q "\"asset_issuer\": \"${USDC_ASSET#USDC:}\""; }
fund() { curl -fsS "https://friendbot.stellar.org/?addr=$1" >/dev/null 2>&1; }
trust_usdc() { quiet stellar tx new change-trust --source "$1" --line "$USDC_ASSET" "${NET[@]}" >/dev/null; }

ensure_account() {
  local name="$1" address
  if ! stellar keys address "$name" >/dev/null 2>&1; then
    stellar keys generate "$name" "${NET[@]}" >/dev/null 2>&1
  fi
  address="$(addr "$name")"
  account_exists "$address" || retry fund "$address" || { echo "could not fund $name" >&2; return 1; }
  has_usdc_trustline "$address" || retry trust_usdc "$name" || { echo "could not add USDC trustline for $name" >&2; return 1; }
}

send_usdc() {
  quiet stellar contract invoke --id "$USDC_SAC" --source "$ORGANIZER" "${NET[@]}" -- \
    transfer --from "$(addr "$ORGANIZER")" --to "$(addr "$1")" --amount "$2" >/dev/null
}

echo "Preparing demo accounts..."
for name in inowo-demo-sponsor-1 inowo-demo-sponsor-2 inowo-demo-attendee inowo-demo-venue inowo-demo-crew; do
  ensure_account "$name"
done
send_usdc inowo-demo-sponsor-1 "$(usdc 800)"
send_usdc inowo-demo-sponsor-2 "$(usdc 500)"
send_usdc inowo-demo-attendee "$(usdc 550)"

ORG_ADDR="$(addr "$ORGANIZER")"

create() { # name description venue date goal tiers-json
  invoke "$ORGANIZER" create_event --organizer "$ORG_ADDR" \
    --name "$1" --description "$2" --venue "$3" \
    --date_unix "$4" --funding_goal "$5" --tiers "$6" | tr -d '"'
}

tier() { printf '{"name":"%s","price":"%s","supply_cap":%s}' "$1" "$2" "$3"; }

echo "1/3 Active event..."
ACTIVE=$(create "Lagos Stellar Builders Meetup" \
  "Monthly meetup for developers building on Stellar and Soroban. Talks, demos, and pizza." \
  "Co-Creation Hub, Yaba, Lagos" 1794672000 "$(usdc 10000)" \
  "[$(tier General "$(usdc 100)" 150),$(tier VIP "$(usdc 300)" 30)]")
invoke inowo-demo-sponsor-1 sponsor_event --sponsor "$(addr inowo-demo-sponsor-1)" --event_id "$ACTIVE" --amount "$(usdc 500)" >/dev/null
invoke inowo-demo-sponsor-2 sponsor_event --sponsor "$(addr inowo-demo-sponsor-2)" --event_id "$ACTIVE" --amount "$(usdc 300)" >/dev/null
invoke inowo-demo-attendee buy_ticket --buyer "$(addr inowo-demo-attendee)" --event_id "$ACTIVE" --tier_index 0 >/dev/null
invoke inowo-demo-attendee buy_ticket --buyer "$(addr inowo-demo-attendee)" --event_id "$ACTIVE" --tier_index 1 >/dev/null

echo "2/3 Ended event with payouts..."
ENDED=$(create "Soroban Smart Contract Workshop" \
  "Hands-on workshop: write, test, and deploy your first Soroban contract." \
  "Abuja Tech Hub, Abuja" 1790416800 "$(usdc 2000)" \
  "[$(tier General "$(usdc 100)" 60)]")
invoke inowo-demo-sponsor-1 sponsor_event --sponsor "$(addr inowo-demo-sponsor-1)" --event_id "$ENDED" --amount "$(usdc 300)" >/dev/null
invoke inowo-demo-attendee buy_ticket --buyer "$(addr inowo-demo-attendee)" --event_id "$ENDED" --tier_index 0 >/dev/null
invoke "$ORGANIZER" redeem_ticket --organizer "$ORG_ADDR" --event_id "$ENDED" --ticket_id 0 >/dev/null
invoke "$ORGANIZER" end_event --organizer "$ORG_ADDR" --event_id "$ENDED" >/dev/null
invoke "$ORGANIZER" release_funds --organizer "$ORG_ADDR" --event_id "$ENDED" \
  --recipient "$(addr inowo-demo-venue)" --amount "$(usdc 250)" --memo "Venue hire - workshop room" >/dev/null
invoke "$ORGANIZER" release_funds --organizer "$ORG_ADDR" --event_id "$ENDED" \
  --recipient "$(addr inowo-demo-crew)" --amount "$(usdc 150)" --memo "AV and livestream crew" >/dev/null

echo "3/3 Cancelled event with refunds..."
CANCELLED=$(create "Campus Hack Night" \
  "Overnight student hackathon. Cancelled after the venue fell through - all funds refunded." \
  "University of Lagos, Akoka" 1792864800 "$(usdc 3000)" \
  "[$(tier Student "$(usdc 50)" 200)]")
invoke inowo-demo-sponsor-2 sponsor_event --sponsor "$(addr inowo-demo-sponsor-2)" --event_id "$CANCELLED" --amount "$(usdc 200)" >/dev/null
TICKET=$(invoke inowo-demo-attendee buy_ticket --buyer "$(addr inowo-demo-attendee)" --event_id "$CANCELLED" --tier_index 0 | tr -d '"')
invoke "$ORGANIZER" cancel_event --organizer "$ORG_ADDR" --event_id "$CANCELLED" >/dev/null
invoke inowo-demo-sponsor-2 refund_sponsorship --sponsor "$(addr inowo-demo-sponsor-2)" --event_id "$CANCELLED" >/dev/null
invoke inowo-demo-attendee refund_ticket --owner "$(addr inowo-demo-attendee)" --event_id "$CANCELLED" --ticket_id "$TICKET" >/dev/null

echo
echo "Seeded contract $CONTRACT"
echo "  Active event:    $ACTIVE"
echo "  Ended event:     $ENDED"
echo "  Cancelled event: $CANCELLED"
