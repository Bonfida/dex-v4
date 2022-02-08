require("dotenv").config();
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import {
  cancelOrder,
  createMarket,
  initializeAccount,
  placeOrder,
} from "./bindings";
import { Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  createAssociatedTokenAccount,
  findAssociatedTokenAddress,
  signAndSendTransactionInstructions,
  sleep,
} from "./utils";
import { Market } from "./market";
import { OrderType, Side } from "./types";
import { SelfTradeBehavior } from "./state";
import * as aaob from "@bonfida/aaob";
import { deserialize } from "borsh";
import { deserializeUnchecked } from "borsh";
import { parseNode, SlabHeader } from "@bonfida/aaob";
import { CALLBACK_INFO_LEN } from "./state";
import { OpenOrders } from "./openOrders";
import BN from "bn.js";

require("source-map-support").install();

const URL = "https://api.devnet.solana.com";

const connection = new Connection(URL);

const SECRET_KEY = process.env.SECRET_KEY;

if (!SECRET_KEY) {
  throw new Error("No secret key");
}

const wallet = Keypair.fromSecretKey(new Uint8Array(JSON.parse(SECRET_KEY)));

console.log(`Wallet ${wallet.publicKey.toBase58()}`);

const mint1 = new PublicKey("72m4rktxyKqWQxTnXz1rpjJ6v9RPaa6mW5Qb2aizQ8Zq");
const mint2 = new PublicKey("Cetq9LiKkhvQuyHRjbk1FSbbsWSCCEVvPVQ4BHCHDF3t");

const marketAddress = new PublicKey(
  "Gdaxn4WkV2ZyNcMYsUWiAnmjy4YqSka4woy8ggazh4ba"
);

const test = async () => {
  // Load market
  const market = await Market.load(connection, marketAddress);
  // Create market
  //
  // const instructions = await createMarket(
  //   connection,
  //   mint1,
  //   mint2,
  //   1,
  //   wallet.publicKey,
  //   wallet.publicKey
  // );
  // for (let primedTx of instructions) {
  //   const tx = await signAndSendTransactionInstructions(
  //     connection,
  //     primedTx[0],
  //     wallet,
  //     primedTx[1]
  //   );
  //   await sleep(1_000);
  //   console.log(`Tx ${tx}`);
  // }
  // Create user account
  // const instUA = await initializeAccount(market.address, wallet.publicKey, 10);
  // await signAndSendTransactionInstructions(connection, [wallet], wallet, [
  //   instUA,
  // ]);
  // return;
  // Place order

  // const init_inst = await initializeAccount(marketAddress, wallet.publicKey);
  // const init_tx = await signAndSendTransactionInstructions(
  //   connection,
  //   [wallet],
  //   wallet,
  //   [init_inst]
  // );
  // console.log(`Init acc ${init_tx}`);

  // await sleep(30_000);

  console.log(
    (
      await findAssociatedTokenAddress(wallet.publicKey, market.baseMintAddress)
    ).toBase58()
  );

  console.log(market.baseMintAddress.toBase58());

  const inst = await placeOrder(
    market,
    Side.Ask,
    1_500,
    5 * Math.pow(10, market.baseDecimals),
    OrderType.Limit,
    SelfTradeBehavior.CancelProvide,
    await findAssociatedTokenAddress(wallet.publicKey, market.baseMintAddress),
    wallet.publicKey
  );
  const tx = await signAndSendTransactionInstructions(
    connection,
    [wallet],
    wallet,
    [inst]
  );
  console.log(`Tx place order ${tx}`);

  // const slotSize = Math.max(CALLBACK_INFO_LEN + 8 + 16 + 1, 32);
  // const info = await connection.getAccountInfo(market.orderbookState.asks);
  // if (!info?.data) {
  //   throw new Error("Invalid data");
  // }

  // const { data } = info;
  // const slabHeader = aaob.SlabHeader.deserialize(
  //   data.slice(0, aaob.SlabHeader.LEN)
  // ) as aaob.SlabHeader;
  // const slab = new aaob.Slab({
  //   header: slabHeader,
  //   callBackInfoLen: 33,
  //   data: data,
  // });
  // console.log(slab.getMinMax(false));

  // User account

  // let user_wallet = new PublicKey(
  //   "3uf6wzMet5ZvzcfSeodTSdnNDD6xSauP3GpiBTvkLbvz"
  // );

  // const ua = await OpenOrders.load(connection, market.address, user_wallet);

  // console.log(ua.quoteTokenFree.toNumber());
  // let marketVaultBalance = await connection.getTokenAccountBalance(
  //   market.quoteVault
  // );
  // console.log(marketVaultBalance);

  // const o = ua.orders[0];
  // const inst = await cancelOrder(market, new BN(0), wallet.publicKey);

  // const tx = await signAndSendTransactionInstructions(
  //   connection,
  //   [wallet],
  //   wallet,
  //   [inst]
  // );

  // console.log(`Cancel ${tx}`);
};
test();
