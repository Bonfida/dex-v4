import {
  Keypair,
  PublicKey,
  Connection,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { DEX_ID, SRM_MINT } from "./ids";
import {
  cancelOrderInstruction,
  consumeEventsInstruction,
  createMarketInstruction,
  initializeAccountInstruction,
  newOrderInstruction,
  settleInstruction,
  closeAccountInstruction,
  swapInstruction,
  closeMarketInstruction,
  sweepFeesInstruction,
} from "./raw_instructions";
import { OrderType, PrimedTransaction, Side } from "./types";
import * as aaob from "@bonfida/aaob";
import BN from "bn.js";
import { SelfTradeBehavior } from "./state";
import { Market } from "./market";
import {
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import crypto from "crypto";
import { getMetadataKeyFromMint } from "./metadata";
import { Metadata } from "@metaplex-foundation/mpl-token-metadata";
import { computeFp32Price } from "./utils";

/**
 * Constants
 */
const MARKET_STATE_SPACE = 280;
const NODE_CAPACITY = 100;
const EVENT_CAPACITY = 100;
const U64_MAX = "18446744073709551615";

/**
 *
 * @param connection The Solana RPC connection
 * @param baseMint The mint of the base token
 * @param quoteMint The mint of the quote token
 * @param minBaseOrderSize The minimum base order size
 * @param feePayer The fee payer of the transaction
 * @param marketAdmin The market admin
 * @param tickSize The tick size of the market (FP32)
 * @param crankerReward The cranker rewards (raw amount SOL)
 * @returns
 */
export const createMarket = async (
  connection: Connection,
  baseMint: PublicKey,
  quoteMint: PublicKey,
  minBaseOrderSize: BN,
  feePayer: PublicKey,
  marketAdmin: PublicKey,
  tickSize: BN,
  crankerReward: BN,
  baseCurrencyMultiplier?: BN,
  quoteCurrencyMultiplier?: BN,
  programId = DEX_ID
): Promise<PrimedTransaction[]> => {
  // Metadata account
  const metadataAccount = await getMetadataKeyFromMint(baseMint);

  // Market Account
  const marketAccount = new Keypair();
  console.log(`Market address ${marketAccount.publicKey.toBase58()}`);
  const balance = await connection.getMinimumBalanceForRentExemption(
    MARKET_STATE_SPACE
  );
  if (!baseCurrencyMultiplier) {
    baseCurrencyMultiplier = new BN(1);
  }
  if (!quoteCurrencyMultiplier) {
    quoteCurrencyMultiplier = new BN(1);
  }

  // Adjust tick size
  tickSize = tickSize.mul(baseCurrencyMultiplier).div(quoteCurrencyMultiplier);

  const createMarketAccount = SystemProgram.createAccount({
    fromPubkey: feePayer,
    lamports: balance,
    newAccountPubkey: marketAccount.publicKey,
    programId,
    space: MARKET_STATE_SPACE,
  });

  // Market signer
  const [marketSigner, marketSignerNonce] = await PublicKey.findProgramAddress(
    [marketAccount.publicKey.toBuffer()],
    programId ?? DEX_ID
  );

  // AAOB instructions
  const [aaobSigners, aaobInstructions] = await aaob.createMarket(
    connection,
    marketSigner,
    new BN(33),
    new BN(32),
    EVENT_CAPACITY,
    NODE_CAPACITY,
    new BN(minBaseOrderSize),
    feePayer,
    tickSize,
    crankerReward,
    programId
  );
  // Remove the AOB create_market instruction as it is not needed with lib usage
  aaobInstructions.pop();

  // Base vault
  const createBaseVault = await createAssociatedTokenAccountInstruction(
    feePayer,
    await getAssociatedTokenAddress(baseMint, marketSigner, true),
    marketSigner,
    baseMint
  );

  // Quote vault
  const createQuoteVault = await createAssociatedTokenAccountInstruction(
    feePayer,
    await getAssociatedTokenAddress(quoteMint, marketSigner, true),
    marketSigner,
    quoteMint
  );

  const createMarket = new createMarketInstruction({
    signerNonce: new BN(marketSignerNonce),
    minBaseOrderSize: new BN(minBaseOrderSize),
    tickSize: tickSize,
    crankerReward: new BN(crankerReward),
    baseCurrencyMultiplier,
    quoteCurrencyMultiplier,
  }).getInstruction(
    programId,
    marketAccount.publicKey,
    aaobSigners[3].publicKey,
    await getAssociatedTokenAddress(baseMint, marketSigner, true),
    await getAssociatedTokenAddress(quoteMint, marketSigner, true),
    marketAdmin,
    aaobSigners[0].publicKey,
    aaobSigners[1].publicKey,
    aaobSigners[2].publicKey,
    metadataAccount
  );

  return [
    [[marketAccount], [createMarketAccount]],
    [aaobSigners, aaobInstructions],
    [[], [createBaseVault, createQuoteVault, createMarket]],
  ];
};

/**
 *
 * @param market Market object on which the order is placed
 * @param side The side of the order (Bid or Ask)
 * @param limitPrice The limit price (UI limit price not FP32)
 * @param size The size of the order (raw ammount i.e with decimals)
 * @param type The order type
 * @param selfTradeBehaviour The self trade behavior
 * @param ownerTokenAccount The token account from which token will be debited
 * @param owner The user placing the address
 * @param clientOrderId Optional client order ID
 * @param discountTokenAccount Optional SRM token account
 * @returns
 */
export const placeOrder = async (
  market: Market,
  side: Side,
  limitPrice: number,
  size: number,
  type: OrderType,
  selfTradeBehaviour: SelfTradeBehavior,
  ownerTokenAccount: PublicKey,
  owner: PublicKey,
  clientOrderId?: BN,
  discountTokenAccount?: PublicKey,
  maxBaseQty?: BN,
  maxQuoteQty?: BN
) => {
  const [userAccount] = await PublicKey.findProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    market.programId
  );

  // Uncomment for mainnet
  // if (!discountTokenAccount) {
  //   discountTokenAccount = await findAssociatedTokenAddress(owner, SRM_MINT);
  // }

  if (!clientOrderId) {
    clientOrderId = new BN(crypto.randomBytes(16));
  }

  const priceFp32 = computeFp32Price(market, limitPrice);

  const instruction = new newOrderInstruction({
    side: side as number,
    limitPrice: priceFp32,
    maxBaseQty:
      maxBaseQty || new BN(size * market.baseCurrencyMultiplier.toNumber()),
    maxQuoteQty: maxQuoteQty || new BN(U64_MAX),
    orderType: type,
    selfTradeBehavior: selfTradeBehaviour,
    matchLimit: new BN(Number.MAX_SAFE_INTEGER),
    clientOrderId,
    hasDiscountTokenAccount: discountTokenAccount === undefined ? 0 : 1, // TODO Change
  }).getInstruction(
    market.programId,
    TOKEN_PROGRAM_ID,
    SystemProgram.programId,
    market.address,
    market.orderbookAddress,
    market.eventQueueAddress,
    market.bidsAddress,
    market.asksAddress,
    market.baseVault,
    market.quoteVault,
    userAccount,
    ownerTokenAccount,
    owner,
    discountTokenAccount
  );

  return instruction;
};

/**
 *
 * @param market  Market object on which the order is canceled
 * @param owner The owner of the order being cancelled
 * @param orderId The order ID
 * @param orderIndex The index of the order in the user account orders list
 * @param clientOrderId Optional client order ID
 * @returns
 */
export const cancelOrder = async (
  market: Market,
  owner: PublicKey,
  orderId: BN,
  orderIndex?: BN,
  clientOrderId?: BN
) => {
  const [userAccount] = await PublicKey.findProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    market.programId
  );

  const instruction = new cancelOrderInstruction({
    orderId: clientOrderId ? clientOrderId : orderId,
    orderIndex: orderIndex ? orderIndex : new BN(0),
    isClientId: clientOrderId ? 1 : 0,
  }).getInstruction(
    market.programId,
    market.address,
    market.orderbookAddress,
    market.eventQueueAddress,
    market.bidsAddress,
    market.asksAddress,
    userAccount,
    owner
  );

  return instruction;
};

