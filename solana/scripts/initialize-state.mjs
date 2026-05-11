/**
 * One-shot init script: calls `initialize_state` on the deployed program so
 * subsequent `new_agent` / `create_campaign` calls have a counter to read.
 *
 * Usage:
 *   node scripts/initialize-state.mjs
 *
 * Reads the deployer keypair from ~/.config/solana/apex-deployer.json and
 * sends a transaction to devnet.
 */

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

const PROGRAM_ID = new PublicKey("3YKNvs1ZizwFzbraboHsxAeLSoKx4UFDwxkuNXqMkEX5");
const RPC = "https://api.devnet.solana.com";
const REGISTRATION_FEE_LAMPORTS = 5_000_000n; // 0.005 SOL

// Anchor instruction discriminator = first 8 bytes of sha256("global:initialize_state")
import { createHash } from "node:crypto";
const disc = createHash("sha256").update("global:initialize_state").digest().subarray(0, 8);

// Args: u64 little-endian
const args = Buffer.alloc(8);
args.writeBigUInt64LE(REGISTRATION_FEE_LAMPORTS);
const data = Buffer.concat([disc, args]);

const keyPath = path.join(os.homedir(), ".config/solana/apex-deployer.json");
const secret = JSON.parse(fs.readFileSync(keyPath, "utf8"));
const deployer = Keypair.fromSecretKey(Uint8Array.from(secret));
console.log("Deployer:", deployer.publicKey.toBase58());

const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("state")],
  PROGRAM_ID,
);
console.log("GlobalState PDA:", statePda.toBase58());

const ix = new TransactionInstruction({
  programId: PROGRAM_ID,
  keys: [
    { pubkey: deployer.publicKey, isSigner: true, isWritable: true },
    { pubkey: statePda, isSigner: false, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ],
  data,
});

const connection = new Connection(RPC, "confirmed");
const existing = await connection.getAccountInfo(statePda);
if (existing) {
  console.log("GlobalState already initialized — nothing to do.");
  process.exit(0);
}

const tx = new Transaction().add(ix);
const sig = await sendAndConfirmTransaction(connection, tx, [deployer]);
console.log("✓ initialized");
console.log("Signature:", sig);
console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
