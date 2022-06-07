import { TokenMint, signAndSendInstructions } from "@bonfida/utils";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { createMarket } from "../src/bindings";
import BN from "bn.js";
import { expect } from "@jest/globals";
import { MarketState, AccountTag, MarketFeeType } from "../src/state";
import { DEX_ID } from "../src/ids";
import { AccountLayout, TOKEN_PROGRAM_ID } from "@solana/spl-token";

export const createMarketTest = async (
  connection: Connection,
  feePayer: Keypair
) => {
  /**
   * Base and quote
   */

  const base = await TokenMint.init(connection, feePayer);
  const quote = await TokenMint.init(connection, feePayer);

  const metadata = Keypair.generate().publicKey;

  const tickSize = new BN(1);
  const minBaseOrderSize = 1;

  let ixs = await createMarket(
    connection,
    base.token,
    quote.token,
    minBaseOrderSize,
    feePayer.publicKey,
    feePayer.publicKey,
    tickSize,
    new BN(0),
    // Metadata not checked on devnet
    metadata
  );

  for (let ix of ixs) {
    let tx = await signAndSendInstructions(connection, ix[0], feePayer, ix[1]);
    console.log(tx);
  }

  const marketKey = ixs[0][0][0].publicKey;
  console.log("marketKey", marketKey.toBase58());
  let marketObj = await MarketState.retrieve(connection, marketKey);
  const now = new Date().getTime() / 1_000;

  /**
   * Verify market state
   */

  expect(marketObj.tag).toBe(AccountTag.MarketState);
  expect(marketObj.baseMint.toBase58()).toBe(base.token.toBase58());
  expect(marketObj.quoteMint.toBase58()).toBe(quote.token.toBase58());
  expect(marketObj.quoteMint.toBase58()).toBe(quote.token.toBase58());
  expect(marketObj.admin.toBase58()).toBe(feePayer.publicKey.toBase58());
  expect(marketObj.creationTimestamp.toNumber()).toBeLessThanOrEqual(now);
  expect(marketObj.baseVolume.toNumber()).toBe(0);
  expect(marketObj.quoteVolume.toNumber()).toBe(0);
  expect(marketObj.accumulatedFees.toNumber()).toBe(0);
  expect(marketObj.minBaseOrderSize.toNumber()).toBe(minBaseOrderSize);
  expect(marketObj.royaltiesBps.toNumber()).toBe(0);
  expect(marketObj.accumulatedRoyalties.toNumber()).toBe(0);
  expect(marketObj.feeType).toBe(MarketFeeType.Default);

  const [marketSigner] = await PublicKey.findProgramAddress(
    [marketKey.toBuffer()],
    DEX_ID
  );

  /**
   * Verify vaults
   */

  const baseVaultRaw = await connection.getAccountInfo(marketObj.baseVault);

  if (!baseVaultRaw) {
    throw new Error("Cannot retrieve base vault");
  }

  const baseVault = AccountLayout.decode(baseVaultRaw.data);

  expect(baseVaultRaw.owner.toBase58()).toBe(TOKEN_PROGRAM_ID.toBase58());
  expect(baseVault.amount.toString()).toBe("0");
  expect(baseVault.owner.toBase58()).toBe(marketSigner.toBase58());
  expect(baseVault.mint.toBase58()).toBe(base.token.toBase58());

  const quoteVaultRaw = await connection.getAccountInfo(marketObj.quoteVault);

  if (!quoteVaultRaw) {
    throw new Error("Cannot retrieve quote vault");
  }

  const quoteVault = AccountLayout.decode(quoteVaultRaw.data);

  expect(quoteVaultRaw.owner.toBase58()).toBe(TOKEN_PROGRAM_ID.toBase58());
  expect(quoteVault.amount.toString()).toBe("0");
  expect(quoteVault.owner.toBase58()).toBe(marketSigner.toBase58());
  expect(quoteVault.mint.toBase58()).toBe(quote.token.toBase58());
};
