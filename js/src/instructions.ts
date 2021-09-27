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

  /**
   * Returns a TransactionInstruction to create a market
   * @param dexId Serum DEX program ID
   * @param market Address of the market
   * @param orderbook Address of the AAOB
   * @param baseVault Address of the market base vault
   * @param quoteVault Address of the market quote vault
   * @param aaobId AAOB program ID
   * @param marketAdmin Address of the market admin
   * @returns Returns a TransactionInstruction object
   */
  getInstruction(
    dexId: PublicKey,
    market: PublicKey,
    orderbook: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    aaobId: PublicKey,
    marketAdmin: PublicKey
  ): TransactionInstruction {
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
        pubkey: orderbook,
        isSigner: false,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: baseVault,
        isSigner: false,
        isWritable: false,
      },
      // Account 4
      {
        pubkey: quoteVault,
        isSigner: false,
        isWritable: false,
      },
      // Account 5
      {
        pubkey: aaobId,
        isSigner: false,
        isWritable: false,
      },
      // Account 6
      {
        pubkey: marketAdmin,
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

  /**
   * Returns a TransactionInstruction to place a new order
   * @param dexId Serum DEX program ID
   * @param aaobId AAOB program ID
   * @param market Address of the market
   * @param marketSigner Address of the market signer
   * @param orderbook Address of the AAOB
   * @param eventQueue Address of the event queue
   * @param bids Address of the bids slab
   * @param asks Address of the asks slab
   * @param baseVault Address of the market base vault
   * @param quoteVault Address of the market quote vault
   * @param user Address of the open order account
   * @param userTokenAccount Address of the token account
   * @param userOwner Address of the owner of the order
   * @param discountTokenAccount Address of the (M)SRM discount token account
   * @returns Returns a TransactionInstruction object
   */
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
        pubkey: market,
        isSigner: false,
        isWritable: true,
      },
      // Account 5
      {
        pubkey: marketSigner,
        isSigner: false,
        isWritable: false,
      },
      // Account 6
      {
        pubkey: orderbook,
        isSigner: false,
        isWritable: true,
      },
      // Account 7
      {
        pubkey: eventQueue,
        isSigner: false,
        isWritable: true,
      },
      // Account 8
      {
        pubkey: bids,
        isSigner: false,
        isWritable: true,
      },
      // Account 9
      {
        pubkey: asks,
        isSigner: false,
        isWritable: true,
      },
      // Account 10
      {
        pubkey: baseVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 11
      {
        pubkey: quoteVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 12
      {
        pubkey: user,
        isSigner: false,
        isWritable: true,
      },
      // Account 13
      {
        pubkey: userTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      // Account 14
      {
        pubkey: userOwner,
        isSigner: true,
        isWritable: true,
      },
    ];
    if (discountTokenAccount) {
      keys.push(
        // Account 15
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
  orderId: BN;

  constructor(obj: { orderIndex: BN; orderId: BN }) {
    this.tag = 2;
    this.orderIndex = obj.orderIndex;
    this.orderId = obj.orderId;
  }

  static schema: Schema = new Map([
    [
      cancelOrderInstruction,
      {
        kind: "struct",
        fields: [
          ["tag", "u8"],
          ["orderIndex", "u64"],
          ["orderId", "u128"],
        ],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(cancelOrderInstruction.schema, this);
  }

  /**
   * Returns a TransactionInstruction to cancel an order
   * @param dexId Serum DEX program ID
   * @param aaobId AAOB program ID
   * @param market Address of the market
   * @param marketSigner Address of the market signer
   * @param orderbook Address of the AAOB
   * @param eventQueue Address of the event queue
   * @param bids Address of the bids slab
   * @param asks Address of the asks slab
   * @param user Address of the open order account
   * @param userOwner Address of the owner of the order
   * @returns Returns a TransactionInstruction object
   */
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

  /**
   * Returns a TransactionInstruction to consume an event
   * @param dexId Serum DEX program ID
   * @param aaobId AAOB program ID
   * @param market  Address of the market
   * @param marketSigner Address of the market signer
   * @param orderbook Address of the AAOB
   * @param eventQueue Address of the event queue
   * @param rewardTarget Address to send the cranking reward
   * @param msrmTokenAccount Address of the MSRM token account
   * @param msrmTokenAccountOwner Address of the MSRM token account owner
   * @param userAccounts Address of the user accounts to crank
   * @returns Returns a TransactionInstruction object
   */
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

  /**
   * Returns a TransactionInstruction to settle funds
   * @param dexId Serum DEX program ID
   * @param aaobId AAOB program ID
   * @param market Address of the market
   * @param baseVault Address of the market base vault
   * @param quoteVault Address of the market quote vault
   * @param marketSigner Address of the market signer
   * @param user Address of the open order account
   * @param userOwner Address of the owner of the open order account
   * @param destinationBaseAccount Address of the destination for the base tokens settled
   * @param destinationQuoteAccount Address of the destination for the quote tokens settled
   * @returns Returns a TransactionInstruction object
   */
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
        isSigner: false,
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
  market: Uint8Array;
  maxOrders: BN;

  constructor(obj: { market: Uint8Array; maxOrders: BN }) {
    this.tag = 5;
    this.market = obj.market;
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

  /**
   * Returns a TransactionInstruction to initialize an open order account
   * @param dexId Serum DEX program ID
   * @param user Address of the open order account
   * @param userOwner Address of the open order account owner
   * @param feePayer Address of the fee payer
   * @returns Returns a TransactionInstruction object
   */
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
        pubkey: user,
        isSigner: false,
        isWritable: true,
      },
      // Account 3
      {
        pubkey: userOwner,
        isSigner: true,
        isWritable: false,
      },
      // Account 4
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
  /**
   * Returns a TransactionInstruction to sweep fees
   * @param dexId Serum DEX program ID
   * @param market Address of the market
   * @param marketSigner Address of the market signer
   * @param marketAdmin Address of the market admin
   * @param quoteVault Address of the market quote vault
   * @param destinationTokenAccount Address of the destination for the quote tokens settled
   * @returns Returns a TransactionInstruction object
   */
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

export class closeAccountIntruction {
  tag: number;
  constructor() {
    this.tag = 7;
  }

  static schema: Schema = new Map([
    [
      closeAccountIntruction,
      {
        kind: "struct",
        fields: [["tag", "u8"]],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(closeAccountIntruction.schema, this);
  }

  /**
   * Returns a TransactionInstruction to close an open order account
   * @param dexId Serum DEX program ID
   * @param user Address of the open order account
   * @param userOwner Address of the open order account owner
   * @param targetLamportAccount Address of the lamport receiver
   * @returns Returns a TransactionInstruction object
   */
  getInstruction(
    dexId: PublicKey,
    user: PublicKey,
    userOwner: PublicKey,
    targetLamportAccount: PublicKey
  ) {
    const data = Buffer.from(this.serialize());
    const keys = [
      // Account 1
      {
        pubkey: user,
        isSigner: false,
        isWritable: true,
      },
      // Account 2
      {
        pubkey: userOwner,
        isSigner: true,
        isWritable: false,
      },
      // Account 3
      {
        pubkey: targetLamportAccount,
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

export class closeMarketInstruction {
  tag: number;
  constructor() {
    this.tag = 8;
  }

  static schema: Schema = new Map([
    [
      closeMarketInstruction,
      {
        kind: "struct",
        fields: [["tag", "u8"]],
      },
    ],
  ]);

  serialize(): Uint8Array {
    return serialize(closeMarketInstruction.schema, this);
  }

  getInstruction(
    dexId: PublicKey,
    market: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    marketSigner: PublicKey,
    orderbook: PublicKey,
    eventQueue: PublicKey,
    bids: PublicKey,
    asks: PublicKey,
    aaobId: PublicKey,
    marketAdmin: PublicKey,
    targetLamportAccount: PublicKey
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
        pubkey: baseVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 3
      {
        pubkey: quoteVault,
        isSigner: false,
        isWritable: true,
      },
      // Account 4
      {
        pubkey: marketSigner,
        isSigner: false,
        isWritable: true,
      },
      // Account 5
      {
        pubkey: orderbook,
        isSigner: false,
        isWritable: true,
      },
      // Account 6
      {
        pubkey: eventQueue,
        isSigner: false,
        isWritable: true,
      },
      // Account 7
      {
        pubkey: bids,
        isSigner: false,
        isWritable: true,
      },
      // Account 8
      {
        pubkey: asks,
        isSigner: false,
        isWritable: true,
      },
      // Account 9
      {
        pubkey: aaobId,
        isSigner: false,
        isWritable: true,
      },
      // Account 10
      {
        pubkey: marketAdmin,
        isSigner: false,
        isWritable: true,
      },
      // Account 11
      {
        pubkey: targetLamportAccount,
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
