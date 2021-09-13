import { Connection, PublicKey } from "@solana/web3.js";
import {
  Token,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

export function throwIfNull<T>(
  value: T | null,
  message = "account not found"
): T {
  if (value === null) {
    throw new Error(message);
  }
  return value;
}

export const getMintDecimals = async (
  connection: Connection,
  mint: PublicKey
) => {
  const { value } = throwIfNull(
    await connection.getParsedAccountInfo(mint),
    "Mint not found"
  );
  // @ts-ignore
  return value?.data?.parsed.info.decimals;
};

export const findAssociatedTokenAccount = async (
  owner: PublicKey,
  mint: PublicKey
) => {
  const account = await Token.getAssociatedTokenAddress(
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    mint,
    owner
  );
  return account;
};

export const getTokenBalance = async (
  connection: Connection,
  address: PublicKey
) => {
  const { value } = throwIfNull(
    await connection.getParsedAccountInfo(address),
    "Token account does not exist"
  );
  // @ts-ignore
  return value?.data.parsed.uiAmount;
};
