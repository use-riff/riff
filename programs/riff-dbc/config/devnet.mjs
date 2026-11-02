// riff-dbc on devnet: one-time setup, then an end-to-end run of a coin's
// whole life against Meteora's real devnet programs.
//
//   node devnet.mjs setup   # riff's Meteora config + riff-dbc's config (once)
//   node devnet.mjs e2e     # launch, trade, collect, claim, graduate, LP fees
//                           # (--no-claim: without the passport steps)
//   node devnet.mjs status  # what's deployed
//                           # (the claim steps issue a test passport, then reset it)
//   node devnet.mjs reset-passport  # if an e2e stopped before its reset
//
// Keys come from ~/.config/riff/devnet (never the repo): deployer.json is
// riff-dbc's upgrade authority and pays; verifier.json co-signs claims;
// rehearsal/e2e-trader.json is the wallet of devnet passport spotify:0RiffPassportDemo00001.
// RPC: ~/.config/riff/devnet/rpc-url if present, else the public devnet RPC.
import { createHash, webcrypto } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import anchor from "@coral-xyz/anchor";
import BN from "bn.js";
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  NATIVE_MINT,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createSyncNativeInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { DynamicBondingCurveClient, createDbcProgram } from "@meteora-ag/dynamic-bonding-curve-sdk";
import { riffConfig, validate } from "./gen-config.mjs";

const KEYS = `${homedir()}/.config/riff/devnet`;
const RPC = existsSync(`${KEYS}/rpc-url`) ? readFileSync(`${KEYS}/rpc-url`, "utf8").trim() : "https://api.devnet.solana.com";
const IDL = JSON.parse(readFileSync(new URL("../../../idl/riff_dbc.json", import.meta.url)));
const PASSPORT_IDL = JSON.parse(readFileSync(new URL("../../../idl/riff_passport.json", import.meta.url)));
const CP_AMM_IDL = JSON.parse(readFileSync(new URL("../../../idls/cp_amm.json", import.meta.url)));

const RIFF_DBC = new PublicKey(IDL.address);
const DBC = new PublicKey("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const DAMM = new PublicKey("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
const PASSPORT = new PublicKey("2nke6euXvAnbtdtcbbk8N2Z3SRLuR7i67VYmSdcY5kwj");
const DBC_POOL_AUTHORITY = new PublicKey("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
const DAMM_POOL_AUTHORITY = new PublicKey("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");
/// Meteora's DAMM v2 config for DBC pools with a customizable migrated fee.
const DAMM_CONFIG = new PublicKey("A8gMrEPJkacWkcb3DGwtJwTe16HktSEfvwtuDh2MCtck");

const ARTIST_ID = "spotify:0RiffPassportDemo00001";
// Devnet settings: 0.3 → 3 SOL market cap (gen-config.mjs), a 0.01 SOL
// launch buy cap, a 1-hour claim window.
const LAUNCHER_SHARE_BPS = 3750;
const MAX_LAUNCH_BUY = 0.01 * LAMPORTS_PER_SOL;
const CLAIM_WINDOW_SECS = 60 * 60;

const key = (name) => Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(`${KEYS}/${name}`))));
const connection = new Connection(RPC, "confirmed");
const deployer = key("deployer.json");
const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(deployer), { commitment: "confirmed" });
const riff = new anchor.Program(IDL, provider);
const dbc = createDbcProgram(connection);
const damm = new anchor.Program(CP_AMM_IDL, provider);
const passportProgram = new anchor.Program(PASSPORT_IDL, provider);
const meteora = new DynamicBondingCurveClient(connection, "confirmed");

const pda = (seeds, program = RIFF_DBC) => PublicKey.findProgramAddressSync(seeds, program)[0];
const b = (s) => Buffer.from(s);
const configPda = pda([b("config")]);
const feeAuthority = pda([b("fee_authority")]);
const feesVault = pda([b("fees_vault")]);
const dbcEventAuthority = pda([b("__event_authority")], DBC);
const dammEventAuthority = pda([b("__event_authority")], DAMM);
const wsolOf = (owner) => getAssociatedTokenAddressSync(NATIVE_MINT, owner, true);
const sol = (lamports) => (Number(lamports) / LAMPORTS_PER_SOL).toFixed(6);
const sorted = (a, c) => (Buffer.compare(a.toBuffer(), c.toBuffer()) > 0 ? [a, c] : [c, a]);
const link = (sig) => `https://explorer.solana.com/tx/${sig}?cluster=devnet`;

