import { submitConfiguredOrder } from "./order-actions";

async function main() {
    const { wallet, order } = await submitConfiguredOrder();
    console.log("Execute script from wallet: ", wallet.publicKey.toBase58());
    console.log("Order created: ", order.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
