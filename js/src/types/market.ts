import { Commitment } from "@solana/web3.js";

export interface MarketOptions {
  skipPreflight?: boolean;
  commitment?: Commitment;
}