async function send(ixs, signers, what) {
  const tx = new Transaction().add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ...ixs);
  tx.feePayer = signers[0].publicKey;
  const sig = await provider.connection.sendTransaction(tx, signers);
  const { value } = await connection.confirmTransaction(sig, "confirmed");
  if (value.err) throw new Error(`${what} failed: ${JSON.stringify(value.err)} ${link(sig)}`);
  console.log(`  ${what}: ${link(sig)}`);
  return sig;
}

const wrap = (owner, lamports) => [
  createAssociatedTokenAccountIdempotentInstruction(deployer.publicKey, wsolOf(owner), owner, NATIVE_MINT),
  ...(lamports > 0
    ? [SystemProgram.transfer({ fromPubkey: owner, toPubkey: wsolOf(owner), lamports }), createSyncNativeInstruction(wsolOf(owner))]
    : []),
];

const tokenAmount = async (account) => {
  const info = await connection.getAccountInfo(account);
  return info ? info.data.readBigUInt64LE(64) : 0n;
};

// ------------------------------------------------------------ setup

function dbcConfigKey() {
  const path = `${KEYS}/riff-dbc-meteora-config.json`;
  if (!existsSync(path)) writeFileSync(path, JSON.stringify(Array.from(Keypair.generate().secretKey)), { mode: 0o600 });
  return key("riff-dbc-meteora-config.json");
}

