import { Keypair, PublicKey, Connection, SystemProgram } from "@solana/web3.js";
import { DEX_ID, SRM_MINT } from "./ids";
import {
  cancelOrderInstruction,
  consumeEventsInstruction,
  createMarketInstruction,
  initializeAccountInstruction,
  newOrderInstruction,
  settleInstruction,
  closeAccountInstruction,
} from "./raw_instructions";
import { OrderType, PrimedTransaction, Side } from "./types";
import * as aaob from "@bonfida/aaob";
import BN from "bn.js";
import {
  createAssociatedTokenAccount,
  findAssociatedTokenAddress,
} from "./utils";
import { SelfTradeBehavior } from "./state";
import { Market } from "./market";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

/**
 * Constants
 */
const MARKET_STATE_SPACE = 408;
const NODE_CAPACITY = 100;
const EVENT_CAPACITY = 100;

export const createMarket = async (
  connection: Connection,
  baseMint: PublicKey,
  quoteMint: PublicKey,
  minBaseOrderSize: number,
  feePayer: PublicKey,
  marketAdmin: PublicKey,
  tickSize: BN,
  crankerReward: BN,
  feeTierThresholds: BN[],
  feeTierTakerBpsRates: BN[],
  feeTierMakerBpsRebates: BN[]
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
    new BN(minBaseOrderSize),
    feePayer,
    tickSize,
    crankerReward,
    DEX_ID
  );
  // Remove the AOB create_market instruction as it is not needed with lib usage
  aaobInstructions.pop();

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
    signerNonce: new BN(marketSignerNonce),
    minBaseOrderSize: new BN(minBaseOrderSize),
    tickSize: tickSize,
    crankerReward: new BN(crankerReward),
    feeTierThresholds,
    feeTierTakerBpsRates,
    feeTierMakerBpsRebates,
  }).getInstruction(
    DEX_ID,
    marketAccount.publicKey,
    aaobSigners[3].publicKey,
    await findAssociatedTokenAddress(marketSigner, baseMint),
    await findAssociatedTokenAddress(marketSigner, quoteMint),
    marketAdmin,
    aaobSigners[0].publicKey,
    aaobSigners[1].publicKey,
    aaobSigners[2].publicKey
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
    limitPrice: new BN(limitPrice * 2 ** 32),
    maxBaseQty: new BN(size),
    maxQuoteQty: new BN(Math.ceil(size * limitPrice)),
    orderType: type,
    selfTradeBehavior: selfTradeBehaviour,
    matchLimit: new BN(Number.MAX_SAFE_INTEGER), // TODO Change
  }).getInstruction(
    DEX_ID,
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

export const cancelOrder = async (
  market: Market,
  orderIndex: BN,
  orderId: BN,
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

  const instruction = new cancelOrderInstruction({
    orderIndex,
    orderId,
  }).getInstruction(
    DEX_ID,
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
  }).getInstruction(DEX_ID, SystemProgram.programId, userAccount, owner, owner);

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

export const comsumEvents = async (
  market: Market,
  rewardTarget: PublicKey,
  userAccounts: PublicKey[],
  maxIterations: BN
) => {
  const [marketSigner] = await PublicKey.findProgramAddress(
    [market.address.toBuffer()],
    DEX_ID
  );

  const instruction = new consumeEventsInstruction({
    maxIterations,
  }).getInstruction(
    DEX_ID,
    market.address,
    market.orderbookAddress,
    market.eventQueueAddress,
    rewardTarget,
    userAccounts
  );

  return instruction;
};

export const closeAccount = async (market: PublicKey, owner: PublicKey) => {
  const [userAccount] = await PublicKey.findProgramAddress(
    [market.toBuffer(), owner.toBuffer()],
    DEX_ID
  );

  const instruction = new closeAccountInstruction().getInstruction(
    DEX_ID,
    userAccount,
    owner,
    owner
  );

  return instruction;
};
