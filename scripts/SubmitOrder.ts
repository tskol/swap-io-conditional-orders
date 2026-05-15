import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../tests/helpers/ordo";
import { OrderType } from "../tests/helpers/constants";
import {
  requirePublicKey,
  SCRIPT_COMMITMENT,
  SCRIPT_GLOBAL_CONFIG,
  SCRIPT_INPUT_AMOUNT,
  SCRIPT_INPUT_MINT,
  SCRIPT_OUTPUT_AMOUNT,
  SCRIPT_OUTPUT_MINT,
  SCRIPT_SL_OUTPUT_AMOUNT,
  SCRIPT_TP_OUTPUT_AMOUNT,
} from "./config";

const ORDER_TYPE = OrderType.Vanilla;

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

    const { signature, order } = await ordoHelper.createOrder({
        maker: user,
        inputMint: SCRIPT_INPUT_MINT ?? requirePublicKey("ORDO_INPUT_MINT"),
        outputMint: SCRIPT_OUTPUT_MINT ?? requirePublicKey("ORDO_OUTPUT_MINT"),
        inputAmount: SCRIPT_INPUT_AMOUNT,
        outputAmount: SCRIPT_OUTPUT_AMOUNT,
        orderType: ORDER_TYPE,
        tpOutputAmount: SCRIPT_TP_OUTPUT_AMOUNT,
        slOutputAmount: SCRIPT_SL_OUTPUT_AMOUNT,
    });

    const blockhash = await connection.getLatestBlockhash(SCRIPT_COMMITMENT);
    await connection.confirmTransaction({ signature, ...blockhash }, SCRIPT_COMMITMENT);

    console.log("Order created: ", order.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
