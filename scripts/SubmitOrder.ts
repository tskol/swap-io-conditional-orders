import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import {
  PublicKey,
} from "@solana/web3.js";
import { OrdoHelper } from "../tests/helpers/ordo";
import { OrderType } from "../tests/helpers/constants";
import { BN } from "@coral-xyz/anchor";

const COMMITMENT: web3.Commitment = 'confirmed';

const GLOBAL_CONFIG = new PublicKey("G5t5rvSjYPFfNWKjUvJ5eVU9xrb4SdhSkkiBFQRfUBNR");
const INPUT_MINT = new PublicKey("E7bZyqvN5a46AyyzQLm1ns3PCheG2qmmt9uWqVFPWDvo");
const OUTPUT_MINT = new PublicKey("5WsTaQwxNhXNyCvzTeGxyYynGL1nmGwPQQCuXwCtGimn");

const INPUT_AMOUNT = new BN(1000000000);
const OUTPUT_AMOUNT = new BN(1000000000);
const ORDER_TYPE = OrderType.Vanilla;
const TP_OUTPUT_AMOUNT = new BN(0);
const SL_OUTPUT_AMOUNT = new BN(0);

async function main() {
    const envProvider = anchor.AnchorProvider.env();
    const connection = new web3.Connection(envProvider.connection.rpcEndpoint, { commitment: COMMITMENT });
    const provider = new anchor.AnchorProvider(connection, envProvider.wallet, {
        commitment: COMMITMENT,
        preflightCommitment: COMMITMENT,
    });
    anchor.setProvider(provider);

    const user = provider.wallet as anchor.Wallet;
    console.log("Execute script from wallet: ", user.publicKey.toBase58());

    const ordoHelper = new OrdoHelper(provider);
    ordoHelper.setGlobalConfig(GLOBAL_CONFIG);

    const { signature, order } = await ordoHelper.createOrder({
        maker: user,
        inputMint: INPUT_MINT,
        outputMint: OUTPUT_MINT,
        inputAmount: new BN(INPUT_AMOUNT),
        outputAmount: new BN(OUTPUT_AMOUNT),
        orderType: ORDER_TYPE,
        tpOutputAmount: new BN(TP_OUTPUT_AMOUNT),
        slOutputAmount: new BN(SL_OUTPUT_AMOUNT),
    });

    const blockhash = await connection.getLatestBlockhash(COMMITMENT);
    await connection.confirmTransaction({ signature, ...blockhash }, COMMITMENT);

    console.log("Order created: ", order.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });