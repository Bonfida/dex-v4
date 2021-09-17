import { Slab, SlabHeader } from "@bonfida/aaob";
import { PublicKey, Connection } from "@solana/web3.js";
import { Market } from "./market";
import { throwIfNull } from "./utils";
import * as aaob from "@bonfida/aaob";
import { CALLBACK_INFO_LEN } from "./state";

/**
 * Orderbook class
 */
export class Orderbook {
  /** Market of the orderbook
   * @private
   */
  private _market: Market;

  /** Slab that contains asks
   * @private
   */
  private _slabAsks: Slab;

  /** Slab that contains bids
   * @private
   */
  private _slabBids: Slab;

  constructor(market: Market, slabBids: Slab, slabAsks: Slab) {
    this._market = market;
    this._slabBids = slabBids;
    this._slabAsks = slabAsks;
  }

  /**
   *
   * @param connection The solana connection object to the RPC node
   * @param slabAddress The address of the Slab
   * @returns A deserialized Slab object
   */
  static async loadSlab(connection, slabAddress: PublicKey) {
    const { data } = throwIfNull(await connection.getAccountInfo(slabAddress));
    const slabHeader = aaob.SlabHeader.deserialize(
      data.slice(0, SlabHeader.LEN)
    );
    return new Slab({
      header: slabHeader,
      callBackInfoLen: CALLBACK_INFO_LEN,
      data,
    });
  }

  /**
   *
   * @param connection The solana connection object to the RPC node
   * @param marketAddress The address of the market
   * @returns Returns an orderbook object
   */
  static async load(connection: Connection, marketAddress: PublicKey) {
    const market = await Market.load(connection, marketAddress);
    const slabBids = await Orderbook.loadSlab(connection, market.bidsAddress);
    const slabAsks = await Orderbook.loadSlab(connection, market.asksAddress);
    return new Orderbook(market, slabBids, slabAsks);
  }
}