async function setup() {
  const [verifier, treasury, charity] = ["verifier.json", "treasury.json", "charity.json"].map((f) => key(f).publicKey);
  const dbcConfig = dbcConfigKey();
  if (!(await connection.getAccountInfo(RIFF_DBC))) throw new Error(`riff-dbc isn't deployed at ${RIFF_DBC}: run make deploy-dbc-devnet`);

  if (!(await connection.getAccountInfo(dbcConfig.publicKey))) {
    const params = riffConfig("devnet");
    validate(params);
    const tx = await meteora.partner.createConfig({
      config: dbcConfig.publicKey,
      feeClaimer: feeAuthority,
      leftoverReceiver: treasury,
      payer: deployer.publicKey,
      quoteMint: NATIVE_MINT,
      ...params,
    });
    await send(tx.instructions, [deployer, dbcConfig], "Meteora config");
  }
  console.log(`riff's Meteora config: ${dbcConfig.publicKey}`);

  if (!(await connection.getAccountInfo(configPda))) {
    const programData = pda([RIFF_DBC.toBuffer()], new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111"));
    const ix = await riff.methods
      .initialize({
        verifier,
        charity,
        treasury,
        dbcConfig: dbcConfig.publicKey,
        launcherShareBps: LAUNCHER_SHARE_BPS,
        maxLaunchBuy: new BN(MAX_LAUNCH_BUY),
        claimWindowSecs: new BN(CLAIM_WINDOW_SECS),
      })
      .accountsStrict({
        admin: deployer.publicKey,
        config: configPda,
        feeAuthority,
        feesVault,
        quoteMint: NATIVE_MINT,
        program: RIFF_DBC,
        programData,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    await send([ix], [deployer], "riff-dbc config");
  }
  console.log(`riff-dbc config: ${configPda}`);
}

// ------------------------------------------------------------ passport

const passportPda = pda([b("passport"), b(ARTIST_ID)], PASSPORT);
const passportVaultPda = pda([b("vault"), passportPda.toBuffer()], PASSPORT);
const passportConfigPda = pda([b("config")], PASSPORT);
const sha256 = (...parts) => createHash("sha256").update(Buffer.concat(parts)).digest();
const u64le = (n) => {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(n));
  return out;
};

/// The demo passport's passkey: a software P-256 key in place of a browser's.
async function demoPasskey() {
  const path = `${KEYS}/rehearsal/e2e-passkey.jwk`;
  const { subtle } = webcrypto;
  if (!existsSync(path)) {
    const pair = await subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign"]);
    writeFileSync(path, JSON.stringify(await subtle.exportKey("jwk", pair.privateKey)), { mode: 0o600 });
  }
  const jwk = JSON.parse(readFileSync(path));
  const privateKey = await subtle.importKey("jwk", jwk, { name: "ECDSA", namedCurve: "P-256" }, false, ["sign"]);
  const y = Buffer.from(jwk.y, "base64url");
  const passkey = Buffer.concat([Buffer.of(y[31] & 1 ? 3 : 2), Buffer.from(jwk.x, "base64url")]);
  return { passkey, privateKey };
}

/// A WebAuthn assertion over `challenge`, as riffpad.fun's browser would
/// make it, and the secp256r1 precompile instruction that checks it.
async function assertion({ passkey, privateKey }, rpIdHash, challenge) {
  const authenticatorData = Buffer.concat([Buffer.from(rpIdHash), Buffer.of(0x05), Buffer.alloc(4)]); // user present + verified
  const clientDataJson = Buffer.from(
    JSON.stringify({ type: "webauthn.get", challenge: Buffer.from(challenge).toString("base64url"), origin: "https://riffpad.fun" }),
  );
  const message = Buffer.concat([authenticatorData, sha256(clientDataJson)]);
  const sig = Buffer.from(await webcrypto.subtle.sign({ name: "ECDSA", hash: "SHA-256" }, privateKey, message));
  // Solana's precompile wants a low-S signature.
  const N = 0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551n;
  const s = BigInt(`0x${sig.subarray(32).toString("hex")}`);
  if (s > N / 2n) Buffer.from((N - s).toString(16).padStart(64, "0"), "hex").copy(sig, 32);
  const header = Buffer.alloc(16);
  const [keyOff, sigOff, msgOff] = [16, 49, 113];
  header[0] = 1;
  [sigOff, 0xffff, keyOff, 0xffff, msgOff, message.length, 0xffff].forEach((v, i) => header.writeUInt16LE(v, 2 + i * 2));
  const precompile = new anchor.web3.TransactionInstruction({
    programId: new PublicKey("Secp256r1SigVerify1111111111111111111111111"),
    keys: [],
    data: Buffer.concat([header, passkey, sig, message]),
  });
  return { precompile, proof: { authenticatorData, clientDataJson } };
}

/// Closes the test passport (devnet-only `reset_passport`), so it doesn't
/// linger on riffpad.fun. Its vault's SOL goes to the deployer.
async function resetPassport() {
  // Not in the published IDL (that's the mainnet build), so built by hand.
  const ix = new anchor.web3.TransactionInstruction({
    programId: PASSPORT,
    keys: [
      { pubkey: deployer.publicKey, isSigner: true, isWritable: true },
      { pubkey: passportConfigPda, isSigner: false, isWritable: false },
      { pubkey: passportPda, isSigner: false, isWritable: true },
      { pubkey: passportVaultPda, isSigner: false, isWritable: true },
    ],
    data: sha256(b("global:reset_passport")).subarray(0, 8),
  });
  await send([ix], [deployer], "reset the test passport");
}

/// Issues the e2e passport to rehearsal/e2e-trader.json, with two proofs
/// riff's verifier records (devnet: nothing is checked off-chain).
async function issuePassport() {
  if (await connection.getAccountInfo(passportPda)) return console.log(`passport ${ARTIST_ID}: ${passportPda} (already issued)`);
  const wallet = key("rehearsal/e2e-trader.json");
  const verifier = key("verifier.json");
  const config = await passportProgram.account.passportConfig.fetch(passportConfigPda);
  const kinds = [{ spotifyEmail: {} }, { website: {} }];
  const records = [0, 2].map((k) => pda([b("proof"), b(ARTIST_ID), wallet.publicKey.toBuffer(), Buffer.of(k)], PASSPORT));
  const proofIxs = await Promise.all(
    kinds.map((kind, i) =>
      passportProgram.methods
        .recordProof(ARTIST_ID, kind, [...sha256(b(`devnet demo proof ${i}`))])
        .accountsStrict({
          wallet: wallet.publicKey,
          verifier: verifier.publicKey,
          config: passportConfigPda,
          proofRecord: records[i],
          systemProgram: SystemProgram.programId,
        })
        .instruction(),
    ),
  );
  await send(proofIxs, [wallet, verifier], "record_proof x2 (verifier co-signs)");

  const key_ = await demoPasskey();
  const challenge = sha256(b("riff-passport:v1"), passportPda.toBuffer(), u64le(0), Buffer.of(0), key_.passkey);
  const { precompile, proof } = await assertion(key_, config.rpIdHash, challenge);
  const issue = await passportProgram.methods
    .issuePassport(ARTIST_ID, [...key_.passkey], proof)
    .accountsStrict({
      wallet: wallet.publicKey,
      config: passportConfigPda,
      passport: passportPda,
      vault: passportVaultPda,
      instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      systemProgram: SystemProgram.programId,
    })
    .remainingAccounts(records.map((pubkey) => ({ pubkey, isSigner: false, isWritable: false })))
    .instruction();
  await send([precompile, issue], [wallet], "issue_passport (with passkey)");
  console.log(`passport ${ARTIST_ID}: ${passportPda}, vault ${passportVaultPda}`);
}

/// Withdraws from the passport's vault to its wallet: under the daily
/// allowance, so the wallet alone signs. Waits out the passport's probation.
async function withdrawFromVault(lamports) {
  const wallet = key("rehearsal/e2e-trader.json");
  const config = await passportProgram.account.passportConfig.fetch(passportConfigPda);
  const issuedAt = (await passportProgram.account.passport.fetch(passportPda)).issuedAt.toNumber();
  // The chain's clock, not this machine's: they can be minutes apart.
  const chainNow = async () => connection.getBlockTime(await connection.getSlot());
  const settledAt = issuedAt + config.recoveryDelay.toNumber();
  if ((await chainNow()) < settledAt) console.log(`  waiting for the passport's probation to end (${settledAt - (await chainNow())}s on chain)`);
  while ((await chainNow()) < settledAt + 2) await new Promise((r) => setTimeout(r, 10_000));
  const before = await connection.getBalance(wallet.publicKey);
  const ix = await passportProgram.methods
    .withdraw(new BN(lamports), null)
    .accountsStrict({
      wallet: wallet.publicKey,
      config: passportConfigPda,
      passport: passportPda,
      vault: passportVaultPda,
      instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
    })
    .instruction();
  await send([ix], [wallet], `passport withdraw ${sol(lamports)} SOL`);
  console.log(`  wallet +${sol((await connection.getBalance(wallet.publicKey)) - before)} SOL (less the tx fee)`);
}

// ------------------------------------------------------------ e2e

function coinKeys(dbcConfig, mint) {
  const [hi, lo] = sorted(mint, NATIVE_MINT);
  const pool = pda([b("pool"), dbcConfig.toBuffer(), hi.toBuffer(), lo.toBuffer()], DBC);
  const escrow = pda([b("escrow"), mint.toBuffer()]);
  return {
    mint,
    pool,
    escrow,
    coin: pda([b("coin"), mint.toBuffer()]),
    artistVault: pda([b("artist_vault"), mint.toBuffer()]),
    unwrap: pda([b("unwrap"), mint.toBuffer()]),
    escrowBase: getAssociatedTokenAddressSync(mint, escrow, true, TOKEN_2022_PROGRAM_ID),
    baseVault: pda([b("token_vault"), mint.toBuffer(), pool.toBuffer()], DBC),
    quoteVault: pda([b("token_vault"), NATIVE_MINT.toBuffer(), pool.toBuffer()], DBC),
  };
}

function swapIx(dbcConfig, c, trader, lamports, partial = false) {
  const accounts = {
    poolAuthority: DBC_POOL_AUTHORITY,
    config: dbcConfig,
    pool: c.pool,
    inputTokenAccount: wsolOf(trader),
    outputTokenAccount: getAssociatedTokenAddressSync(c.mint, trader, false, TOKEN_2022_PROGRAM_ID),
    baseVault: c.baseVault,
    quoteVault: c.quoteVault,
    baseMint: c.mint,
    quoteMint: NATIVE_MINT,
    payer: trader,
    tokenBaseProgram: TOKEN_2022_PROGRAM_ID,
    tokenQuoteProgram: TOKEN_PROGRAM_ID,
    referralTokenAccount: null,
    eventAuthority: dbcEventAuthority,
    program: DBC,
  };
  return partial
    ? dbc.methods.swap2({ amount0: new BN(lamports), amount1: new BN(0), swapMode: 1 }).accountsStrict(accounts).instruction()
    : dbc.methods.swap({ amountIn: new BN(lamports), minimumAmountOut: new BN(0) }).accountsStrict(accounts).instruction();
}

async function buy(dbcConfig, c, trader, lamports, partial = false) {
  const base = getAssociatedTokenAddressSync(c.mint, trader.publicKey, false, TOKEN_2022_PROGRAM_ID);
  await send(
    [
      ...wrap(trader.publicKey, lamports),
      createAssociatedTokenAccountIdempotentInstruction(trader.publicKey, base, trader.publicKey, c.mint, TOKEN_2022_PROGRAM_ID),
      await swapIx(dbcConfig, c, trader.publicKey, lamports, partial),
    ],
    [trader],
    `buy ${sol(lamports)} SOL${partial ? " (fills to graduation)" : ""}`,
  );
}

const poolAccounts = (dbcConfig, c) => ({
  dbcConfig,
  poolAuthority: DBC_POOL_AUTHORITY,
  pool: c.pool,
  baseVault: c.baseVault,
  quoteVault: c.quoteVault,
  baseMint: c.mint,
  quoteMint: NATIVE_MINT,
  dbcEventAuthority,
  dbcProgram: DBC,
  tokenQuoteProgram: TOKEN_PROGRAM_ID,
  tokenBaseProgram: TOKEN_2022_PROGRAM_ID,
});

async function collectFees(config, c, launcher) {
  const ix = await riff.methods
    .collectFees()
    .accountsStrict({
      config: configPda,
      coin: c.coin,
      escrow: c.escrow,
      feeAuthority,
      feesVault,
      artistVault: c.artistVault,
      escrowBaseAccount: c.escrowBase,
      launcherQuoteAccount: wsolOf(launcher),
      treasuryQuoteAccount: wsolOf(config.treasury),
      ...poolAccounts(config.dbcConfig, c),
    })
    .instruction();
  await send([ix], [deployer], "collect_fees");
}

async function payArtist(c, vault) {
  const ix = await riff.methods
    .payArtist()
    .accountsStrict({
      payer: deployer.publicKey,
      coin: c.coin,
      passportVault: vault,
      escrow: c.escrow,
      artistVault: c.artistVault,
      unwrap: c.unwrap,
      baseMint: c.mint,
      quoteMint: NATIVE_MINT,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
  await send([ix], [deployer], "pay_artist");
}

/// The DAMM v2 position in `dammPool` whose NFT `owner` holds, after
/// migration (whoever ran it: us, or Meteora's keeper). The fee authority
/// holds one in every graduated coin's pool.
async function positionOf(owner, dammPool) {
  const { value } = await connection.getTokenAccountsByOwner(owner, { programId: TOKEN_2022_PROGRAM_ID });
  for (const { account } of value) {
    const mint = new PublicKey(account.data.subarray(0, 32));
    const position = pda([b("position"), mint.toBuffer()], DAMM);
    if (account.data.readBigUInt64LE(64) !== 1n) continue;
    const info = await connection.getAccountInfo(position);
    if (info && dammPool.equals(new PublicKey(info.data.subarray(8, 40))))
      return { position, nftAccount: pda([b("position_nft_account"), mint.toBuffer()], DAMM) };
  }
  throw new Error(`no DAMM v2 position in ${dammPool} held by ${owner}`);
}

async function e2e() {
  const config = await riff.account.config.fetch(configPda);
  const verifier = key("verifier.json");
  const artistWallet = key("rehearsal/e2e-trader.json");
  const passport = passportPda;
  const passportVault = passportVaultPda;
  if (!config.verifier.equals(verifier.publicKey)) throw new Error("verifier.json isn't riff-dbc's verifier");

  // Receiving WSOL accounts: launcher (the deployer here), treasury, charity.
  await send([...wrap(deployer.publicKey, 0), ...wrap(config.treasury, 0), ...wrap(config.charity, 0)], [deployer], "WSOL accounts");

  console.log("\n1. Launch, with the launcher's capped first buy in the same transaction");
  const mintKp = Keypair.generate();
  const c = coinKeys(config.dbcConfig, mintKp.publicKey);
  const create = await riff.methods
    .createCoin({
      name: "Riff Devnet Demo",
      symbol: "RIFFD",
      uri: "https://riffpad.fun/meta/demo.json",
      artistId: ARTIST_ID,
      artistName: "Riff Passport Demo",
    })
    .accountsStrict({
      launcher: deployer.publicKey,
      config: configPda,
      coin: c.coin,
      escrow: c.escrow,
      artistVault: c.artistVault,
      escrowBaseAccount: c.escrowBase,
      baseMint: c.mint,
      quoteMint: NATIVE_MINT,
      dbcConfig: config.dbcConfig,
      poolAuthority: DBC_POOL_AUTHORITY,
      pool: c.pool,
      baseVault: c.baseVault,
      quoteVault: c.quoteVault,
      dbcEventAuthority,
      instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      dbcProgram: DBC,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
  const launcherBase = getAssociatedTokenAddressSync(c.mint, deployer.publicKey, false, TOKEN_2022_PROGRAM_ID);
  await send(
    [
      create,
      ...wrap(deployer.publicKey, MAX_LAUNCH_BUY),
      createAssociatedTokenAccountIdempotentInstruction(deployer.publicKey, launcherBase, deployer.publicKey, c.mint, TOKEN_2022_PROGRAM_ID),
      await swapIx(config.dbcConfig, c, deployer.publicKey, MAX_LAUNCH_BUY),
    ],
    [deployer, mintKp],
    "create_coin + launch buy",
  );
  console.log(`  coin ${c.mint}, pool ${c.pool}`);
  console.log(`  launcher holds ${(await tokenAmount(launcherBase)) / 1_000_000n} tokens`);

  console.log("\n2. A trade straight on Meteora, then the fee crank");
  await buy(config.dbcConfig, c, deployer, 0.1 * LAMPORTS_PER_SOL);
  await collectFees(config, c, deployer.publicKey);
  console.log(`  artist share held: ${sol(await tokenAmount(c.artistVault))} SOL`);

  const claiming = !process.argv.includes("--no-claim");
  if (claiming) {
  console.log("\n3. The artist claims with their passport");
  await issuePassport();
  const vaultBefore = await connection.getBalance(passportVault);
  const claim = await riff.methods
    .claimArtist()
    .accountsStrict({
      wallet: artistWallet.publicKey,
      verifier: verifier.publicKey,
      config: configPda,
      coin: c.coin,
      passport,
      passportVault,
      escrow: c.escrow,
      artistVault: c.artistVault,
      unwrap: c.unwrap,
      escrowBaseAccount: c.escrowBase,
      charityQuoteAccount: wsolOf(config.charity),
      ...poolAccounts(config.dbcConfig, c),
      systemProgram: SystemProgram.programId,
    })
    .instruction();
  await send([claim], [artistWallet, verifier], "claim_artist");
  console.log(`  passport vault +${sol((await connection.getBalance(passportVault)) - vaultBefore)} SOL`);
  }

  console.log("\n4. Graduation: buy out the curve, migrate to DAMM v2");
  await buy(config.dbcConfig, c, deployer, 0.8 * LAMPORTS_PER_SOL, true);
  const dammPool = pda([b("pool"), DAMM_CONFIG.toBuffer(), ...sorted(c.mint, NATIVE_MINT).map((k) => k.toBuffer())], DAMM);
  if (!(await connection.getAccountInfo(dammPool))) {
    const { transaction, firstPositionNftKeypair, secondPositionNftKeypair } = await meteora.migration.migrateToDammV2({
      pool: c.pool,
      dammConfig: DAMM_CONFIG,
      payer: deployer.publicKey,
    });
    await send(transaction.instructions.filter((ix) => !ix.programId.equals(ComputeBudgetProgram.programId)), [deployer, firstPositionNftKeypair, secondPositionNftKeypair], "migrate to DAMM v2");
  }
  const creator = await positionOf(c.escrow, dammPool);
  const partner = await positionOf(feeAuthority, dammPool);
  console.log(`  DAMM v2 pool ${dammPool}; creator position ${creator.position}, partner position ${partner.position}`);

  console.log("\n5. A trade on the graduated pool, LP fees, and the artist's payout");
  const tokenAVault = pda([b("token_vault"), c.mint.toBuffer(), dammPool.toBuffer()], DAMM);
  const tokenBVault = pda([b("token_vault"), NATIVE_MINT.toBuffer(), dammPool.toBuffer()], DAMM);
  const swap = await damm.methods
    .swap({ amountIn: new BN(0.1 * LAMPORTS_PER_SOL), minimumAmountOut: new BN(0) })
    .accountsStrict({
      poolAuthority: DAMM_POOL_AUTHORITY,
      pool: dammPool,
      inputTokenAccount: wsolOf(deployer.publicKey),
      outputTokenAccount: launcherBase,
      tokenAVault,
      tokenBVault,
      tokenAMint: c.mint,
      tokenBMint: NATIVE_MINT,
      payer: deployer.publicKey,
      tokenAProgram: TOKEN_2022_PROGRAM_ID,
      tokenBProgram: TOKEN_PROGRAM_ID,
      referralTokenAccount: null,
      eventAuthority: dammEventAuthority,
      program: DAMM,
    })
    .instruction();
  await send([...wrap(deployer.publicKey, 0.1 * LAMPORTS_PER_SOL), swap], [deployer], "buy 0.1 SOL on DAMM v2");
  const lp = await riff.methods
    .collectLpFees()
    .accountsStrict({
      config: configPda,
      coin: c.coin,
      escrow: c.escrow,
      feeAuthority,
      feesVault,
      artistVault: c.artistVault,
      escrowBaseAccount: c.escrowBase,
      launcherQuoteAccount: wsolOf(deployer.publicKey),
      treasuryQuoteAccount: wsolOf(config.treasury),
      dammPoolAuthority: DAMM_POOL_AUTHORITY,
      dammPool,
      creatorPosition: creator.position,
      creatorPositionNftAccount: creator.nftAccount,
      partnerPosition: partner.position,
      partnerPositionNftAccount: partner.nftAccount,
      tokenAVault,
      tokenBVault,
      baseMint: c.mint,
      quoteMint: NATIVE_MINT,
      dammEventAuthority,
      dammProgram: DAMM,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      tokenBaseProgram: TOKEN_2022_PROGRAM_ID,
    })
    .instruction();
  await send([lp], [deployer], "collect_lp_fees");
  await collectFees(config, c, deployer.publicKey);
  if (claiming) {
    const before = await connection.getBalance(passportVault);
    await payArtist(c, passportVault);
    console.log(`  passport vault +${sol((await connection.getBalance(passportVault)) - before)} SOL`);

    console.log("\n6. The artist withdraws from the passport's vault");
    await withdrawFromVault(Math.min(0.001 * LAMPORTS_PER_SOL, (await connection.getBalance(passportVault)) - (await connection.getMinimumBalanceForRentExemption(8 + 32))));
    await resetPassport();
  } else console.log(`  artist share held: ${sol(await tokenAmount(c.artistVault))} SOL`);

  const record = await riff.account.coin.fetch(c.coin);
  console.log(`\nDone. Coin ${c.mint}: artist fees ${sol(record.artistFeesTotal)} SOL, launcher fees ${sol(record.launcherFeesTotal)} SOL, artist ${record.artist}`);
}

async function status() {
  console.log(`RPC ${RPC.replace(/api-key=.*/, "api-key=…")}`);
  console.log(`riff-dbc ${RIFF_DBC}: ${(await connection.getAccountInfo(RIFF_DBC)) ? "deployed" : "not deployed"}`);
  console.log(`deployer ${deployer.publicKey}: ${sol(await connection.getBalance(deployer.publicKey))} SOL`);
  const info = await connection.getAccountInfo(configPda);
  if (info) console.log(JSON.stringify(await riff.account.config.fetch(configPda), (k, v) => (v?.toBase58 ? v.toBase58() : v), 2));
  else console.log("riff-dbc config: not initialized");
}

const commands = { setup, e2e, status, "reset-passport": resetPassport };
const command = commands[process.argv[2]];
if (!command) {
  console.error(`usage: node devnet.mjs ${Object.keys(commands).join("|")}`);
  process.exit(1);
}
await command();
// The connection keeps a websocket open after confirming; end the process.
process.exit(0);
