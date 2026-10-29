// `stellar contract bindings typescript` emits the ContractSpec's entries in
// whatever order the host build produced the WASM's custom section in —
// which isn't guaranteed stable across platforms/toolchains, even for
// identical contract source. Sorting them canonicalizes the output so
// CI's regenerate-and-diff check only fails on a real content change, not
// a platform-dependent reordering of the exact same entries.
import { readFileSync, writeFileSync } from "node:fs";

const path = "bindings/src/index.ts";
const src = readFileSync(path, "utf8");

const start = src.indexOf("new ContractSpec([");
if (start === -1) throw new Error("new ContractSpec([ not found — did the generator's output format change?");
const openBracket = src.indexOf("[", start);

let depth = 0;
let end = -1;
for (let i = openBracket; i < src.length; i++) {
  if (src[i] === "[") depth++;
  else if (src[i] === "]") {
    depth--;
    if (depth === 0) {
      end = i;
      break;
    }
  }
}
if (end === -1) throw new Error("no matching closing bracket for the ContractSpec array");

const inner = src.slice(openBracket + 1, end);
const entries = inner.match(/"(?:[^"\\]|\\.)*"/g) || [];
entries.sort();

const replacement = "[" + entries.map((e) => `\n        ${e}`).join(",") + " ]";
writeFileSync(path, src.slice(0, openBracket) + replacement + src.slice(end + 1));
