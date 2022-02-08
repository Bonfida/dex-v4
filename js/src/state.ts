import { Connection, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { deserialize, deserializeUnchecked, Schema } from "borsh";

export const CALLBACK_INFO_LEN = 33;

export enum AccountTag {
  Initialized = 0,
  MarketState = 1,
  UserAccount = 2,
}

export enum SelfTradeBehavior {
  DecrementTake = 0,
  CancelProvide = 1,
  AbortTransaction = 2,
}

export class MarketState {
  tag: AccountTag;
  baseMint: PublicKey;
  quoteMint: PublicKey;
  baseVault: PublicKey;
  quoteVault: PublicKey;
  orderbook: PublicKey;
  admin: PublicKey;
  creationTimestamp: BN;
  baseVolume: BN;
  quoteVolume: BN;
  accumulatedFees: BN;
  minBaseOrderSize: BN;
  signerNonce: number;
  feeTierThresholds: BN[];
  feeTierTakerBpsRates: BN[];
  feeTierMakerBpsRates: BN[];

  static schema: Schema = new Map([
    [
      MarketState,
      {
        kind: "struct",
        fields: [
          ["tag", "u64"],
          ["baseMint", [32]],
          ["quoteMint", [32]],
          ["baseVault", [32]],
          ["quoteVault", [32]],
          ["orderbook", [32]],
          ["admin", [32]],
          ["creationTimestamp", "u64"],
          ["baseVolume", "u64"],
          ["quoteVolume", "u64"],
          ["accumulatedFees", "u64"],
          ["minBaseOrderSize", "u64"],
          ["signerNonce", "u64"],
          ["feeTierThresholds", ["u64", 6]],
          ["feeTierTakerBpsRates", ["u64", 7]],
          ["feeTierMakerBpsRates", ["u64", 7]],
        ],
      },
    ],
  ]);

  constructor(obj: {
    tag: BN;
    signerNonce: BN;
    baseMint: Uint8Array;
    quoteMint: Uint8Array;
    baseVault: Uint8Array;
    quoteVault: Uint8Array;
    orderbook: Uint8Array;
    admin: Uint8Array;
    creationTimestamp: BN;
    baseVolume: BN;
    quoteVolume: BN;
    accumulatedFees: BN;
    minBaseOrderSize: BN;
    feeTierThresholds: BN[];
    feeTierTakerBpsRates: BN[];
    feeTierMakerBpsRates: BN[];
  }) {
    this.tag = obj.tag.toNumber() as AccountTag;
    this.signerNonce = obj.signerNonce.toNumber();
    this.baseMint = new PublicKey(obj.baseMint);
    this.quoteMint = new PublicKey(obj.quoteMint);
    this.baseVault = new PublicKey(obj.baseVault);
    this.quoteVault = new PublicKey(obj.quoteVault);
    this.orderbook = new PublicKey(obj.orderbook);
    this.admin = new PublicKey(obj.admin);
    this.creationTimestamp = obj.creationTimestamp;
    this.baseVolume = obj.baseVolume;
    this.quoteVolume = obj.quoteVolume;
    this.accumulatedFees = obj.accumulatedFees;
    this.minBaseOrderSize = obj.minBaseOrderSize;
    this.feeTierThresholds = obj.feeTierThresholds;
    this.feeTierTakerBpsRates = obj.feeTierTakerBpsRates;
    this.feeTierMakerBpsRates = obj.feeTierMakerBpsRates;
  }

  static async retrieve(connection: Connection, market: PublicKey) {
    const accountInfo = await connection.getAccountInfo(market);
    if (!accountInfo?.data) {
      throw new Error("Invalid account provided");
    }
    return deserialize(
      this.schema,
      MarketState,
      accountInfo.data
    ) as MarketState;
  }
}

export class UserAccount {
  tag: AccountTag;
  market: PublicKey;
  owner: PublicKey;
  baseTokenFree: BN;
  baseTokenLocked: BN;
  quoteTokenFree: BN;
  quoteTokenLocked: BN;
  accumulatedRebates: BN;
  accumulatedMakerQuoteVolume: BN;
  accumulatedMakerBaseVolume: BN;
  accumulatedTakerQuoteVolume: BN;
  accumulatedTakerBaseVolume: BN;
  orders: BN[];

  static schema: Schema = new Map([
    [
      UserAccount,
      {
        kind: "struct",
        fields: [
          ["tag", "u64"],
          ["market", [32]],
          ["owner", [32]],
          ["baseTokenFree", "u64"],
          ["baseTokenLocked", "u64"],
          ["quoteTokenFree", "u64"],
          ["quoteTokenLocked", "u64"],
          ["accumulatedRebates", "u64"],
          ["accumulatedMakerQuoteVolume", "u64"],
          ["accumulatedMakerBaseeVolume", "u64"],
          ["accumulatedTakerQuoteVolume", "u64"],
          ["accumulatedTakerBaseVolume", "u64"],
          ["_padding", "u32"],
          ["orders", ["u128"]],
        ],
      },
    ],
  ]);

  constructor(obj: {
    tag: BN;
    market: Uint8Array;
    owner: Uint8Array;
    baseTokenFree: BN;
    baseTokenLocked: BN;
    quoteTokenFree: BN;
    quoteTokenLocked: BN;
    orders: BN[];
    accumulatedRebates: BN;
    accumulatedMakerQuoteVolume: BN;
    accumulatedMakerBaseVolume: BN;
    accumulatedTakerQuoteVolume: BN;
    accumulatedTakerBaseVolume: BN;
  }) {
    this.tag = obj.tag.toNumber();
    this.market = new PublicKey(obj.market);
    this.owner = new PublicKey(obj.owner);
    this.baseTokenFree = obj.baseTokenFree;
    this.baseTokenLocked = obj.baseTokenLocked;
    this.quoteTokenFree = obj.quoteTokenFree;
    this.quoteTokenLocked = obj.quoteTokenLocked;
    this.orders = obj.orders;
    this.accumulatedRebates = obj.accumulatedRebates;
    this.accumulatedMakerQuoteVolume = obj.accumulatedMakerQuoteVolume;
    this.accumulatedMakerBaseVolume = obj.accumulatedMakerBaseVolume;
    this.accumulatedTakerQuoteVolume = obj.accumulatedTakerQuoteVolume;
    this.accumulatedTakerBaseVolume = obj.accumulatedTakerBaseVolume;
  }

  static async retrieve(connection: Connection, userAccount: PublicKey) {
    const accountInfo = await connection.getAccountInfo(userAccount);
    if (!accountInfo?.data) {
      throw new Error("Invalid account provided");
    }
    return deserializeUnchecked(
      this.schema,
      UserAccount,
      accountInfo.data
    ) as UserAccount;
  }
}
