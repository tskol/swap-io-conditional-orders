import * as anchor from "@coral-xyz/anchor";
import { BN, web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../ordo-client";
import { OrderType, UpdateOrderMode } from "../ordo-constants";

async function markExecutable(args: {
  ordoHelper: OrdoHelper;
  maker: anchor.Wallet;
  taker: web3.PublicKey;
  order: web3.PublicKey;
}) {
  await args.ordoHelper.updateOrder({
    maker: args.maker,
    order: args.order,
    mode: UpdateOrderMode.UpdatePermissionless,
    value: new BN(1).toBuffer(),
  });
  await args.ordoHelper.updateOrder({
    maker: args.maker,
    order: args.order,
    mode: UpdateOrderMode.UpdateCounterparty,
    value: args.taker.toBuffer(),
  });
}

export async function createExecutableVanillaFillOrder(args: {
  ordoHelper: OrdoHelper;
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

  await markExecutable({
    ordoHelper: args.ordoHelper,
    maker: args.maker,
    taker: args.taker.publicKey,
    order,
  });

  return order;
}

export async function createExecutableParentFillOrder(args: {
  ordoHelper: OrdoHelper;
  maker: anchor.Wallet;
  taker: web3.Keypair;
  inputMint: web3.PublicKey;
  outputMint: web3.PublicKey;
  inputAmount: BN;
  outputAmount: BN;
  tpOutputAmount: BN;
  slOutputAmount: BN;
  activeDurationSeconds: BN;
}): Promise<web3.PublicKey> {
  const { order } = await args.ordoHelper.createOrder({
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

  await markExecutable({
    ordoHelper: args.ordoHelper,
    maker: args.maker,
    taker: args.taker.publicKey,
    order,
  });

  return order;
}