/**
 *
 * @param market Market object on which the user account is created
 * @param owner The owner of the user account
 * @param maxOrders The max capacity of orders
 * @param feePayer The fee payer of the transaction
 * @returns
 */
export const initializeAccount = async (
  market: PublicKey,
  owner: PublicKey,
  maxOrders = 20,
  feePayer?: PublicKey,
  programId = DEX_ID
) => {
  const [userAccount] = await PublicKey.findProgramAddress(
    [market.toBuffer(), owner.toBuffer()],
    programId
  );

  const instruction = new initializeAccountInstruction({
    market: market.toBuffer(),
    maxOrders: new BN(maxOrders),
  }).getInstruction(
    programId,
    SystemProgram.programId,
    userAccount,
    owner,
    feePayer || owner
  );

  return instruction;
};

/**
 *
 * @param market Market object on which funds are settled
 * @param owner The user settling their funds
 * @param destinationBaseAccount The user base token account
 * @param destinationQuoteAccount The user quote token account
 * @returns
 */
export const settle = async (
  market: Market,
  owner: PublicKey,
  destinationBaseAccount: PublicKey,
  destinationQuoteAccount: PublicKey
) => {
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    market.programId
  );

  const [userAccount] = await PublicKey.findProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    market.programId
  );

  const instruction = new settleInstruction().getInstruction(
    market.programId,
    TOKEN_PROGRAM_ID,
    market.address,
    market.baseVault,
    market.quoteVault,
    marketSigner,
    userAccount,
    owner,
    destinationBaseAccount,
    destinationQuoteAccount
  );

  return instruction;
};

