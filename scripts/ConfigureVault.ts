import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../tests/helpers/ordo";
import {
  requirePublicKey,
  SCRIPT_COMMITMENT,
  SCRIPT_GLOBAL_CONFIG,
  SCRIPT_MINT,
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

    const { signature, vault, feeVault } = await ordoHelper.initializeVault({
        payer: user,
        mint: SCRIPT_MINT ?? requirePublicKey("ORDO_MINT")
    });

    const blockhash = await connection.getLatestBlockhash(SCRIPT_COMMITMENT);
    await connection.confirmTransaction({ signature, ...blockhash }, SCRIPT_COMMITMENT);

    console.log("Vault initialized: ", vault.toBase58());
    console.log("Fee vault initialized: ", feeVault.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
