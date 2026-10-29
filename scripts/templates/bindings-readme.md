# @inowo/contract-bindings

A typed TypeScript client for the Inowo Soroban contract, generated from the
contract's own spec with [`stellar contract bindings typescript`](https://github.com/stellar/js-stellar-sdk#cli).
Every argument type, return type, and error code is generated straight from
`src/lib.rs` — there's no hand-written `xdr.ScVal` encoding to drift out of
sync with the contract.

**Never hand-edit `src/index.ts`.** Regenerate it after any contract change:

```bash
./scripts/generate-bindings.sh
```

CI runs the same script and fails the build if the committed output doesn't
match what the current contract produces (see `.github/workflows`).

## Install

Not published to npm; add it as a local dependency from a sibling checkout,
or copy `bindings/` into your project:

```json
{
  "dependencies": {
    "@inowo/contract-bindings": "file:../inowo-Contract/bindings"
  }
}
```

Then `npm install` and `npm run build` inside `bindings/` once to produce `dist/`.

## Use it

```ts
import { Client, Errors } from "@inowo/contract-bindings";

const client = new Client({
  contractId: "CCWFDV2MIDV7QOEUJNZDFRRJY75O7S65JPVJ2S2WNARJ4INEMDTCNOPV",
  networkPassphrase: "Test SDF Network ; September 2015",
  rpcUrl: "https://soroban-testnet.stellar.org",
});

// Read: every method simulates first, so a read never needs a signature.
// Every function here returns the contract's own Result<T, Error>, so check
// isErr()/isOk() rather than assuming success.
const event = await client.get_event({ event_id: 0 });
if (event.result.isOk()) {
  console.log(event.result.unwrap().name);
}

// Write: simulate, sign, then send. publicKey must match the account
// server-side, and signTransaction is whatever your wallet integration
// provides (e.g. Freighter's signTransaction).
const sponsor = await client.sponsor_event(
  { sponsor: publicKey, event_id: 0, amount: 50_000_000n },
  { fee: 100 }
);
await sponsor.signAndSend({ signTransaction });
```

`Errors` (exported above) maps every code in the contract's `Error` enum to
its name, e.g. `Errors[16].message === "NotTicketOwner"`. Decoding *which*
code a given `Result.unwrapErr()` carries takes a bit more XDR-unwrapping
than fits in this quickstart — for now, `Errors` is most directly useful for
matching codes you already have from elsewhere (an RPC diagnostic, a log, the
inowo-api error responses this contract's error codes are mirrored into).

Every exported function and type carries the doc comment from the contract
source, so editor autocomplete doubles as the API reference — see
`src/index.ts` for the full list, or the main [README](../README.md#function-reference)
for the same functions described in prose.
