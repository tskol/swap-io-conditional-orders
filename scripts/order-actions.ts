import * as anchor from "@coral-xyz/anchor";
import { OrderType } from "../tests/support/ordo-constants";
import {
  requirePublicKey,
  SCRIPT_INPUT_AMOUNT,
  SCRIPT_INPUT_MINT,
  SCRIPT_ORDER,
  SCRIPT_OUTPUT_AMOUNT,
  SCRIPT_OUTPUT_MINT,
  SCRIPT_SL_OUTPUT_AMOUNT,
  SCRIPT_TP_OUTPUT_AMOUNT,
} from "./config";
import {
  applyScriptGlobalConfig,
  confirmSignature,
  createScriptContext,
} from "./runtime";

export async function submitConfiguredOrder() {
  const { connection, wallet, ordoHelper } = createScriptContext();
  applyScriptGlobalConfig(ordoHelper);

  const { signature, order } = await ordoHelper.createOrder({
    maker: wallet,
    inputMint: SCRIPT_INPUT_MINT ?? requirePublicKey("ORDO_INPUT_MINT"),
    outputMint: SCRIPT_OUTPUT_MINT ?? requirePublicKey("ORDO_OUTPUT_MINT"),
    inputAmount: SCRIPT_INPUT_AMOUNT,
    outputAmount: SCRIPT_OUTPUT_AMOUNT,
    orderType: OrderType.Vanilla,
    tpOutputAmount: SCRIPT_TP_OUTPUT_AMOUNT,
    slOutputAmount: SCRIPT_SL_OUTPUT_AMOUNT,
  });
  await confirmSignature(connection, signature);

  return { wallet, signature, order };
}

export async function executeConfiguredOrder() {
  const { connection, wallet, ordoHelper } = createScriptContext();
  applyScriptGlobalConfig(ordoHelper);
  const order = SCRIPT_ORDER ?? requirePublicKey("ORDO_ORDER");

  const minOutputAmount = await ordoHelper.calcOrderMinOutputAmount(
    SCRIPT_INPUT_AMOUNT,
    order,
  );

  const { signatures } = await ordoHelper.takeOrder({
    taker: wallet,
    order,
    inputAmount: SCRIPT_INPUT_AMOUNT,
    minOutputAmount,
    tipAmountPermissionlessTaking: new anchor.BN(0),
  });
  await confirmSignature(connection, signatures[0]);

  for (const signature of signatures.slice(1)) {
    await confirmSignature(connection, signature);
  }

  return { wallet, signatures };
}

export async function exitConfiguredOrder() {
  const { connection, wallet, ordoHelper } = createScriptContext();
  applyScriptGlobalConfig(ordoHelper);

  const { signature } = await ordoHelper.closeOrder({
    closer: wallet,
    order: SCRIPT_ORDER ?? requirePublicKey("ORDO_ORDER"),
  });
  await confirmSignature(connection, signature);

  return { wallet, signature };
}
