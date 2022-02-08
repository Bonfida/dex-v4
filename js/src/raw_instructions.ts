// This file is auto-generated. DO NOT EDIT
import BN from "bn.js";
import { Schema, serialize } from "borsh";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";

export interface AccountKey {
  pubkey: PublicKey;
  isSigner: boolean;
  isWritable: boolean;
}
export class closeAccountInstruction {
  tag: BN;
  static schema: Schema = new Map([
    [
      closeAccountInstruction,
      {
        kind: "struct",
        fields: [["tag", "u64"]],
      },
    ],
  ]);
  constructor() {
    this.tag = new BN(7);
  }
  serialize(): Uint8Array {
    return serialize(closeAccountInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    user: PublicKey,
    userOwner: PublicKey,
    targetLamportsAccount: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: user,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: userOwner,
      isSigner: true,
      isWritable: false,
    });
    keys.push({
      pubkey: targetLamportsAccount,
      isSigner: false,
      isWritable: true,
    });
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class newOrderInstruction {
  tag: BN;
  limitPrice: BN;
  maxBaseQty: BN;
  maxQuoteQty: BN;
  matchLimit: BN;
  side: number;
  orderType: number;
  selfTradeBehavior: number;
  padding: Uint8Array;
  static schema: Schema = new Map([
    [
      newOrderInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u64"],
          ["limitPrice", "u64"],
          ["maxBaseQty", "u64"],
          ["maxQuoteQty", "u64"],
          ["matchLimit", "u64"],
          ["side", "u8"],
          ["orderType", "u8"],
          ["selfTradeBehavior", "u8"],
          ["padding", [5]],
        ],
      },
    ],
  ]);
  constructor(obj: {
    limitPrice: BN;
    maxBaseQty: BN;
    maxQuoteQty: BN;
    matchLimit: BN;
    side: number;
    orderType: number;
    selfTradeBehavior: number;
  }) {
    this.tag = new BN(1);
    this.limitPrice = obj.limitPrice;
    this.maxBaseQty = obj.maxBaseQty;
    this.maxQuoteQty = obj.maxQuoteQty;
    this.matchLimit = obj.matchLimit;
    this.side = obj.side;
    this.orderType = obj.orderType;
    this.selfTradeBehavior = obj.selfTradeBehavior;
    this.padding = new Uint8Array(5).fill(0);
  }
  serialize(): Uint8Array {
    return serialize(newOrderInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    splTokenProgram: PublicKey,
    systemProgram: PublicKey,
    market: PublicKey,
    orderbook: PublicKey,
    eventQueue: PublicKey,
    bids: PublicKey,
    asks: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    user: PublicKey,
    userTokenAccount: PublicKey,
    userOwner: PublicKey,
    discountTokenAccount?: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: splTokenProgram,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: systemProgram,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: market,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: orderbook,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: eventQueue,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: bids,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: asks,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: baseVault,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: quoteVault,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: user,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: userTokenAccount,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: userOwner,
      isSigner: true,
      isWritable: true,
    });
    if (!!discountTokenAccount) {
      keys.push({
        pubkey: discountTokenAccount,
        isSigner: false,
        isWritable: false,
      });
    }
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class cancelOrderInstruction {
  tag: BN;
  orderIndex: BN;
  orderId: BN;
  static schema: Schema = new Map([
    [
      cancelOrderInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u64"],
          ["orderIndex", "u64"],
          ["orderId", "u128"],
        ],
      },
    ],
  ]);
  constructor(obj: { orderIndex: BN; orderId: BN }) {
    this.tag = new BN(2);
    this.orderIndex = obj.orderIndex;
    this.orderId = obj.orderId;
  }
  serialize(): Uint8Array {
    return serialize(cancelOrderInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    market: PublicKey,
    orderbook: PublicKey,
    eventQueue: PublicKey,
    bids: PublicKey,
    asks: PublicKey,
    user: PublicKey,
    userOwner: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: market,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: orderbook,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: eventQueue,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: bids,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: asks,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: user,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: userOwner,
      isSigner: true,
      isWritable: false,
    });
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class settleInstruction {
  tag: BN;
  static schema: Schema = new Map([
    [
      settleInstruction,
      {
        kind: "struct",
        fields: [["tag", "u64"]],
      },
    ],
  ]);
  constructor() {
    this.tag = new BN(4);
  }
  serialize(): Uint8Array {
    return serialize(settleInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    splTokenProgram: PublicKey,
    market: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    marketSigner: PublicKey,
    user: PublicKey,
    userOwner: PublicKey,
    destinationBaseAccount: PublicKey,
    destinationQuoteAccount: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: splTokenProgram,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: market,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: baseVault,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: quoteVault,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: marketSigner,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: user,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: userOwner,
      isSigner: true,
      isWritable: false,
    });
    keys.push({
      pubkey: destinationBaseAccount,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: destinationQuoteAccount,
      isSigner: false,
      isWritable: true,
    });
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class initializeAccountInstruction {
  tag: BN;
  market: Uint8Array;
  maxOrders: BN;
  static schema: Schema = new Map([
    [
      initializeAccountInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u64"],
          ["market", [32]],
          ["maxOrders", "u64"],
        ],
      },
    ],
  ]);
  constructor(obj: { market: Uint8Array; maxOrders: BN }) {
    this.tag = new BN(5);
    this.market = obj.market;
    this.maxOrders = obj.maxOrders;
  }
  serialize(): Uint8Array {
    return serialize(initializeAccountInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    systemProgram: PublicKey,
    user: PublicKey,
    userOwner: PublicKey,
    feePayer: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: systemProgram,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: user,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: userOwner,
      isSigner: true,
      isWritable: false,
    });
    keys.push({
      pubkey: feePayer,
      isSigner: true,
      isWritable: true,
    });
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class closeMarketInstruction {
  tag: BN;
  static schema: Schema = new Map([
    [
      closeMarketInstruction,
      {
        kind: "struct",
        fields: [["tag", "u64"]],
      },
    ],
  ]);
  constructor() {
    this.tag = new BN(8);
  }
  serialize(): Uint8Array {
    return serialize(closeMarketInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    market: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    orderbook: PublicKey,
    eventQueue: PublicKey,
    bids: PublicKey,
    asks: PublicKey,
    marketAdmin: PublicKey,
    targetLamportsAccount: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: market,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: baseVault,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: quoteVault,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: orderbook,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: eventQueue,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: bids,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: asks,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: marketAdmin,
      isSigner: true,
      isWritable: false,
    });
    keys.push({
      pubkey: targetLamportsAccount,
      isSigner: false,
      isWritable: true,
    });
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class consumeEventsInstruction {
  tag: BN;
  maxIterations: BN;
  static schema: Schema = new Map([
    [
      consumeEventsInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u64"],
          ["maxIterations", "u64"],
        ],
      },
    ],
  ]);
  constructor(obj: { maxIterations: BN }) {
    this.tag = new BN(3);
    this.maxIterations = obj.maxIterations;
  }
  serialize(): Uint8Array {
    return serialize(consumeEventsInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    market: PublicKey,
    orderbook: PublicKey,
    eventQueue: PublicKey,
    rewardTarget: PublicKey,
    userAccounts: PublicKey[]
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: market,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: orderbook,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: eventQueue,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: rewardTarget,
      isSigner: false,
      isWritable: true,
    });
    for (let k of userAccounts) {
      keys.push({
        pubkey: k,
        isSigner: false,
        isWritable: true,
      });
    }
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class sweepFeesInstruction {
  tag: BN;
  static schema: Schema = new Map([
    [
      sweepFeesInstruction,
      {
        kind: "struct",
        fields: [["tag", "u64"]],
      },
    ],
  ]);
  constructor() {
    this.tag = new BN(6);
  }
  serialize(): Uint8Array {
    return serialize(sweepFeesInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    market: PublicKey,
    marketSigner: PublicKey,
    marketAdmin: PublicKey,
    quoteVault: PublicKey,
    destinationTokenAccount: PublicKey,
    splTokenProgram: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: market,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: marketSigner,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: marketAdmin,
      isSigner: true,
      isWritable: false,
    });
    keys.push({
      pubkey: quoteVault,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: destinationTokenAccount,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: splTokenProgram,
      isSigner: false,
      isWritable: false,
    });
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
export class createMarketInstruction {
  tag: BN;
  signerNonce: BN;
  minBaseOrderSize: BN;
  tickSize: BN;
  crankerReward: BN;
  feeTierThresholds: BN[];
  feeTierTakerBpsRates: BN[];
  feeTierMakerBpsRebates: BN[];
  static schema: Schema = new Map([
    [
      createMarketInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u64"],
          ["signerNonce", "u64"],
          ["minBaseOrderSize", "u64"],
          ["tickSize", "u64"],
          ["crankerReward", "u64"],
          ["feeTierThresholds", ["u64", 6]],
          ["feeTierTakerBpsRates", ["u64", 7]],
          ["feeTierMakerBpsRebates", ["u64", 7]],
        ],
      },
    ],
  ]);
  constructor(obj: {
    signerNonce: BN;
    minBaseOrderSize: BN;
    tickSize: BN;
    crankerReward: BN;
    feeTierThresholds: BN[];
    feeTierTakerBpsRates: BN[];
    feeTierMakerBpsRebates: BN[];
  }) {
    this.tag = new BN(0);
    this.signerNonce = obj.signerNonce;
    this.minBaseOrderSize = obj.minBaseOrderSize;
    this.tickSize = obj.tickSize;
    this.crankerReward = obj.crankerReward;
    this.feeTierThresholds = obj.feeTierThresholds;
    this.feeTierTakerBpsRates = obj.feeTierTakerBpsRates;
    this.feeTierMakerBpsRebates = obj.feeTierMakerBpsRebates;
  }
  serialize(): Uint8Array {
    return serialize(createMarketInstruction.schema, this);
  }
  getInstruction(
    programId: PublicKey,
    market: PublicKey,
    orderbook: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    marketAdmin: PublicKey,
    eventQueue: PublicKey,
    asks: PublicKey,
    bids: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    let keys: AccountKey[] = [];
    keys.push({
      pubkey: market,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: orderbook,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: baseVault,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: quoteVault,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: marketAdmin,
      isSigner: false,
      isWritable: false,
    });
    keys.push({
      pubkey: eventQueue,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: asks,
      isSigner: false,
      isWritable: true,
    });
    keys.push({
      pubkey: bids,
      isSigner: false,
      isWritable: true,
    });
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
