import { Keypair, PublicKey, Connection, SystemProgram } from "@solana/web3.js";
import { DEX_ID } from "./ids";
import {
  cancelOrderInstruction,
  consumeEventInstruction,
  createMarketInstruction,
  initializeAccountInstruction,
  newOrderInstruction,
  settleInstruction,
} from "./instructions";
import { OrderType, PrimedTransaction, Side } from "./types";
import * as aaob from "@bonfida/aaob";
import BN from "bn.js";
import {
  createAssociatedTokenAccount,
  findAssociatedTokenAccount,
} from "./utils";
import { SelfTradeBehavior } from "./state";
import { Market } from "./market";

const MARKET_STATE_SPACE = 1 + 1 + 32 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8;
const NODE_CAPACITY = 100;
const EVENT_CAPACITY = 100;

export const createMarket = async (
  connection: Connection,
  baseMint: PublicKey,
  quoteMint: PublicKey,
  feePayer: PublicKey
): Promise<PrimedTransaction> => {
  // Market Account
  const marketAccount = new Keypair();
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
  }).getInstruction(
    DEX_ID,
    marketAccount.publicKey,
    aaobSigners[3].publicKey,
    await findAssociatedTokenAccount(marketSigner, baseMint),
    await findAssociatedTokenAccount(marketSigner, quoteMint),
    aaob.AAOB_ID
  );

  return [
    [marketAccount, ...aaobSigners],
    [
      createMarketAccount,
      ...aaobInstructions,
      createBaseVault,
      createQuoteVault,
      createMarket,
    ],
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
  owner: PublicKey
) => {
  const marketSigner = await PublicKey.createProgramAddress(
    [market.address.toBuffer()],
    DEX_ID
  );

  const userAccount = await PublicKey.createProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  const instruction = new newOrderInstruction({
    side: side as number,
    limitPrice: new BN(limitPrice),
    maxBaseQty: new BN(1),
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
    owner
  );

  return instruction;
};

export const cancelOrder = async (
  market: Market,
  orderIndex: BN,
  owner: PublicKey
) => {
  const marketSigner = await PublicKey.createProgramAddress(
    [market.address.toBuffer()],
    DEX_ID
  );

  const userAccount = await PublicKey.createProgramAddress(
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
  const userAccount = await PublicKey.createProgramAddress(
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
  const userAccount = await PublicKey.createProgramAddress(
    [market.address.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  const marketSigner = await PublicKey.createProgramAddress(
    [market.address.toBuffer()],
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
  const marketSigner = await PublicKey.createProgramAddress(
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
