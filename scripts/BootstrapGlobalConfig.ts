import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../tests/helpers/ordo";
import { SCRIPT_COMMITMENT } from "./config";

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

    const { signature, globalConfig } = await ordoHelper.initializeGlobalConfig({
        payer: user,
    });

    const blockhash = await connection.getLatestBlockhash(SCRIPT_COMMITMENT);
    await connection.confirmTransaction({ signature, ...blockhash }, SCRIPT_COMMITMENT);

    console.log("Global config initialized: ", globalConfig.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
