import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { BN, web3 } from "@coral-xyz/anchor";
import { OrdoHelper } from "../ordo-client";
import { OrderType, UpdateOrderMode } from "../ordo-constants";

type ChildOrderSet = {
  order: web3.PublicKey;
  tpOrder: web3.PublicKey;
  slOrder: web3.PublicKey;
};

type ExecutableChildOrderArgs = {
  ordoHelper: OrdoHelper;
  maker: anchor.Wallet;
  taker: web3.Keypair;
  inputMint: web3.PublicKey;
  outputMint: web3.PublicKey;
  inputAmount: BN;
  outputAmount: BN;
  tpOutputAmount: BN;
  slOutputAmount: BN;
  activeDurationSeconds?: BN;
};

async function markOrderExecutable(args: {
  ordoHelper: OrdoHelper;
  maker: anchor.Wallet;
  order: web3.PublicKey;
  taker: web3.PublicKey;
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

export async function ensureChildFlowTokenAccounts(args: {
  connection: web3.Connection;
  taker: web3.Keypair;
  maker: web3.Keypair;
  inputMint: web3.PublicKey;
  outputMint: web3.PublicKey;
}) {
  await spl.createAssociatedTokenAccountIdempotent(
    args.connection,
    args.taker,
    args.inputMint,
    args.taker.publicKey,
  );

  await spl.createAssociatedTokenAccountIdempotent(
    args.connection,
    args.maker,
    args.outputMint,
    args.maker.publicKey,
  );
}

export async function calcProRataMinOutputAmount(args: {
  ordoHelper: OrdoHelper;
  inputAmount: BN;
  order: web3.PublicKey;
}): Promise<BN> {
  const orderAccount = await args.ordoHelper.getOrderAccount(args.order);
  const numerator = new BN(args.inputAmount).mul(
    orderAccount.expectedOutputAmount,
  );
  const denominator = orderAccount.initialInputAmount;
  return numerator.add(denominator).sub(new BN(1)).div(denominator);
}

export async function createExecutableChildOrderSet(
  args: ExecutableChildOrderArgs,
): Promise<ChildOrderSet> {
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

  await markOrderExecutable({
    ordoHelper: args.ordoHelper,
    maker: args.maker,
    order,
    taker: args.taker.publicKey,
  });
  await markOrderExecutable({
    ordoHelper: args.ordoHelper,
    maker: args.maker,
    order: tpOrder,
    taker: args.taker.publicKey,
  });
  await markOrderExecutable({
    ordoHelper: args.ordoHelper,
    maker: args.maker,
    order: slOrder,
    taker: args.taker.publicKey,
  });

  return { order, tpOrder, slOrder };
}
