import {
  requireEnv,
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

    const { signature, oraclePool } = await ordoHelper.initializeOraclePool({
        payer: wallet,
        mint: SCRIPT_MINT ?? requirePublicKey("ORDO_MINT"),
        feedId: requireEnv("ORDO_FEED_ID"),
    });
    await confirmSignature(connection, signature);

    console.log("Oracle pool initialized: ", oraclePool.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
