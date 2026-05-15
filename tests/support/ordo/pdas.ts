import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

export function findPdaAuthority(args: {
  programId: web3.PublicKey;
  seed: string;
  globalConfig: web3.PublicKey;
}): [web3.PublicKey, number] {
  return web3.PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode(args.seed),
      args.globalConfig.toBuffer(),
    ],
    args.programId,
  );
}

export function findMintScopedPda(args: {
  programId: web3.PublicKey;
  seed: string;
  globalConfig: web3.PublicKey;
  mint: web3.PublicKey;
}): [web3.PublicKey, number] {
  return web3.PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode(args.seed),
      args.globalConfig.toBuffer(),
      args.mint.toBuffer(),
    ],
    args.programId,
  );
}
