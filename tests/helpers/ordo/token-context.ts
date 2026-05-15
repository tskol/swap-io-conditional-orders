import * as spl from "@solana/spl-token";
import { web3 } from "@coral-xyz/anchor";

export async function getMintTokenProgram(
  connection: web3.Connection,
  mint: web3.PublicKey,
): Promise<web3.PublicKey | undefined> {
  return (await connection.getAccountInfo(mint))?.owner;
}

export async function getTokenPairPrograms(args: {
  connection: web3.Connection;
  inputMint: web3.PublicKey;
  outputMint: web3.PublicKey;
}): Promise<{
  inputTokenProgram: web3.PublicKey | undefined;
  outputTokenProgram: web3.PublicKey | undefined;
}> {
  const [inputTokenProgram, outputTokenProgram] = await Promise.all([
    getMintTokenProgram(args.connection, args.inputMint),
    getMintTokenProgram(args.connection, args.outputMint),
  ]);

  return { inputTokenProgram, outputTokenProgram };
}

export function getOwnerAta(args: {
  mint: web3.PublicKey;
  owner: web3.PublicKey;
  tokenProgram: web3.PublicKey | undefined;
}): web3.PublicKey {
  return spl.getAssociatedTokenAddressSync(
    args.mint,
    args.owner,
    false,
    args.tokenProgram,
  );
}
