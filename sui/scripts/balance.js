#!/usr/bin/env node
// Check Sui wallet balance
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { execSync } from "child_process";

const network = process.argv[2] || "testnet";
const client = new SuiClient({ url: getFullnodeUrl(network) });

// Get active address from sui CLI
const address = execSync("sui client active-address", {
  encoding: "utf-8",
}).trim();

const balance = await client.getBalance({ owner: address });
const sui = Number(balance.totalBalance) / 1e9;

console.log(`Network:  ${network}`);
console.log(`Address:  ${address}`);
console.log(`Balance:  ${sui} SUI`);
