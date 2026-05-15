import * as anchor from "@coral-xyz/anchor";
import {
  requirePublicKey,
  SCRIPT_INPUT_AMOUNT,
  SCRIPT_ORDER,
} from "./config";
import {
  applyScriptGlobalConfig,
  confirmSignature,
  createScriptContext,
} from "./runtime";

async function main() {
    const { connection, wallet, ordoHelper } = createScriptContext();
    console.log("Execute script from wallet: ", wallet.publicKey.toBase58());
    applyScriptGlobalConfig(ordoHelper);
    const order = SCRIPT_ORDER ?? requirePublicKey("ORDO_ORDER");

    const minOutputAmount = await ordoHelper.calcOrderMinOutputAmount(SCRIPT_INPUT_AMOUNT, order);

    const { signatures } = await ordoHelper.takeOrder({
        taker: wallet,
        order,
        inputAmount: SCRIPT_INPUT_AMOUNT,
        minOutputAmount: minOutputAmount,
        tipAmountPermissionlessTaking: new anchor.BN(0),
    });
    await confirmSignature(connection, signatures[0]);

    console.log("Order taken: ", signatures[0]);
    if (signatures.length > 1) {
        for (let index = 1; index < signatures.length; index++) {
            const signature = signatures[index];
            await confirmSignature(connection, signature);
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
