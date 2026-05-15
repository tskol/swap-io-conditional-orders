import { Wallet, web3 } from "@coral-xyz/anchor";
import { PriceServiceConnection } from "@pythnetwork/price-service-client";

export function encodeFeedId(feedId: number[]): string {
  return `0x${feedId
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("")}`;
}

export async function executeStopLossOrder(args: {
  connection: web3.Connection;
  taker: Wallet;
  pythConnection: PriceServiceConnection;
  inputFeedId: string;
  outputFeedId: string;
  buildInstruction: (
    inputPriceUpdate: web3.PublicKey,
    outputPriceUpdate: web3.PublicKey,
  ) => Promise<web3.TransactionInstruction>;
}): Promise<string[]> {
  const { PythSolanaReceiver } = await import(
    "@pythnetwork/pyth-solana-receiver"
  );
  const pyth = new PythSolanaReceiver({
    connection: args.connection,
    wallet: args.taker,
  });

  const priceUpdateData = await args.pythConnection.getLatestVaas([
    args.inputFeedId,
    args.outputFeedId,
  ]);
  const builder = pyth.newTransactionBuilder({ closeUpdateAccounts: true });
  await builder.addPostPriceUpdates(priceUpdateData);

  await builder.addPriceConsumerInstructions(async (getPriceUpdateAccount) => [
    {
      instruction: await args.buildInstruction(
        getPriceUpdateAccount(args.inputFeedId),
        getPriceUpdateAccount(args.outputFeedId),
      ),
      signers: [],
    },
  ]);

  const transactions = await builder.buildVersionedTransactions({});
  for (const transaction of transactions) {
    transaction.tx.sign(transaction.signers);
  }

  const signedTransactions = await args.taker.signAllTransactions(
    transactions.map((transaction) => transaction.tx),
  );

  const signatures: string[] = [];
  for (const signedTransaction of signedTransactions) {
    const signature = await args.connection.sendTransaction(signedTransaction);
    const blockhash = await args.connection.getLatestBlockhash();
    await args.connection.confirmTransaction({ signature, ...blockhash });
    signatures.push(signature);
  }

  return signatures;
}
