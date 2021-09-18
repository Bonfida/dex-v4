import { Keypair, PublicKey, Connection, SystemProgram } from "@solana/web3.js";
import { DEX_ID, SRM_MINT } from "./ids";
import {
  cancelOrderInstruction,
  consumeEventInstruction,
  createMarketInstruction,
  initializeAccountInstruction,
  newOrderInstruction,
  settleInstruction,
  closeAccountIntruction,
} from "./instructions";
import { OrderType, PrimedTransaction, Side } from "./types";
import * as aaob from "@bonfida/aaob";
import BN from "bn.js";
import {
  createAssociatedTokenAccount,
  findAssociatedTokenAddress,
} from "./utils";
import { SelfTradeBehavior } from "./state";
import { Market } from "./market";

/**
 * Constants
 */
const MARKET_STATE_SPACE =
  1 + 1 + 32 + 32 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8;
const NODE_CAPACITY = 100;
const EVENT_CAPACITY = 100;

export const createMarket = async (
  connection: Connection,
  baseMint: PublicKey,
  quoteMint: PublicKey,
  minBaseOrderSize: number,
  feePayer: PublicKey,
  marketAdmin: PublicKey
): Promise<PrimedTransaction[]> => {
  // Market Account
  const marketAccount = new Keypair();
  console.log(`Market address ${marketAccount.publicKey.toBase58()}`);
  const balance = await connection.getMinimumBalanceForRentExemption(
    MARKET_STATE_SPACE
  );
  const createMarketAccount = SystemProgram.createAccount({
    fromPubkey: feePayer,
    lamports: balance,
    newAccountPubkey: marketAccount.publicKey,
    programId: DEX_ID,
    space: MARKET_STATE_SPACE,
  });

  // Market signer
  const [marketSigner, marketSignerNonce] = await PublicKey.findProgramAddress(
    [marketAccount.publicKey.toBuffer()],
    DEX_ID
  );

  // AAOB instructions
  const [aaobSigners, aaobInstructions] = await aaob.createMarket(
    connection,
    marketSigner,
    new BN(33),
    new BN(32),
    EVENT_CAPACITY,
    NODE_CAPACITY,
    feePayer
  );

  // Base vault
  const createBaseVault = await createAssociatedTokenAccount(
    feePayer,
    marketSigner,
    baseMint
  );

  // Quote vault
  const createQuoteVault = await createAssociatedTokenAccount(
    feePayer,
    marketSigner,
    quoteMint
  );

  const createMarket = new createMarketInstruction({
    signerNonce: marketSignerNonce,
    minBaseOrderSize: new BN(minBaseOrderSize),
  }).getInstruction(
    DEX_ID,
    marketAccount.publicKey,
    aaobSigners[3].publicKey,
    await findAssociatedTokenAddress(marketSigner, baseMint),
    await findAssociatedTokenAddress(marketSigner, quoteMint),
    aaob.AAOB_ID,
    marketAdmin
  );

  return [
    [[marketAccount], [createMarketAccount]],
    [aaobSigners, aaobInstructions],
    [[], [createBaseVault, createQuoteVault, createMarket]],
  ];
};

export const placeOrder = async (
  market: Market,
  side: Side,
  limitPrice: number,
  size: number,
  type: OrderType,
  selfTradeBehaviour: SelfTradeBehavior,
  ownerTokenAccount: PublicKey,
  owner: PublicKey,
  discountTokenAccount?: PublicKey
) => {
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    DEX_ID
  );

  const [userAccount] = await PublicKey.findProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  // Uncomment for mainnet
  // if (!discountTokenAccount) {
  //   discountTokenAccount = await findAssociatedTokenAddress(owner, SRM_MINT);
  // }

  const instruction = new newOrderInstruction({
    side: side as number,
    limitPrice: new BN(limitPrice),
    maxBaseQty: new BN(size),
    maxQuoteQty: new BN(Math.ceil(size / limitPrice)),
    orderType: type,
    selfTradeBehaviour: selfTradeBehaviour,
    matchLimit: new BN(Number.MAX_SAFE_INTEGER), // TODO Change
  }).getInstruction(
    DEX_ID,
    aaob.AAOB_ID,
    market.address,
    marketSigner,
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

export const cancelOrder = async (
  market: Market,
  orderIndex: BN,
  owner: PublicKey
) => {
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    DEX_ID
  );

  const [userAccount] = await PublicKey.findProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  const instruction = new cancelOrderInstruction({ orderIndex }).getInstruction(
    DEX_ID,
    aaob.AAOB_ID,
    market.address,
    marketSigner,
    market.orderbookAddress,
    market.eventQueueAddress,
    market.bidsAddress,
    market.asksAddress,
    userAccount,
    owner
  );

  return instruction;
};

export const initializeAccount = async (
  market: PublicKey,
  owner: PublicKey,
  maxOrders = 20
) => {
  const [userAccount] = await PublicKey.findProgramAddress(
    [market.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  const instruction = new initializeAccountInstruction({
    market: market.toBuffer(),
    maxOrders: new BN(maxOrders),
  }).getInstruction(DEX_ID, userAccount, owner, owner);

  return instruction;
};

export const settle = async (
  market: Market,
  owner: PublicKey,
  destinationBaseAccount: PublicKey,
  destinationQuoteAccount: PublicKey
) => {
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    DEX_ID
  );

  const [userAccount] = await PublicKey.findProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  const instruction = new settleInstruction().getInstruction(
    DEX_ID,
    aaob.AAOB_ID,
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

export const comsumEvents = async (
  market: Market,
  rewardTarget: PublicKey,
  msrmTokenAccount: PublicKey,
  msrmTokenAccountOwner: PublicKey,
  userAccounts: PublicKey[],
  maxIteration: BN
) => {
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    DEX_ID
  );

  const instruction = new consumeEventInstruction({
    maxIteration,
  }).getInstruction(
    DEX_ID,
    aaob.AAOB_ID,
    market.address,
    marketSigner,
    market.orderbookAddress,
    market.eventQueueAddress,
    rewardTarget,
    msrmTokenAccount,
    msrmTokenAccountOwner,
    userAccounts
  );

  return instruction;
};

export const closeAccount = async (market: PublicKey, owner: PublicKey) => {
  const [userAccount] = await PublicKey.findProgramAddress(
    [market.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  const instruction = new closeAccountIntruction().getInstruction(
    DEX_ID,
    userAccount,
    owner,
    owner
  );

  return instruction;
};
