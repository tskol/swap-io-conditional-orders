import { BN, web3 } from "@coral-xyz/anchor";

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`Missing required env var: ${name}`);
  }
  return value;
}

function optionalPublicKey(name: string): web3.PublicKey | undefined {
  const value = process.env[name];
  return value ? new web3.PublicKey(value) : undefined;
}

function requirePublicKey(name: string): web3.PublicKey {
  return new web3.PublicKey(requireEnv(name));
}

function envBn(name: string, fallback?: string): BN {
  const value = process.env[name] ?? fallback;
  if (value == null) {
    throw new Error(`Missing required env var: ${name}`);
  }
  return new BN(value);
}

export const SCRIPT_COMMITMENT: web3.Commitment = "confirmed";
export const SCRIPT_GLOBAL_CONFIG = optionalPublicKey("ORDO_GLOBAL_CONFIG");
export const SCRIPT_ORDER = optionalPublicKey("ORDO_ORDER");
export const SCRIPT_ALLOWED_TAKER = optionalPublicKey("ORDO_ALLOWED_TAKER");
export const SCRIPT_INPUT_MINT = optionalPublicKey("ORDO_INPUT_MINT");
export const SCRIPT_OUTPUT_MINT = optionalPublicKey("ORDO_OUTPUT_MINT");
export const SCRIPT_MINT = optionalPublicKey("ORDO_MINT");
export const SCRIPT_FEED_ID = process.env.ORDO_FEED_ID;
export const SCRIPT_INPUT_AMOUNT = envBn("ORDO_INPUT_AMOUNT", "1000000000");
export const SCRIPT_OUTPUT_AMOUNT = envBn("ORDO_OUTPUT_AMOUNT", "1000000000");
export const SCRIPT_TP_OUTPUT_AMOUNT = envBn("ORDO_TP_OUTPUT_AMOUNT", "0");
export const SCRIPT_SL_OUTPUT_AMOUNT = envBn("ORDO_SL_OUTPUT_AMOUNT", "0");

export {
  requireEnv,
  requirePublicKey,
};
