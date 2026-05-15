import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "./ordo-client";
import { airdrop, createMintWithInitialBalance } from "./token-fixtures";

export const STABLE_PRICE_FEED =
  "0x8b1e8e689fbb95ece35155a8b42cb9f1b14208a2f6507866a9a90e8dc955289a";

export function createFlowProvider(
  commitment: web3.Commitment = "confirmed",
): {
  commitment: web3.Commitment;
  connection: web3.Connection;
  provider: anchor.AnchorProvider;
} {
  const envProvider = anchor.AnchorProvider.env();
  const connection = new web3.Connection(
    envProvider.connection.rpcEndpoint,
    commitment,
  );
  const provider = new anchor.AnchorProvider(connection, envProvider.wallet, {
    commitment,
    preflightCommitment: commitment,
  });

  anchor.setProvider(provider);
  return { commitment, connection, provider };
}

function createActorWallet() {
  const keypair = web3.Keypair.generate();
  return {
    keypair,
    wallet: new anchor.Wallet(keypair),
  };
}

export function createFlowActors() {
  const payerActor = createActorWallet();
  const makerActor = createActorWallet();
  const takerActor = createActorWallet();
  const tokenCreatorActor = createActorWallet();

  return {
    payer: payerActor.keypair,
    payerWallet: payerActor.wallet,
    maker: makerActor.keypair,
    makerWallet: makerActor.wallet,
    taker: takerActor.keypair,
    takerWallet: takerActor.wallet,
    tokenCreator: tokenCreatorActor.keypair,
    tokenCreatorWallet: tokenCreatorActor.wallet,
  };
}

export async function ensureLocalValidator(connection: web3.Connection) {
  try {
    await connection.getVersion();
  } catch (error: any) {
    if (
      error.message?.includes("ECONNREFUSED") ||
      error.message?.includes("fetch failed")
    ) {
      throw new Error(
        "Local Solana validator is not running. " +
          "Please start it with: solana-test-validator " +
          "or use 'anchor test' which handles this automatically.",
      );
    }
    throw error;
  }
}

export async function bootstrapOrderFlow(args: {
  provider: anchor.AnchorProvider;
  ordoHelper: OrdoHelper;
  payer: web3.Keypair;
  payerWallet: anchor.Wallet;
  maker: web3.Keypair;
  taker: web3.Keypair;
  tokenCreator: web3.Keypair;
  inputDecimals?: number;
  outputDecimals?: number;
  makerInputBalance: number;
  takerOutputBalance: number;
  seedCounterpartyBalances?: {
    inputAmount: number;
    outputAmount: number;
  };
  feedId?: string;
}): Promise<{
  inputMint: web3.PublicKey;
  outputMint: web3.PublicKey;
  makerInputAta: web3.PublicKey;
  takerOutputAta: web3.PublicKey;
  takerInputAta: web3.PublicKey;
  makerOutputAta: web3.PublicKey;
}> {
  const participants = [
    args.payer.publicKey,
    args.maker.publicKey,
    args.taker.publicKey,
    args.tokenCreator.publicKey,
  ];
  await Promise.all(
    participants.map((account) =>
      airdrop(account, 10 * web3.LAMPORTS_PER_SOL),
    ),
  );

  const { mint: inputMint, recipientTokenAccount: makerInputAta } =
    await createMintWithInitialBalance({
      connection: args.provider.connection,
      payer: args.tokenCreator,
      recipient: args.maker.publicKey,
      decimals: args.inputDecimals ?? 6,
      initialBalance: args.makerInputBalance,
    });
  const { mint: outputMint, recipientTokenAccount: takerOutputAta } =
    await createMintWithInitialBalance({
      connection: args.provider.connection,
      payer: args.tokenCreator,
      recipient: args.taker.publicKey,
      decimals: args.outputDecimals ?? 6,
      initialBalance: args.takerOutputBalance,
    });

  const takerInputAta = await spl.createAssociatedTokenAccountIdempotent(
    args.provider.connection,
    args.tokenCreator,
    inputMint,
    args.taker.publicKey,
  );
  const makerOutputAta = await spl.createAssociatedTokenAccountIdempotent(
    args.provider.connection,
    args.tokenCreator,
    outputMint,
    args.maker.publicKey,
  );

  if (args.seedCounterpartyBalances) {
    await spl.transfer(
      args.provider.connection,
      args.maker,
      makerInputAta,
      takerInputAta,
      args.maker.publicKey,
      args.seedCounterpartyBalances.inputAmount,
    );
    await spl.transfer(
      args.provider.connection,
      args.taker,
      takerOutputAta,
      makerOutputAta,
      args.taker.publicKey,
      args.seedCounterpartyBalances.outputAmount,
    );
  }

  await args.ordoHelper.initializeGlobalConfig({
    payer: args.payerWallet,
  });
  await args.ordoHelper.initializeVault({
    payer: args.payerWallet,
    mint: inputMint,
  });
  await args.ordoHelper.initializeVault({
    payer: args.payerWallet,
    mint: outputMint,
  });

  const feedId = args.feedId ?? STABLE_PRICE_FEED;
  await args.ordoHelper.initializeOraclePool({
    payer: args.payerWallet,
    mint: inputMint,
    feedId,
  });
  await args.ordoHelper.initializeOraclePool({
    payer: args.payerWallet,
    mint: outputMint,
    feedId,
  });

  return {
    inputMint,
    outputMint,
    makerInputAta,
    takerOutputAta,
    takerInputAta,
    makerOutputAta,
  };
}