/**
 *
 * @param market Market object on which events are consumed
 * @param rewardTarget The cranker rewards target
 * @param userAccounts The user accounts
 * @param maxIterations
 * @param noOpErr
 * @returns
 */
export const consumeEvents = async (
  market: Market,
  rewardTarget: PublicKey,
  userAccounts: PublicKey[],
  maxIterations: BN,
  noOpErr: BN
) => {
  const instruction = new consumeEventsInstruction({
    maxIterations,
    noOpErr,
  }).getInstruction(
    market.programId,
    market.address,
    market.orderbookAddress,
    market.eventQueueAddress,
    rewardTarget,
    userAccounts
      .map((e) => e.toBuffer())
      .sort(Buffer.compare)
      .map((e) => new PublicKey(e))
  );

  return instruction;
};

export const closeAccount = async (
  market: PublicKey,
  owner: PublicKey,
  programId = DEX_ID
) => {
  const [userAccount] = await PublicKey.findProgramAddress(
    [market.toBuffer(), owner.toBuffer()],
    programId
  );

  const instruction = new closeAccountInstruction().getInstruction(
    programId,
    userAccount,
    owner,
    owner
  );

  return instruction;
};

export const swap = async (
  market: Market,
  side: Side,
  minOutputQuantity: number,
  inputQuantity: number,
  selfTradeBehaviour: SelfTradeBehavior,
  ownerBaseTokenAccount: PublicKey,
  ownerQuoteTokenAccount: PublicKey,
  owner: PublicKey,
  discountTokenAccount?: PublicKey,
  referralFeeAccount?: PublicKey
) => {
  // Market signer
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    market.programId
  );

  // Uncomment for mainnet
  // if (!discountTokenAccount) {
  //   discountTokenAccount = await findAssociatedTokenAddress(owner, SRM_MINT);
  // }

  const instruction = new swapInstruction({
    side: side as number,
    baseQty:
      side === Side.Bid ? new BN(minOutputQuantity) : new BN(inputQuantity),
    quoteQty:
      side === Side.Bid ? new BN(inputQuantity) : new BN(minOutputQuantity),
    matchLimit: new BN(Number.MAX_SAFE_INTEGER), // TODO Change
    hasDiscountTokenAccount: Number(discountTokenAccount !== undefined),
  }).getInstruction(
    market.programId,
    TOKEN_PROGRAM_ID,
    SystemProgram.programId,
    market.address,
    market.orderbookAddress,
    market.eventQueueAddress,
    market.bidsAddress,
    market.asksAddress,
    market.baseVault,
    market.quoteVault,
    marketSigner,
    ownerBaseTokenAccount,
    ownerQuoteTokenAccount,
    owner,
    discountTokenAccount,
    referralFeeAccount
  );

  return instruction;
};

export const closeMarket = async (market: Market, target: PublicKey) => {
  // Market signer
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    market.programId
  );

  const ix = new closeMarketInstruction().getInstruction(
    market.programId,
    market.address,
    market.baseVault,
    market.quoteVault,
    market.orderbookAddress,
    market.eventQueueAddress,
    market.bidsAddress,
    market.asksAddress,
    market.marketAdmin,
    target,
    marketSigner,
    TOKEN_PROGRAM_ID
  );

  return ix;
};

export const sweepFees = async (
  connection: Connection,
  market: Market,
  destination: PublicKey,
  feePayer: PublicKey
) => {
  // Market signer
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    market.programId
  );
  // Metadata account
  const creatorTokenAccounts: PublicKey[] = [];
  const tokenIxs: TransactionInstruction[] = [];
  const metadataAccount = await getMetadataKeyFromMint(market.baseMintAddress);
  const info = await connection.getAccountInfo(metadataAccount);

  if (!!info?.data) {
    const [metadata] = Metadata.fromAccountInfo(info);
    const creators = metadata?.data?.creators;
    if (creators) {
      for (let c of creators) {
        const key = await getAssociatedTokenAddress(
          market.quoteMintAddress,
          c.address
        );
        const info = await connection.getAccountInfo(key);
        // Create token acc if does not exist
        if (!info?.data) {
          tokenIxs.push(
            createAssociatedTokenAccountInstruction(
              feePayer,
              key,
              c.address,
              market.quoteMintAddress
            )
          );
        }
        creatorTokenAccounts.push(key);
      }
    }
  }

  const ix = new sweepFeesInstruction().getInstruction(
    market.programId,
    market.address,
    marketSigner,
    market.quoteVault,
    destination,
    TOKEN_PROGRAM_ID,
    metadataAccount,
    creatorTokenAccounts
  );

  return [...tokenIxs, ix];
};
