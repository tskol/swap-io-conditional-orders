import { confirmSignature, createScriptContext } from "./runtime";

async function main() {
    const { connection, wallet, ordoHelper } = createScriptContext();
    console.log("Execute script from wallet: ", wallet.publicKey.toBase58());

    const { signature, globalConfig } = await ordoHelper.initializeGlobalConfig({
        payer: wallet,
    });
    await confirmSignature(connection, signature);

    console.log("Global config initialized: ", globalConfig.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
