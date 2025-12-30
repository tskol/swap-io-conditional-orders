import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { LimoHelper } from "../tests/helpers/limo";

const COMMITMENT: web3.Commitment = 'confirmed';

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

    const limoHelper = new LimoHelper(provider);

    const { signature, globalConfig } = await limoHelper.initializeGlobalConfig({
        payer: user,
    });

    const blockhash = await connection.getLatestBlockhash(COMMITMENT);
    await connection.confirmTransaction({ signature, ...blockhash }, COMMITMENT);

    console.log("Global config initialized: ", globalConfig.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });