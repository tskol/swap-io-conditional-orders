import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../tests/helpers/ordo";
import { UpdateGlobalConfigMode } from "../tests/helpers/constants";
import {
  requirePublicKey,
  SCRIPT_ALLOWED_TAKER,
  SCRIPT_COMMITMENT,
  SCRIPT_GLOBAL_CONFIG,
} from "./config";

async function main() {
  const envProvider = anchor.AnchorProvider.env();
  const connection = new web3.Connection(envProvider.connection.rpcEndpoint, { commitment: SCRIPT_COMMITMENT });
  const provider = new anchor.AnchorProvider(connection, envProvider.wallet, {
    commitment: SCRIPT_COMMITMENT,
    preflightCommitment: SCRIPT_COMMITMENT,
  });
  anchor.setProvider(provider);

  const user = provider.wallet as anchor.Wallet;
  console.log("Execute script from wallet: ", user.publicKey.toBase58());

  const ordoHelper = new OrdoHelper(provider);
  ordoHelper.setGlobalConfig(SCRIPT_GLOBAL_CONFIG ?? requirePublicKey("ORDO_GLOBAL_CONFIG"));
  const allowedTaker = SCRIPT_ALLOWED_TAKER ?? requirePublicKey("ORDO_ALLOWED_TAKER");

  const { signature } = await ordoHelper.updateGlobalConfig({
    payer: user,
    mode: UpdateGlobalConfigMode.UpdateCounterparty,
    value: Array.from(allowedTaker.toBuffer()),
  });

  const blockhash = await connection.getLatestBlockhash(SCRIPT_COMMITMENT);
  await connection.confirmTransaction({ signature, ...blockhash }, SCRIPT_COMMITMENT);
  console.log("Allowed taker updated: ", allowedTaker.toBase58());
  console.log("Signature: ", signature);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
