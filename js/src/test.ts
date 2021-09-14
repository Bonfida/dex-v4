require("dotenv").config();
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import { createMarket } from "./bindings";
import { Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  createAssociatedTokenAccount,
  signAndSendTransactionInstructions,
} from "./utils";

const URL = "https://api.devnet.solana.com";

const connection = new Connection(URL);

const SECRET_KEY = process.env.SECRET_KEY;

if (!SECRET_KEY) {
  throw new Error("No secret key");
}

const wallet = Keypair.fromSecretKey(new Uint8Array(JSON.parse(SECRET_KEY)));

console.log(`Wallet ${wallet.publicKey.toBase58()}`);

const mint1 = new PublicKey("CZen4jVxdisrutQo2FeNY916uoeuEtLwfqSqJk9HHdEF");
const mint2 = new PublicKey("Cq47UeAkQcZmnaLPFpbHF8ZLjrPu4PhjshtsoKifMmMU");

const test = async () => {
  const primedTxs = await createMarket(
    connection,
    mint1,
    mint2,
    0.1,
    wallet.publicKey
  );

  for (let primedTx of primedTxs) {
    const tx = await signAndSendTransactionInstructions(
      connection,
      primedTx[0],
      wallet,
      primedTx[1]
    );
    console.log(`Confirmed signature ${tx}`);
  }
};
test();
