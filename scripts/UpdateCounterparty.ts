import { UpdateGlobalConfigMode } from "../tests/helpers/constants";
import {
  requirePublicKey,
  SCRIPT_ALLOWED_TAKER,
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
  const allowedTaker = SCRIPT_ALLOWED_TAKER ?? requirePublicKey("ORDO_ALLOWED_TAKER");

  const { signature } = await ordoHelper.updateGlobalConfig({
    payer: wallet,
    mode: UpdateGlobalConfigMode.UpdateCounterparty,
    value: Array.from(allowedTaker.toBuffer()),
  });

  await confirmSignature(connection, signature);
  console.log("Allowed taker updated: ", allowedTaker.toBase58());
  console.log("Signature: ", signature);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
