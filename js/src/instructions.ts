import {
  PublicKey,
  TransactionInstruction,
  SYSVAR_CLOCK_PUBKEY,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import BN from "bn.js";
import { Schema, serialize } from "borsh";
import { SelfTradeBehavior } from "./state";
import { OrderType, Side } from "./types";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

export class createMarketInstruction {
  tag: number;
  signerNonce: number;
  minBaseOrderSize: BN;

  static schema: Schema = new Map([
    [
      createMarketInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u8"],
          ["signerNonce", "u8"],
          ["minBaseOrderSize", "u64"],
        ],
      },
    ],
  ]);

  constructor(obj: { signerNonce: number; minBaseOrderSize: BN }) {
    this.tag = 0;
    this.signerNonce = obj.signerNonce;
    this.minBaseOrderSize = obj.minBaseOrderSize;
  }

  serialize(): Uint8Array {
    return serialize(createMarketInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    market: PublicKey,
    orderbook: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    aaobId: PublicKey
  ): TransactionInstruction {
    const data = Buffer.from(this.serialize());
    // TODO check isSigner and isWritable
    const keys = [
      // Account 1
      {
        pubkey: SYSVAR_CLOCK_PUBKEY,
        isSigner: false,
        isWritable: false,
      },
      // Account 2
      {
        pubkey: market,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: orderbook,
        isSigner: false,
        isWritable: true,
      },
      // Account 4
      {
        pubkey: baseVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 5
      {
        pubkey: baseVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 6
      {
        pubkey: quoteVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 7
      {
        pubkey: aaobId,
        isSigner: false,
        isWritable: false,
      },
    ];
    return new TransactionInstruction({
      keys,
      programId: dexId,
      data,
    });
  }
}

export class newOrderInstruction {
  tag: number;
  side: Side;
  limitPrice: BN;
  maxBaseQty: BN;
  maxQuoteQty: BN;
  orderType: OrderType;
  selfTradeBehaviour: SelfTradeBehavior;
  matchLimit: BN;

  static schema: Schema = new Map([
    [
      newOrderInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u8"],
          ["side", "u8"],
          ["limitPrice", "u64"],
          ["maxBaseQty", "u64"],
          ["maxQuoteQty", "u64"],
          ["orderType", "u8"],
          ["selfTradeBehaviour", "u8"],
          ["matchLimit", "u64"],
        ],
      },
    ],
  ]);

  constructor(obj: {
    side: number;
    limitPrice: BN;
    maxBaseQty: BN;
    maxQuoteQty: BN;
    orderType: number;
    selfTradeBehaviour: number;
    matchLimit: BN;
  }) {
    this.tag = 1;
    this.side = obj.side as Side;
    this.limitPrice = obj.limitPrice;
    this.maxBaseQty = obj.maxBaseQty;
    this.maxQuoteQty = obj.maxQuoteQty;
    this.orderType = obj.orderType as OrderType;
    this.selfTradeBehaviour = obj.selfTradeBehaviour as SelfTradeBehavior;
    this.matchLimit = obj.matchLimit;
  }

  serialize(): Uint8Array {
    return serialize(newOrderInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    aaobId: PublicKey,
    market: PublicKey,
    marketSigner: PublicKey,
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
  ) {
    // TODO check isSigner and isWritable
    const data = Buffer.from(this.serialize());
    let keys = [
      // Account 1
      {
        pubkey: aaobId,
        isSigner: false,
        isWritable: false,
      },
      // Account 2
      {
        pubkey: TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
      // Account 4
      {
        pubkey: SYSVAR_RENT_PUBKEY,
        isSigner: false,
        isWritable: false,
      },
      // Account 5
      {
        pubkey: market,
        isSigner: false,
        isWritable: true,
      },
      // Account 6
      {
        pubkey: marketSigner,
        isSigner: false,
        isWritable: false,
      },
      // Account 7
      {
        pubkey: orderbook,
        isSigner: false,
        isWritable: true,
      },
      // Account 8
      {
        pubkey: eventQueue,
        isSigner: false,
        isWritable: true,
      },
      // Account 9
      {
        pubkey: bids,
        isSigner: false,
        isWritable: true,
      },
      // Account 10
      {
        pubkey: asks,
        isSigner: false,
        isWritable: true,
      },
      // Account 11
      {
        pubkey: baseVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 12
      {
        pubkey: quoteVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 13
      {
        pubkey: user,
        isSigner: false,
        isWritable: true,
      },
      // Account 14
      {
        pubkey: userTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      // Account 15
      {
        pubkey: userOwner,
        isSigner: true,
        isWritable: true,
      },
    ];
    if (discountTokenAccount) {
      keys.push(
        // Account 16
        {
          pubkey: discountTokenAccount,
          isSigner: false,
          isWritable: false,
        }
      );
    }
    return new TransactionInstruction({
      keys,
      programId: dexId,
      data,
    });
  }
}

export class cancelOrderInstruction {
  tag: number;
  orderIndex: BN;

  constructor(obj: { orderIndex: BN }) {
    this.tag = 2;
    this.orderIndex = obj.orderIndex;
  }

  static schema: Schema = new Map([
    [
      cancelOrderInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u8"],
          ["orderIndex", "u64"],
        ],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(cancelOrderInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    aaobId: PublicKey,
    market: PublicKey,
    marketSigner: PublicKey,
    orderbook: PublicKey,
    eventQueue: PublicKey,
    bids: PublicKey,
    asks: PublicKey,
    user: PublicKey,
    userOwner: PublicKey
  ) {
    const data = Buffer.from(this.serialize());
    const keys = [
      // Account 1
      {
        pubkey: aaobId,
        isSigner: false,
        isWritable: false,
      },
      // Account 2
      {
        pubkey: market,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: marketSigner,
        isSigner: false,
        isWritable: false,
      },
      // Account 4
      {
        pubkey: orderbook,
        isSigner: false,
        isWritable: true,
      },
      // Account 5
      {
        pubkey: eventQueue,
        isSigner: false,
        isWritable: true,
      },
      // Account 6
      {
        pubkey: bids,
        isSigner: false,
        isWritable: true,
      },
      // Account 7
      {
        pubkey: asks,
        isSigner: false,
        isWritable: true,
      },
      // Account 8
      {
        pubkey: user,
        isSigner: false,
        isWritable: true,
      },
      // Account 9
      {
        pubkey: userOwner,
        isSigner: true,
        isWritable: false,
      },
    ];

    return new TransactionInstruction({
      keys,
      programId: dexId,
      data,
    });
  }
}

export class consumeEventInstruction {
  tag: number;
  maxIteration: BN;

  constructor(obj: { maxIteration: BN }) {
    this.tag = 3;
    this.maxIteration = obj.maxIteration;
  }

  static schema: Schema = new Map([
    [
      consumeEventInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u8"],
          ["maxIteration", "u64"],
        ],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(consumeEventInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    aaobId: PublicKey,
    market: PublicKey,
    marketSigner: PublicKey,
    orderbook: PublicKey,
    eventQueue: PublicKey,
    rewardTarget: PublicKey,
    msrmTokenAccount: PublicKey,
    msrmTokenAccountOwner: PublicKey,
    userAccounts: PublicKey[]
  ) {
    const data = Buffer.from(this.serialize());
    let keys = [
      // Account 1
      {
        pubkey: aaobId,
        isSigner: false,
        isWritable: false,
      },
      // Account 2
      {
        pubkey: market,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: marketSigner,
        isSigner: false,
        isWritable: false,
      },
      // Account 4
      {
        pubkey: orderbook,
        isSigner: false,
        isWritable: true,
      },
      // Account 5
      {
        pubkey: eventQueue,
        isSigner: false,
        isWritable: true,
      },
      // Account 6
      {
        pubkey: rewardTarget,
        isSigner: false,
        isWritable: true,
      },
      // Account 7
      {
        pubkey: msrmTokenAccount,
        isSigner: false,
        isWritable: false,
      },
      // Account 8
      {
        pubkey: msrmTokenAccountOwner,
        isSigner: true,
        isWritable: false,
      },
    ];

    userAccounts.forEach((acc) =>
      keys.push({
        pubkey: acc,
        isSigner: false,
        isWritable: true,
      })
    );

    return new TransactionInstruction({
      keys,
      programId: dexId,
      data,
    });
  }
}

export class settleInstruction {
  tag: number;

  constructor() {
    this.tag = 4;
  }

  static schema: Schema = new Map([
    [
      settleInstruction,
      {
        kind: "struct",
        fields: [["tag", "u8"]],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(settleInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    aaobId: PublicKey,
    market: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    marketSigner: PublicKey,
    user: PublicKey,
    userOwner: PublicKey,
    destinationBaseAccount: PublicKey,
    destinationQuoteAccount: PublicKey
  ) {
    const data = Buffer.from(this.serialize());
    let keys = [
      // Account 1
      {
        pubkey: aaobId,
        isSigner: false,
        isWritable: false,
      },
      // Account 2
      {
        pubkey: TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: market,
        isSigner: false,
        isWritable: false,
      },
      // Account 4
      {
        pubkey: baseVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 5
      {
        pubkey: quoteVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 6
      {
        pubkey: marketSigner,
        isSigner: true,
        isWritable: false,
      },
      // Account 7
      {
        pubkey: user,
        isSigner: false,
        isWritable: true,
      },
      // Account 8
      {
        pubkey: userOwner,
        isSigner: true,
        isWritable: false,
      },
      // Account 9
      {
        pubkey: destinationBaseAccount,
        isSigner: false,
        isWritable: true,
      },
      // Account 10
      {
        pubkey: destinationQuoteAccount,
        isSigner: false,
        isWritable: true,
      },
    ];

    return new TransactionInstruction({
      keys,
      programId: dexId,
      data,
    });
  }
}

export class initializeAccountInstruction {
  tag: number;
  market: PublicKey;
  maxOrders: BN;

  constructor(obj: { market: Uint8Array; maxOrders: BN }) {
    this.tag = 5;
    this.market = new PublicKey(obj.market);
    this.maxOrders = obj.maxOrders;
  }

  static schema: Schema = new Map([
    [
      initializeAccountInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u8"],
          ["market", [32]],
          ["maxOrders", "u64"],
        ],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(initializeAccountInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    user: PublicKey,
    userOwner: PublicKey,
    feePayer: PublicKey
  ) {
    const data = Buffer.from(this.serialize());
    const keys = [
      // Account 1
      {
        pubkey: SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
      // Account 2
      {
        pubkey: SYSVAR_RENT_PUBKEY,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: user,
        isSigner: false,
        isWritable: true,
      },
      // Account 4
      {
        pubkey: userOwner,
        isSigner: true,
        isWritable: false,
      },
      // Account 5
      {
        pubkey: feePayer,
        isSigner: true,
        isWritable: true,
      },
    ];

    return new TransactionInstruction({
      keys,
      programId: dexId,
      data,
    });
  }
}

export class sweepFeesInstruction {
  tag: number;

  constructor() {
    this.tag = 6;
  }

  static schema: Schema = new Map([
    [
      sweepFeesInstruction,
      {
        kind: "struct",
        fields: [["tag", "u8"]],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(sweepFeesInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    market: PublicKey,
    marketSigner: PublicKey,
    marketAdmin: PublicKey,
    quoteVault: PublicKey,
    destinationTokenAccount: PublicKey
  ) {
    const data = Buffer.from(this.serialize());
    const keys = [
      // Account 1
      {
        pubkey: market,
        isSigner: false,
        isWritable: true,
      },
      // Account 2
      {
        pubkey: marketSigner,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: marketAdmin,
        isSigner: true,
        isWritable: false,
      },
      // Account 4
      {
        pubkey: quoteVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 5
      {
        pubkey: destinationTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      // Account 6
      {
        pubkey: TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
    ];

    return new TransactionInstruction({
      keys,
      programId: dexId,
      data,
    });
  }
}
