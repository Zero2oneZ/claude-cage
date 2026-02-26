#!/usr/bin/env node
// Mint a CageNFT on Sui after the package is published.
//
// Usage: node scripts/mint-nft.js <PACKAGE_ID> [--name "My NFT"] [--desc "..."] [--image "https://..."]
//
// Requires: PACKAGE_ID from `sui client publish` output.

import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { Transaction } from "@mysten/sui/transactions";
import { Ed25519Keypair } from "@mysten/sui/keypairs/ed25519";
import { readFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

// Parse args
const args = process.argv.slice(2);
const packageId = args[0];
if (!packageId) {
  console.error(
    "Usage: node mint-nft.js <PACKAGE_ID> [--name ...] [--desc ...] [--image ...]"
  );
  process.exit(1);
}

function getArg(flag) {
  const idx = args.indexOf(flag);
  return idx !== -1 && idx + 1 < args.length ? args[idx + 1] : null;
}

const name = getArg("--name") || "Cage NFT #1";
const desc = getArg("--desc") || "Move-native NFT from Claude Cage";
const imageUrl =
  getArg("--image") || "https://gentlyos.dev/nft-placeholder.png";

// Load keypair from Sui keystore
const keystorePath = join(homedir(), ".sui/sui_config/sui.keystore");
const keystore = JSON.parse(readFileSync(keystorePath, "utf-8"));
// First key in keystore
const rawKey = Buffer.from(keystore[0], "base64");
// Sui keystore format: first byte is scheme flag (0 = ed25519), rest is 32-byte secret
const keypair = Ed25519Keypair.fromSecretKey(rawKey.subarray(1));

const network = "testnet";
const client = new SuiClient({ url: getFullnodeUrl(network) });

console.log(`Package:  ${packageId}`);
console.log(`Address:  ${keypair.toSuiAddress()}`);
console.log(`Minting:  "${name}"`);
console.log();

// Build transaction
const tx = new Transaction();
tx.moveCall({
  target: `${packageId}::nft::mint`,
  arguments: [tx.pure.string(name), tx.pure.string(desc), tx.pure.string(imageUrl)],
});

const result = await client.signAndExecuteTransaction({
  signer: keypair,
  transaction: tx,
  options: { showEffects: true, showObjectChanges: true },
});

console.log("Minted!");
console.log(`Digest: ${result.digest}`);
console.log(
  `Explorer: https://suiscan.xyz/testnet/tx/${result.digest}`
);

const created = result.objectChanges?.filter((o) => o.type === "created");
if (created?.length) {
  console.log(`\nCreated objects:`);
  for (const obj of created) {
    console.log(`  ${obj.objectType} → ${obj.objectId}`);
  }
}
