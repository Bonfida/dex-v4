import { beforeAll, jest, test } from "@jest/globals";
import { Connection, Keypair, PublicKey, clusterApiUrl } from "@solana/web3.js";
import { initializePayer } from "./utils/validator";
import { createMarketTest } from "./create-market";

// Global state initialized once in test startup and cleaned up at test
// teardown.
let connection: Connection;
let feePayer: Keypair;
let payerKeyFile: string;
let programId: PublicKey;

beforeAll(async () => {
  connection = new Connection(clusterApiUrl("devnet"), "confirmed");
  [feePayer, payerKeyFile] = await initializePayer(connection);
  console.log("Fee payer", feePayer.publicKey.toBase58());
  programId = new PublicKey("GGKAzVAfJqtNPHhA8tGzz6RFnbinejaTdJZkVimSenM1");
});

jest.setTimeout(50_000_000);

test("Create market", async () => {
  await createMarketTest(connection, feePayer);
});
