import * as anchor from "@coral-xyz/anchor";
import { BN, web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../ordo-client";
import { OrderType, UpdateGlobalConfigMode } from "../ordo-constants";

type ParentChildOrders = {
  order: web3.PublicKey;
  tpOrder: web3.PublicKey;
  slOrder: web3.PublicKey;
};

async function syncCloseCounterparty(args: {
  ordoHelper: OrdoHelper;
  payer: anchor.Wallet;
  taker: web3.PublicKey;
}) {
  await args.ordoHelper.updateGlobalConfig({
    payer: args.payer,
    mode: UpdateGlobalConfigMode.UpdateCounterparty,
    value: Array.from(args.taker.toBuffer()),
  });
}

export async function waitForOrderExpiry(activeDurationSeconds: BN) {
  const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
  await new Promise((resolve) => setTimeout(resolve, waitMs));
}

export async function createVanillaCloseOrder(args: {
  ordoHelper: OrdoHelper;
  payer: anchor.Wallet;
  maker: anchor.Wallet;
  taker: web3.Keypair;
  inputMint: web3.PublicKey;
  outputMint: web3.PublicKey;
  inputAmount: BN;
  outputAmount: BN;
  activeDurationSeconds: BN;
}): Promise<web3.PublicKey> {
  const { order } = await args.ordoHelper.createOrder({
    maker: args.maker,
    inputMint: args.inputMint,
    outputMint: args.outputMint,
    inputAmount: args.inputAmount,
    outputAmount: args.outputAmount,
    orderType: OrderType.Vanilla,
    activeDurationSeconds: args.activeDurationSeconds,
  });

  await syncCloseCounterparty({
    ordoHelper: args.ordoHelper,
    payer: args.payer,
    taker: args.taker.publicKey,
  });

  return order;
}

export async function createParentCloseOrder(args: {
  ordoHelper: OrdoHelper;
  payer: anchor.Wallet;
  maker: anchor.Wallet;
  taker: web3.Keypair;
  inputMint: web3.PublicKey;
  outputMint: web3.PublicKey;
  inputAmount: BN;
  outputAmount: BN;
  tpOutputAmount: BN;
  slOutputAmount: BN;
  activeDurationSeconds: BN;
}): Promise<ParentChildOrders> {
  const { order, tpOrder, slOrder } = await args.ordoHelper.createOrder({
    maker: args.maker,
    inputMint: args.inputMint,
    outputMint: args.outputMint,
    inputAmount: args.inputAmount,
    outputAmount: args.outputAmount,
    orderType: OrderType.LimitParent,
    tpOutputAmount: args.tpOutputAmount,
    slOutputAmount: args.slOutputAmount,
    activeDurationSeconds: args.activeDurationSeconds,
  });

  await syncCloseCounterparty({
    ordoHelper: args.ordoHelper,
    payer: args.payer,
    taker: args.taker.publicKey,
  });

  return { order, tpOrder, slOrder };
}
