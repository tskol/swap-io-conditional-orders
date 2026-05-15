import {
  requirePublicKey,
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

    const { signature } = await ordoHelper.closeOrder({
        closer: wallet,
        order: SCRIPT_ORDER ?? requirePublicKey("ORDO_ORDER"),
    });
    await confirmSignature(connection, signature);

    console.log("Order closed: ", signature);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
