import {
  requirePublicKey,
  SCRIPT_MINT,
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

    const { signature, vault, feeVault } = await ordoHelper.initializeVault({
        payer: wallet,
        mint: SCRIPT_MINT ?? requirePublicKey("ORDO_MINT")
    });
    await confirmSignature(connection, signature);

    console.log("Vault initialized: ", vault.toBase58());
    console.log("Fee vault initialized: ", feeVault.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
