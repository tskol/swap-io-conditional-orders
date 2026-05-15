import { BN } from "@coral-xyz/anchor";
import { OrderAccount } from "../ordo-client";

export function calcMinOutputAmountFromOrder(
  inputAmount: BN,
  orderAccount: Pick<OrderAccount, "expectedOutputAmount" | "initialInputAmount">,
): BN {
  const numerator = new BN(inputAmount).mul(orderAccount.expectedOutputAmount);
  const denominator = orderAccount.initialInputAmount;
  return numerator.add(denominator).sub(new BN(1)).div(denominator);
}
