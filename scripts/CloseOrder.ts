import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import {
  PublicKey,
} from "@solana/web3.js";
import { LimoHelper } from "../tests/helpers/limo";

const COMMITMENT: web3.Commitment = 'confirmed';

const GLOBAL_CONFIG = new PublicKey("");
const ORDER = new PublicKey("");

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
    limoHelper.setGlobalConfig(GLOBAL_CONFIG);

    const { signature } = await limoHelper.closeOrder({
        maker: user,
        order: ORDER,
    });

    const blockhash = await connection.getLatestBlockhash(COMMITMENT);
    await connection.confirmTransaction({ signature, ...blockhash }, COMMITMENT);

    console.log("Order closed: ", signature);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });