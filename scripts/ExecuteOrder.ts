import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../tests/helpers/ordo";
import {
  requirePublicKey,
  SCRIPT_COMMITMENT,
  SCRIPT_GLOBAL_CONFIG,
  SCRIPT_INPUT_AMOUNT,
  SCRIPT_ORDER,
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
    const order = SCRIPT_ORDER ?? requirePublicKey("ORDO_ORDER");

    const minOutputAmount = await ordoHelper.calcOrderMinOutputAmount(SCRIPT_INPUT_AMOUNT, order);

    const { signatures } = await ordoHelper.takeOrder({
        taker: user,
        order,
        inputAmount: SCRIPT_INPUT_AMOUNT,
        minOutputAmount: minOutputAmount,
        tipAmountPermissionlessTaking: new anchor.BN(0),
    });

    const blockhash = await connection.getLatestBlockhash(SCRIPT_COMMITMENT);
    await connection.confirmTransaction({ signature: signatures[0], ...blockhash }, SCRIPT_COMMITMENT);

    console.log("Order taken: ", signatures[0]);
    if (signatures.length > 1) {
        for (let index = 1; index < signatures.length; index++) {
            const signature = signatures[index];
            await connection.confirmTransaction({ signature, ...blockhash }, SCRIPT_COMMITMENT);
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
