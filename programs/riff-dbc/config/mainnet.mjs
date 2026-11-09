// riff-dbc's mainnet setup. Dry run unless --yes: it simulates each transaction
// and sends nothing. Only run with --yes after the owner's go-ahead; see
// docs/MAINNET_LAUNCH.md.
//
//   node mainnet.mjs setup    # riff's Meteora config + riff-dbc's config
//   node mainnet.mjs handoff  # riff-dbc's admin → the multisig vault
//   node mainnet.mjs status
//
// Keys come from ~/.config/riff/mainnet (never the repo): deployer.json is the
// program's upgrade authority until the handoff, and pays. The settings are in
// mainnet.params.json; --cluster devnet --params <file> rehearses the dry run.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import anchor from "@coral-xyz/anchor";
import BN from "bn.js";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { NATIVE_MINT, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { DynamicBondingCurveClient } from "@meteora-ag/dynamic-bonding-curve-sdk";
import { SystemProgram } from "@solana/web3.js";
import { riffConfig, validate } from "./gen-config.mjs";

const argValue = (name) => {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : undefined;
};
const cluster = argValue("cluster") ?? "mainnet";
if (cluster !== "mainnet" && cluster !== "devnet") throw new Error("--cluster must be mainnet or devnet");
const send = process.argv.includes("--yes");
if (send && cluster !== "mainnet") throw new Error("--yes is for mainnet: devnet has devnet.mjs");

const KEYS = `${homedir()}/.config/riff/${cluster}`;
const RPC = existsSync(`${KEYS}/rpc-url`) ? readFileSync(`${KEYS}/rpc-url`, "utf8").trim() : "https://api.mainnet-beta.solana.com";
const IDL = JSON.parse(readFileSync(new URL("../../../idl/riff_dbc.json", import.meta.url)));
const params = JSON.parse(readFileSync(argValue("params") ?? new URL("./mainnet.params.json", import.meta.url)));

/// The address in `params`, refusing placeholders.
function address(name) {
  const value = params[name];
  if (typeof value !== "string" || value.startsWith("REPLACE")) throw new Error(`${name} in the params file is still a placeholder.`);
  return new PublicKey(value);
}

const readKey = (path) => Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(path))));
if (!existsSync(`${KEYS}/deployer.json`)) throw new Error(`${KEYS}/deployer.json is missing.`);
const deployer = readKey(`${KEYS}/deployer.json`);
const connection = new Connection(RPC, "confirmed");
const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(deployer), { commitment: "confirmed" });
const riff = new anchor.Program(IDL, provider);
const RIFF_DBC = new PublicKey(IDL.address);
const pda = (seeds) => PublicKey.findProgramAddressSync(seeds, RIFF_DBC)[0];
const b = (s) => Buffer.from(s);
const configPda = pda([b("config")]);
const feeAuthority = pda([b("fee_authority")]);
const feesVault = pda([b("fees_vault")]);
const programData = PublicKey.findProgramAddressSync([RIFF_DBC.toBuffer()], new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111"))[0];

/// Simulates `ixs`; with --yes, sends them after that passes.
async function run(ixs, signers, what) {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = deployer.publicKey;
  tx.recentBlockhash = (await connection.getLatestBlockhash("confirmed")).blockhash;
  tx.sign(...signers);
  const size = tx.serialize().length;
  const sim = await connection.simulateTransaction(tx);
  if (sim.value.err) {
    console.error((sim.value.logs ?? []).join("\n"));
    throw new Error(`${what} fails in simulation: ${JSON.stringify(sim.value.err)}`);
  }
  console.log(`  ${what}: simulation passed, ${sim.value.unitsConsumed} compute units, ${size} bytes`);
  if (!send) return;
  const sig = await connection.sendRawTransaction(tx.serialize());
  const { value } = await connection.confirmTransaction(sig, "confirmed");
  if (value.err) throw new Error(`${what} failed: ${JSON.stringify(value.err)} (${sig})`);
  console.log(`  sent: https://explorer.solana.com/tx/${sig}`);
}

async function setup() {
  const treasury = address("treasury");
  const settings = {
    verifier: address("verifier"),
    charity: address("charity"),
    treasury,
    launcherShareBps: params.launcherShareBps,
    maxLaunchBuy: new BN(params.maxLaunchBuyLamports),
    claimWindowSecs: new BN(params.claimWindowSecs),
  };
  console.log(`cluster ${cluster}, deployer ${deployer.publicKey}: ${(await connection.getBalance(deployer.publicKey)) / 1e9} SOL`);
  for (const [k, v] of Object.entries(settings)) console.log(`  ${k}: ${v}`);

  const program = await connection.getAccountInfo(RIFF_DBC);
  if (!program) throw new Error(`riff-dbc isn't deployed at ${RIFF_DBC}: make deploy-dbc-mainnet first.`);
  const data = await connection.getAccountInfo(programData);
  const authority = data.data[12] === 1 ? new PublicKey(data.data.subarray(13, 45)) : null;
  if (!authority?.equals(deployer.publicKey)) throw new Error(`The deployer isn't riff-dbc's upgrade authority (${authority}).`);

  // The key decides the config's address; a dry run doesn't keep one.
  const keyPath = `${KEYS}/riff-dbc-meteora-config.json`;
  const dbcConfig = existsSync(keyPath) ? readKey(keyPath) : Keypair.generate();
  if (send && !existsSync(keyPath)) writeFileSync(keyPath, JSON.stringify(Array.from(dbcConfig.secretKey)), { mode: 0o600 });
  const network = cluster === "mainnet" ? "mainnet" : "devnet";
  if (await connection.getAccountInfo(dbcConfig.publicKey)) console.log(`riff's Meteora config exists: ${dbcConfig.publicKey}`);
  else {
    const curve = riffConfig(network);
    validate(curve);
    const meteora = new DynamicBondingCurveClient(connection, "confirmed");
    const tx = await meteora.partner.createConfig({
      config: dbcConfig.publicKey,
      feeClaimer: feeAuthority,
      leftoverReceiver: treasury,
      payer: deployer.publicKey,
      quoteMint: NATIVE_MINT,
      ...curve,
    });
    console.log(`riff's Meteora config ${dbcConfig.publicKey}: graduates at ${curve.migrationQuoteThreshold} lamports in the curve`);
    await run(tx.instructions, [deployer, dbcConfig], "create Meteora config");
  }

  if (await connection.getAccountInfo(configPda)) return console.log(`riff-dbc config exists: ${configPda}`);
  // Simulated on its own, so before the Meteora config exists this fails only if it would for another reason.
  const ix = await riff.methods
    .initialize({ ...settings, dbcConfig: dbcConfig.publicKey })
    .accountsStrict({
      admin: deployer.publicKey,
      config: configPda,
      feeAuthority,
      feesVault,
      quoteMint: NATIVE_MINT,
      dbcConfig: dbcConfig.publicKey,
      program: RIFF_DBC,
      programData,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
  await run([ix], [deployer], "initialize riff-dbc");
  console.log(`riff-dbc config: ${configPda}`);
}

/// Hands riff-dbc's admin to the multisig in two steps: the deployer proposes
/// it here, and the multisig accepts in a Squads transaction of its own, so a
/// wrong address never takes over.
async function handoff() {
  const admin = address("admin");
  const config = await riff.account.config.fetch(configPda);
  const pendingAdmin = pda([b("pending_admin")]);
  if (config.admin.equals(admin)) console.log(`The admin is already ${admin}.`);
  else {
    console.log(`Proposing admin ${config.admin} → ${admin}. Nothing changes until ${admin} accepts.`);
    const ix = await riff.methods
      .proposeAdmin(admin)
      .accountsStrict({ admin: deployer.publicKey, config: configPda, pendingAdmin, systemProgram: SystemProgram.programId })
      .instruction();
    await run([ix], [deployer], "propose the admin");
    const accept = await riff.methods
      .acceptAdmin()
      .accountsStrict({ newAdmin: admin, config: configPda, pendingAdmin })
      .instruction();
    console.log(`\nThen the multisig accepts: a Squads transaction from vault ${admin} with this one instruction:`);
    console.log(`  program ${accept.programId}`);
    for (const k of accept.keys) console.log(`  ${k.pubkey} ${k.isSigner ? "signer " : ""}${k.isWritable ? "writable" : "readonly"}`);
    console.log(`  data (base58) ${anchor.utils.bytes.bs58.encode(accept.data)}`);
    console.log(`Check with \`node mainnet.mjs status\` that the admin is ${admin}.`);
  }
  console.log(`\nThen, by hand, after checking the address again:\n  solana program set-upgrade-authority ${RIFF_DBC} --new-upgrade-authority ${admin} --skip-new-upgrade-authority-signer-check --keypair ${KEYS}/deployer.json --url mainnet-beta`);
}

async function status() {
  console.log(`riff-dbc ${RIFF_DBC}: ${(await connection.getAccountInfo(RIFF_DBC)) ? "deployed" : "not deployed"}`);
  const data = await connection.getAccountInfo(programData);
  if (data) console.log(`upgrade authority: ${data.data[12] === 1 ? new PublicKey(data.data.subarray(13, 45)) : "none"}`);
  if (await connection.getAccountInfo(configPda)) console.log(JSON.stringify(await riff.account.config.fetch(configPda), (k, v) => (v?.toBase58 ? v.toBase58() : k.endsWith("Secs") || k === "maxLaunchBuy" ? new BN(v, 16).toString() : v), 2));
  else console.log("riff-dbc config: not initialized");
}

const commands = { setup, handoff, status };
const command = commands[process.argv[2]];
if (!command) {
  console.error(`usage: node mainnet.mjs ${Object.keys(commands).join("|")} [--yes]`);
  process.exit(1);
}
await command();
if (!send && command !== status) console.log("\nDry run: nothing sent. --yes sends, after the owner's go-ahead.");
process.exit(0);
