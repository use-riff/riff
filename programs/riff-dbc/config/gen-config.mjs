// Builds riff's curve config for Meteora's Dynamic Bonding Curve with
// Meteora's own SDK, checks it with the SDK's validator, and writes the
// Borsh-encoded ConfigParameters the tests pass to `create_config`.
//
//   npm install && npm run gen
//
// Prototype numbers: they show the mechanics and will be tuned before mainnet.
import { writeFileSync } from "node:fs";
import { Connection, PublicKey } from "@solana/web3.js";
import {
  ActivationType,
  BaseFeeMode,
  CollectFeeMode,
  MigrationFeeOption,
  MigrationOption,
  TokenAuthorityOption,
  TokenDecimal,
  TokenType,
  buildCurveWithMarketCap,
  createDbcProgram,
  validateConfigParameters,
} from "@meteora-ag/dynamic-bonding-curve-sdk";

// 2% per trade. Meteora keeps 20% (0.4%); of the 1.6% left, half is the pool
// creator's (the artist's share, 0.8%) and half the partner's (riff's
// program splits it: 0.3% for whoever launched the coin, 0.5% for riff).
export const TRADE_FEE_BPS = 200;
export const CREATOR_SHARE_PERCENT = 50;

export const params = buildCurveWithMarketCap({
  token: {
    tokenType: TokenType.Token2022,
    tokenBaseDecimal: TokenDecimal.SIX,
    tokenQuoteDecimal: 9,
    tokenAuthorityOption: TokenAuthorityOption.Immutable,
    totalTokenSupply: 1_000_000_000,
    leftover: 0,
  },
  fee: {
    baseFeeParams: {
      baseFeeMode: BaseFeeMode.FeeSchedulerLinear,
      feeSchedulerParam: { startingFeeBps: TRADE_FEE_BPS, endingFeeBps: TRADE_FEE_BPS, numberOfPeriod: 0, totalDuration: 0 },
    },
    dynamicFeeEnabled: false,
    collectFeeMode: CollectFeeMode.QuoteToken,
    creatorTradingFeePercentage: CREATOR_SHARE_PERCENT,
    poolCreationFee: 0,
    enableFirstSwapWithMinFee: false,
  },
  migration: {
    migrationOption: MigrationOption.MET_DAMM_V2,
    // After graduation the DAMM v2 pool charges 1%, earned by the locked liquidity.
    migrationFeeOption: MigrationFeeOption.FixedBps100,
    migrationFee: { feePercentage: 0, creatorFeePercentage: 0 },
  },
  // All liquidity is locked for good at graduation, split between the
  // artist's side (creator) and riff's program (partner).
  liquidityDistribution: {
    partnerPermanentLockedLiquidityPercentage: 50,
    partnerLiquidityPercentage: 0,
    creatorPermanentLockedLiquidityPercentage: 50,
    creatorLiquidityPercentage: 0,
  },
  lockedVesting: { totalLockedVestingAmount: 0, numberOfVestingPeriod: 0, cliffUnlockAmount: 0, totalVestingDuration: 0, cliffDurationFromMigrationTime: 0 },
  activationType: ActivationType.Timestamp,
  initialMarketCap: 30,
  migrationMarketCap: 300,
});

// The leftover receiver is an account of create_config, not part of these
// parameters; the validator only needs a real (non-default) key for its check.
validateConfigParameters({ ...params, leftoverReceiver: new PublicKey("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN") });

// Only the coder is used; nothing connects.
const { program } = createDbcProgram(new Connection("http://127.0.0.1:8899"));
const data = program.coder.instruction.encode("createConfig", { configParameters: params });
// Drop the 8-byte instruction discriminator: the tests add their own.
writeFileSync(new URL("../tests/fixtures/config_parameters.bin", import.meta.url), data.subarray(8));
console.log(`wrote ${data.length - 8} bytes; migration threshold ${params.migrationQuoteThreshold.toString()} lamports`);
