import { OrderType } from "../tests/helpers/constants";
import {
  requirePublicKey,
  SCRIPT_INPUT_AMOUNT,
  SCRIPT_INPUT_MINT,
  SCRIPT_OUTPUT_AMOUNT,
  SCRIPT_OUTPUT_MINT,
  SCRIPT_SL_OUTPUT_AMOUNT,
  SCRIPT_TP_OUTPUT_AMOUNT,
} from "./config";
import {
  applyScriptGlobalConfig,
  confirmSignature,
  createScriptContext,
} from "./runtime";

const ORDER_TYPE = OrderType.Vanilla;

async function main() {
    const { connection, wallet, ordoHelper } = createScriptContext();
    console.log("Execute script from wallet: ", wallet.publicKey.toBase58());
    applyScriptGlobalConfig(ordoHelper);

    const { signature, order } = await ordoHelper.createOrder({
        maker: wallet,
        inputMint: SCRIPT_INPUT_MINT ?? requirePublicKey("ORDO_INPUT_MINT"),
        outputMint: SCRIPT_OUTPUT_MINT ?? requirePublicKey("ORDO_OUTPUT_MINT"),
        inputAmount: SCRIPT_INPUT_AMOUNT,
        outputAmount: SCRIPT_OUTPUT_AMOUNT,
        orderType: ORDER_TYPE,
        tpOutputAmount: SCRIPT_TP_OUTPUT_AMOUNT,
        slOutputAmount: SCRIPT_SL_OUTPUT_AMOUNT,
    });
    await confirmSignature(connection, signature);

    console.log("Order created: ", order.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
