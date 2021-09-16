import { Connection, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { DEX_ID } from "./ids";
import { UserAccount } from "./state";
import { initializeAccount } from "./bindings";

export class OpenOrders {
  address: PublicKey;
  market: PublicKey;
  owner: PublicKey;

  baseTokenFree: BN;
  baseTokenTotal: BN;

  quoteTokenFree: BN;
  quoteTokenTotal: BN;

  orders: BN[];

  accumulatedRebates: BN;

  constructor(
    address: PublicKey,
    market: PublicKey,
    owner: PublicKey,
    baseTokenFree: BN,
    baseTokenTotal: BN,
    quoteTokenFree: BN,
    quoteTokenTotal: BN,
    orders: BN[],
    accumulatedRebates: BN
  ) {
    this.address = address;
    this.market = market;
    this.owner = owner;
    this.baseTokenFree = baseTokenFree;
    this.baseTokenTotal = baseTokenTotal;
    this.quoteTokenFree = quoteTokenFree;
    this.quoteTokenTotal = quoteTokenTotal;
    this.orders = orders;
    this.accumulatedRebates = accumulatedRebates;
  }

  static async load(
    connection: Connection,
    market: PublicKey,
    owner: PublicKey
  ) {
    const [address] = await PublicKey.findProgramAddress(
      [market.toBuffer(), owner.toBuffer()],
      DEX_ID
    );

    const userAccount = await UserAccount.retrieve(connection, address);

    return new OpenOrders(
      address,
      market,
      owner,
      userAccount.baseTokenFree,
      userAccount.baseTokenLocked.add(userAccount.baseTokenFree),
      userAccount.quoteTokenFree,
      userAccount.quoteTokenFree.add(userAccount.quoteTokenLocked),
      userAccount.orders,
      userAccount.accumulatedRebates
    );
  }

  static async makeCreateAccountTransaction(
    market: PublicKey,
    owner: PublicKey,
    maxOrders = 20
  ) {
    return await initializeAccount(market, owner, maxOrders);
  }
}
