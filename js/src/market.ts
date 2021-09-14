import {
  Commitment,
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  TransactionSignature,
} from "@solana/web3.js";
import {
  getMintDecimals,
  findAssociatedTokenAddress,
  getTokenBalance,
} from "./utils";
import { MarketOptions } from "./types/market";
import { CALLBACK_INFO_LEN, MarketState } from "./state";
import { DEX_ID, SRM_MINT, MSRM_MINT } from "./ids";
import { EventQueue, MarketState as AaobMarketState } from "@bonfida/aaob";
import { getFeeTier } from "./fees";

/**
 * A Serum DEX Market object
 */
export class Market {
  /** Market state
   * @private
   */
  private _marketState: MarketState;

  /** Asset agnostic orderbook state
   * @private
   */
  private _orderbookState: AaobMarketState;

  /** Address of the Serum DEX market
   * @private
   */
  private _address: PublicKey;

  /** Number of decimals of the base token
   * @private
   */
  private _baseDecimals: number;

  /** Number of decimals of the quote token
   * @private
   */
  private _quoteDecimals: number;

  /** Serum program ID of the market
   * @private
   */
  private _programId: PublicKey;

  /** Base vault address of the market
   * @private
   */
  private _baseVault: PublicKey;

  /** Quote vault address of the market
   * @private
   */
  private _quoteVault: PublicKey;

  /** Event queue address of the market
   * @private
   */
  private _eventQueueAddress: PublicKey;

  /** Address of the orderbook or AAOB market
   * @private
   */
  private _orderbookAddress: PublicKey;

  /** Preflight option (used in the connection object for sending tx)
   * @private
   */
  private _skipPreflight: boolean;
  /** Commitment option (used in the connection object)
   * @private
   */
  private _commitment: Commitment;

  constructor(
    marketState: MarketState,
    orderbookState: AaobMarketState,
    address: PublicKey,
    baseDecimals: number,
    quoteDecimals: number,
    options: MarketOptions,
    programdId: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    eventQueueAddress: PublicKey,
    orderbookAddress: PublicKey
  ) {
    this._marketState = marketState;
    this._orderbookState = orderbookState;
    this._address = address;
    this._baseDecimals = baseDecimals;
    this._quoteDecimals = quoteDecimals;
    this._skipPreflight = !!options.skipPreflight;
    this._commitment = options.commitment || "recent";
    this._programId = programdId;
    this._baseVault = baseVault;
    this._quoteVault = quoteVault;
    this._eventQueueAddress = eventQueueAddress;
    this._orderbookAddress = orderbookAddress;
  }

  /**
   *
   * @param connection The solana connection object to the RPC node
   * @param address Address of the Serum market to load
   * @param programId Program ID of Serum
   * @param options MarketOptions object (skipPreflight and Commitment)
   * @returns Returns a market object
   */
  static async load(
    connection: Connection,
    address: PublicKey,
    programId: PublicKey = DEX_ID,
    options: MarketOptions = {}
  ) {
    const marketState = await MarketState.retrieve(connection, address);

    const orderbookState = await AaobMarketState.retrieve(
      connection,
      marketState.orderbook
    );

    const [baseDecimals, quoteDecimals] = await Promise.all([
      getMintDecimals(connection, marketState.baseMint),
      getMintDecimals(connection, marketState.quoteMint),
    ]);

    return new Market(
      marketState,
      orderbookState,
      address,
      baseDecimals,
      quoteDecimals,
      options,
      programId,
      marketState.baseVault,
      marketState.quoteVault,
      orderbookState.eventQueue,
      marketState.orderbook
    );
  }

  /** Returns the Serum program ID of the market */
  get programId(): PublicKey {
    return this._programId;
  }

  /** Return the market address */
  get address(): PublicKey {
    return this._address;
  }

  /** Returns the mint address of the base token */
  get baseMintAddress(): PublicKey {
    return this._marketState.baseMint;
  }

  /** Returns the mint address of the quote token */
  get quoteMintAddress(): PublicKey {
    return this._marketState.quoteMint;
  }

  /** Returns the bids address (AOB program) of the market */
  get bidsAddress(): PublicKey {
    return this._orderbookState.bids;
  }

  /** Returns the asks address (AOB program) of the market */
  get asksAddress(): PublicKey {
    return this._orderbookState.asks;
  }

  /** Returns the market state */
  get marketState(): MarketState {
    return this._marketState;
  }

  /** Returns the orderbook state */
  get orderbookState(): AaobMarketState {
    return this._orderbookState;
  }

  /** Returns the number of decimals of the quote spl-token */
  get quoteDecimals(): number {
    return this._quoteDecimals;
  }

