import * as anchor from "@coral-xyz/anchor";
import { web3, BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { OrdoHelper } from "../tests/helpers/ordo";
import { UpdateGlobalConfigMode } from "../tests/helpers/constants";

const COMMITMENT: web3.Commitment = "confirmed";
const GLOBAL_CONFIG = new PublicKey("G5t5rvSjYPFfNWKjUvJ5eVU9xrb4SdhSkkiBFQRfUBNR");
const ALLOWED_TAKER = new PublicKey("CNx2DU7PJiai3csVDsdSpTSr46mKJUuYdpqfiPHS7AcU");

async function main() {
  const envProvider = anchor.AnchorProvider.env();
  const connection = new web3.Connection(envProvider.connection.rpcEndpoint, { commitment: COMMITMENT });
  const provider = new anchor.AnchorProvider(connection, envProvider.wallet, {
    commitment: COMMITMENT,
    preflightCommitment: COMMITMENT,
  });
  anchor.setProvider(provider);

  const user = provider.wallet as anchor.Wallet;
  console.log("Execute script from wallet: ", user.publicKey.toBase58());

  const ordoHelper = new OrdoHelper(provider);
  ordoHelper.setGlobalConfig(GLOBAL_CONFIG);

  const { signature } = await ordoHelper.updateGlobalConfig({
    payer: user,
    mode: UpdateGlobalConfigMode.UpdateCounterparty,
    value: Array.from(ALLOWED_TAKER.toBuffer()),
  });

  const blockhash = await connection.getLatestBlockhash(COMMITMENT);
  await connection.confirmTransaction({ signature, ...blockhash }, COMMITMENT);
  console.log("Allowed taker updated: ", ALLOWED_TAKER.toBase58());
  console.log("Signature: ", signature);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });