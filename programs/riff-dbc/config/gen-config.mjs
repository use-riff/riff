// Builds riff's curve config for Meteora's Dynamic Bonding Curve with
// Meteora's own SDK and checks it with the SDK's validator.
//
//   npm install && npm run gen
//
// writes the Borsh-encoded mainnet ConfigParameters the tests pass to
// `create_config`. The devnet scripts import `riffConfig("devnet")`.
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Connection, PublicKey } from "@solana/web3.js";
import {
  ActivationType,
  BaseFeeMode,
  CollectFeeMode,
  DammV2DynamicFeeMode,
  MigratedCollectFeeMode,
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
// No anti-sniping fee: Meteora only lets a buy in the pool's creating
// transaction skip one when that transaction creates the pool directly, and
// riff's program creates it (its escrow signs as creator). The launcher's
// buy would pay the anti-sniping fee. riff caps the launch buy instead.

// After graduation the DAMM v2 pool charges 1%, in SOL only, earned by the
// locked liquidity (half the artist's side, half riff's).
export const GRADUATED_FEE_BPS = 100;

// Market caps in SOL. Devnet is 100x cheaper so a test coin can graduate
// with devnet SOL.
const MARKET_CAPS = {
  mainnet: { initialMarketCap: 30, migrationMarketCap: 300 },
  devnet: { initialMarketCap: 0.3, migrationMarketCap: 3 },
};

export function riffConfig(network = "mainnet") {
  return buildCurveWithMarketCap({
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
      migrationFeeOption: MigrationFeeOption.Customizable,
      migrationFee: { feePercentage: 0, creatorFeePercentage: 0 },
      migratedPoolFee: {
        collectFeeMode: MigratedCollectFeeMode.QuoteToken,
        dynamicFee: DammV2DynamicFeeMode.Disabled,
        poolFeeBps: GRADUATED_FEE_BPS,
      },
    },
    // All liquidity is locked for good at graduation, split between the
    // artist's side (creator: riff's escrow) and riff's program (partner).
    liquidityDistribution: {
      partnerPermanentLockedLiquidityPercentage: 50,
      partnerLiquidityPercentage: 0,
      creatorPermanentLockedLiquidityPercentage: 50,
      creatorLiquidityPercentage: 0,
    },
    lockedVesting: { totalLockedVestingAmount: 0, numberOfVestingPeriod: 0, cliffUnlockAmount: 0, totalVestingDuration: 0, cliffDurationFromMigrationTime: 0 },
    activationType: ActivationType.Timestamp,
    ...MARKET_CAPS[network],
  });
}

// The leftover receiver is an account of create_config, not part of these
// parameters; the validator only needs a real (non-default) key for its check.
export function validate(params) {
  validateConfigParameters({ ...params, leftoverReceiver: new PublicKey("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN") });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  for (const network of Object.keys(MARKET_CAPS)) {
    const params = riffConfig(network);
    validate(params);
    console.log(`${network}: migration threshold ${params.migrationQuoteThreshold.toString()} lamports`);
  }
  // Only the coder is used; nothing connects.
  const program = createDbcProgram(new Connection("http://127.0.0.1:8899"));
  const data = program.coder.instruction.encode("createConfig", { configParameters: riffConfig("mainnet") });
  // Drop the 8-byte instruction discriminator: the tests add their own.
  writeFileSync(new URL("../tests/fixtures/config_parameters.bin", import.meta.url), data.subarray(8));
  console.log(`wrote ${data.length - 8} bytes`);
}
