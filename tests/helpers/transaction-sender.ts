import { web3 } from "@coral-xyz/anchor";
import { Wallet } from "@coral-xyz/anchor/dist/cjs/provider";

export class TransactionSender {
  public readonly connection: web3.Connection;

  constructor(connection: web3.Connection) {
    this.connection = connection;
  }

  protected async sendTransaction(
    wallet: Wallet,
    ixs: web3.TransactionInstruction[],
    signers?: web3.Signer[],
  ): Promise<{ signature: string }> {
    const recentBlockhash = await this.connection.getLatestBlockhash();
    const message = new web3.TransactionMessage({
      instructions: ixs,
      payerKey: wallet.publicKey,
      recentBlockhash: recentBlockhash.blockhash,
    }).compileToV0Message();

    const tx = new web3.VersionedTransaction(message);
    if (signers) {
      tx.sign(signers);
    }

    const signedTx = await wallet.signTransaction(tx);
    const signature = await this.connection.sendTransaction(signedTx);
    const blockhash = await this.connection.getLatestBlockhash();
    await this.connection.confirmTransaction({ signature, ...blockhash });

    return { signature };
  }
}