  /** Returns the number of decimals of the quote spl-token */
  get baseDecimals(): number {
    return this._baseDecimals;
  }

  /** Returns the base vault address of the market */
  get baseVault(): PublicKey {
    return this._baseVault;
  }

  /** Returns the quote vault address of the market */
  get quoteVault(): PublicKey {
    return this._quoteVault;
  }

  /** Returns the orderbook address of the market */
  get orderbookAddress(): PublicKey {
    return this._orderbookAddress;
  }

  /** Returns the event queue address of the market */
  get eventQueueAddress(): PublicKey {
    return this._eventQueueAddress;
  }

  /** Returns the inception base volume */
  baseVolume(): number {
    return this._marketState.baseVolume.toNumber();
  }

  /** Returns the inception quote volume */
  quoteVolume(): number {
    return this._marketState.quoteVolume.toNumber();
  }

  /** Returns the timestamp of the market creation */
  marketCreation(): number {
    return this._marketState.creationTimestamp.toNumber();
  }

  /**
   *
   * @param connection The solana connection object to the RPC node
   * @returns The decoded bids of the market
   */
  async loadBids(connection: Connection) {
    const bids = await this._orderbookState.loadBidsSlab(connection);
    return bids;
  }

  /**
   *
   * @param connection The solana connection object to the RPC node
   * @returns The decoded asks of the market
   */
  async loadAsks(connection: Connection) {
    const asks = await this._orderbookState.loadAsksSlab(connection);
    return asks;
  }

  async loadOrdersForOwner() {}

  filterForOpenOrders() {}

  /**
   * Fetch the associated token account of the owner for the base token of the market
   * @param owner The public key of the owner
   * @returns The public key of the associated token account of the owner
   */
  async findBaseTokenAccountsForOwner(owner: PublicKey) {
    const pubkey = await findAssociatedTokenAddress(
      owner,
      this._marketState.baseMint
    );
    return pubkey;
  }

  /**
   * Fetch the associated token account of the owner for the quote token of the market
   * @param owner The public key of the owner
   * @returns The public key of the associated token account of the owner
   */
  async findQuoteTokenAccountsForOwner(owner: PublicKey) {
    const pubkey = await findAssociatedTokenAddress(
      owner,
      this._marketState.quoteMint
    );
    return pubkey;
  }

  /**
   * Fetch the open order account of the owner
   * @param owner The public key of the owner
   * @returns The public key of the open order account
   */
  async findOpenOrdersAccountForOwner(owner: PublicKey) {
    const [address] = await PublicKey.findProgramAddress(
      [this.address.toBuffer(), owner.toBuffer()],
      this.programId
    );
    return address;
  }

  async placeOrder() {}

  /**
   * This method returns the fee discount keys assuming (M)SRM tokens are held in associated token account.
   * @param connection The solana connection object to the RPC node
   * @param owner The public key of the (M)SRM owner
   * @returns An array of `{ pubkey: PublicKey, mint: PublicKey, balance: number, feeTier: number }`
   */
  async findFeeDiscountKeys(connection: Connection, owner: PublicKey) {
    const [srmAddress, msrmAddress] = await Promise.all(
      [SRM_MINT, MSRM_MINT].map((e) => findAssociatedTokenAddress(owner, e))
    );
    const [srmBalance, msrmBalance] = await Promise.all(
      [srmAddress, msrmAddress].map((e) => getTokenBalance(connection, e))
    );
    return [
      {
        pubkey: srmAddress,
        mint: SRM_MINT,
        balance: srmBalance,
        feeTier: getFeeTier(0, srmBalance),
      },
      {
        pubkey: msrmAddress,
        mint: MSRM_MINT,
        balance: msrmBalance,
        feeTier: getFeeTier(msrmBalance, 0),
      },
    ];
  }

  async makePlaceOrderTransaction() {}

  makePlaceOrderInstruction() {}

  private async _sendTransaction(
    connection: Connection,
    transaction: Transaction,
    signers: Array<Keypair>
  ): Promise<TransactionSignature> {
    const signature = await connection.sendTransaction(transaction, signers, {
      skipPreflight: this._skipPreflight,
    });
    const { value } = await connection.confirmTransaction(
      signature,
      this._commitment
    );
    if (value?.err) {
      throw new Error(JSON.stringify(value.err));
    }
    return signature;
  }

  async cancelOrderByClientId() {}

  async settleFunds() {}

  async loadEventQueue(connection: Connection) {
    const eventQueue = await EventQueue.load(
      connection,
      this._orderbookState.eventQueue,
      CALLBACK_INFO_LEN
    );
    return eventQueue;
  }

  async loadFills() {}
}
