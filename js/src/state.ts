import { Connection, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { deserialize, Schema } from "borsh";

export enum AccountTag {
  Initialized = 0,
  DexState = 1,
  UserAccount = 2,
}

export enum SelfTradeBehavior {
  DecrementTake = 0,
  CancelProvide = 1,
  AbortTransaction = 2,
}

export class DexState {
  tag: AccountTag;
  signerNonce: number;
  baseMint: PublicKey;
  quoteMint: PublicKey;
  baseVault: PublicKey;
  quoteVault: PublicKey;
  orderbook: PublicKey;
  aaobProgram: PublicKey;
  creationTimestamp: BN;
  baseVolume: BN;
  quoteVolume: BN;
  accumulatedFees: BN;

  static schema: Schema = new Map([
    [
      DexState,
      {
        kind: "struct",
        fields: [
          ["tag", "u8"],
          ["signerNonce", "u8"],
          ["baseMint", [32]],
          ["quoteMint", [32]],
          ["baseVault", [32]],
          ["quoteVault", [32]],
          ["orderbook", [32]],
          ["aaobProgram", [32]],
          ["creationTimestamp", "u64"],
          ["baseVolume", "u64"],
          ["quoteVolume", "u64"],
          ["accumulatedFees", "u64"],
        ],
      },
    ],
  ]);

  constructor(obj: {
    tag: number;
    signerNonce: number;
    baseMint: Uint8Array;
    quoteMint: Uint8Array;
    baseVault: Uint8Array;
    quoteVault: Uint8Array;
    orderbook: Uint8Array;
    aaobProgram: Uint8Array;
    creationTimestamp: BN;
    baseVolume: BN;
    quoteVolume: BN;
    accumulatedFees: BN;
  }) {
    this.tag = obj.tag as AccountTag;
    this.signerNonce = obj.signerNonce;
    this.baseMint = new PublicKey(obj.baseMint);
    this.quoteMint = new PublicKey(obj.quoteMint);
    this.baseVault = new PublicKey(obj.baseVault);
    this.quoteVault = new PublicKey(obj.quoteVault);
    this.orderbook = new PublicKey(obj.orderbook);
    this.aaobProgram = new PublicKey(Uint8Array);
    this.creationTimestamp = obj.creationTimestamp;
    this.baseVolume = obj.baseVolume;
    this.quoteVolume = obj.quoteVolume;
    this.accumulatedFees = obj.accumulatedFees;
  }

  static async retrieve(connection: Connection, market: PublicKey) {
    const accountInfo = await connection.getAccountInfo(market);
    if (!accountInfo?.data) {
      throw new Error("Invalid account provided");
    }
    return deserialize(this.schema, DexState, accountInfo.data) as DexState;
  }
}

export class UserAccountHeader {
  tag: AccountTag;
  market: PublicKey;
  owner: PublicKey;
  baseTokenFree: BN;
  baseTokenLocked: BN;
  quoteTokenFree: BN;
  quoteTokenLocked: BN;
  numberOfOrders: BN;
  accumulatedRebates: BN;

  constructor(obj: {
    tag: number;
    market: Uint8Array;
    owner: Uint8Array;
    baseTokenFree: BN;
    baseTokenLocked: BN;
    quoteTokenFree: BN;
    quoteTokenLocked: BN;
    numberOfOrder: BN;
    accumulatedRebates: BN;
  }) {
    this.tag = obj.tag;
    this.market = new PublicKey(obj.market);
    this.owner = new PublicKey(obj.owner);
    this.baseTokenFree = obj.baseTokenFree;
    this.baseTokenLocked = obj.baseTokenLocked;
    this.quoteTokenFree = obj.quoteTokenFree;
    this.quoteTokenLocked = obj.quoteTokenLocked;
    this.numberOfOrders = obj.numberOfOrder;
    this.accumulatedRebates = obj.accumulatedRebates;
  }
}
