import { exitConfiguredOrder } from "./order-actions";

async function main() {
    const { wallet, signature } = await exitConfiguredOrder();
    console.log("Execute script from wallet: ", wallet.publicKey.toBase58());
    console.log("Order closed: ", signature);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
