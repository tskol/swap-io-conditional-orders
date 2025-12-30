import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import {
  PublicKey,
} from "@solana/web3.js";
import { LimoHelper } from "../tests/helpers/limo";
import { BN } from "@coral-xyz/anchor";

const COMMITMENT: web3.Commitment = 'confirmed';

const GLOBAL_CONFIG = new PublicKey("");
const ORDER = new PublicKey("");
const INPUT_AMOUNT = new BN(1000000000);

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

    const minOutputAmount = await limoHelper.calcOrderMinOutputAmount(INPUT_AMOUNT, ORDER);

    const { signatures } = await limoHelper.takeOrder({
        taker: user,
        order: ORDER,
        inputAmount: new BN(INPUT_AMOUNT),
        minOutputAmount: minOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
    });

    const blockhash = await connection.getLatestBlockhash(COMMITMENT);
    await connection.confirmTransaction({ signature: signatures[0], ...blockhash }, COMMITMENT);

    console.log("Order taken: ", signatures[0]);
    if (signatures.length > 1) {
        for (let index = 1; index < signatures.length; index++) {
            const signature = signatures[index];
            await connection.confirmTransaction({ signature, ...blockhash }, COMMITMENT);
            console.log(`Signature ${index + 1}: ${signature}`);
        }
    }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });