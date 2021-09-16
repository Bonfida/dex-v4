import { Slab, SlabHeader } from "@bonfida/aaob";
import { PublicKey, Connection } from "@solana/web3.js";
import { Market } from "./market";
import { throwIfNull } from "./utils";
import * as aaob from "@bonfida/aaob";
import { CALLBACK_INFO_LEN } from "./state";

export class Orderbook {
  market: Market;
  slabBids: Slab;
  slabAsks: Slab;

  constructor(market: Market, slabBids: Slab, slabAsks: Slab) {
    this.market = market;
    this.slabBids = slabBids;
    this.slabAsks = slabAsks;
  }

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

  static async load(connection: Connection, marketAddress: PublicKey) {
    const market = await Market.load(connection, marketAddress);
    const slabBids = await Orderbook.loadSlab(connection, market.bidsAddress);
    const slabAsks = await Orderbook.loadSlab(connection, market.asksAddress);
    return new Orderbook(market, slabBids, slabAsks);
  }
}
