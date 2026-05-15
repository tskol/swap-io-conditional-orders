import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../tests/support/ordo-client";
import {
  requirePublicKey,
  SCRIPT_COMMITMENT,
  SCRIPT_GLOBAL_CONFIG,
} from "./config";

export function createScriptContext(): {
  connection: web3.Connection;
  provider: anchor.AnchorProvider;
  wallet: anchor.Wallet;
  ordoHelper: OrdoHelper;
} {
  const envProvider = anchor.AnchorProvider.env();
  const connection = new web3.Connection(envProvider.connection.rpcEndpoint, {
    commitment: SCRIPT_COMMITMENT,
  });
  const provider = new anchor.AnchorProvider(connection, envProvider.wallet, {
    commitment: SCRIPT_COMMITMENT,
    preflightCommitment: SCRIPT_COMMITMENT,
  });

  anchor.setProvider(provider);

  return {
    connection,
    provider,
    wallet: provider.wallet as anchor.Wallet,
    ordoHelper: new OrdoHelper(provider),
  };
}

export function applyScriptGlobalConfig(ordoHelper: OrdoHelper) {
  ordoHelper.setGlobalConfig(
    SCRIPT_GLOBAL_CONFIG ?? requirePublicKey("ORDO_GLOBAL_CONFIG"),
  );
}

export async function confirmSignature(
  connection: web3.Connection,
  signature: string,
) {
  const blockhash = await connection.getLatestBlockhash(SCRIPT_COMMITMENT);
  await connection.confirmTransaction({ signature, ...blockhash }, SCRIPT_COMMITMENT);
}
