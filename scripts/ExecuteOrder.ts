import { executeConfiguredOrder } from "./order-actions";

async function main() {
    const { wallet, signatures } = await executeConfiguredOrder();
    console.log("Execute script from wallet: ", wallet.publicKey.toBase58());
    console.log("Order taken: ", signatures[0]);
    signatures.slice(1).forEach((signature, index) => {
      console.log(`Signature ${index + 2}: ${signature}`);
    });
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
