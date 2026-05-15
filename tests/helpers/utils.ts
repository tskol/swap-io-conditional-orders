import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { web3 } from "@coral-xyz/anchor";
import { assert } from "chai";

export async function airdrop(
  account: web3.PublicKey,
  balance = web3.LAMPORTS_PER_SOL,
  commitment: web3.Commitment = "confirmed",
) {
  const provider = anchor.getProvider();
  const connection = provider.connection;

  const signature = await connection.requestAirdrop(account, balance);
  const blockhash = await connection.getLatestBlockhash(commitment);
  await connection.confirmTransaction({ signature, ...blockhash }, commitment);
}

export async function findLargestTokenAccount(args: {
  connection: web3.Connection;
  owner: web3.PublicKey;
  mint: web3.PublicKey;
}): Promise<web3.PublicKey> {
  const tokenAccounts = await args.connection.getTokenAccountsByOwner(
    args.owner,
    { mint: args.mint },
  );

  let largestBalance = new anchor.BN(0);
  let largestTokenAccount: web3.PublicKey | null = null;

  for (const tokenAccount of tokenAccounts.value) {
    const tokenAccountData = await args.connection.getTokenAccountBalance(
      tokenAccount.pubkey,
    );
    const balance = new anchor.BN(tokenAccountData.value.amount);
    if (balance.cmp(largestBalance) > 0) {
      largestBalance = balance;
      largestTokenAccount = tokenAccount.pubkey;
    }
  }

  if (largestTokenAccount == null) {
    throw new Error("Largest token account not found");
  }

  return largestTokenAccount;
}

export function generateRandomOrdoAccounts() {
  const adminKeypair = web3.Keypair.generate();
  const adminWallet = new anchor.Wallet(adminKeypair);
  const admin = adminKeypair.publicKey;

  const treasuryKeypair = web3.Keypair.generate();
  const treasuryWallet = new anchor.Wallet(treasuryKeypair);
  const treasury = treasuryKeypair.publicKey;

  const yieldClaimerKeypair = web3.Keypair.generate();
  const yieldClaimerWallet = new anchor.Wallet(yieldClaimerKeypair);
  const yieldClaimer = yieldClaimerKeypair.publicKey;

  const strategyManagerKeypair = web3.Keypair.generate();
  const strategyManagerWallet = new anchor.Wallet(strategyManagerKeypair);
  const strategyManager = strategyManagerKeypair.publicKey;

  return {
    publicKeys: {
      admin,
      treasury,
      yieldClaimer,
      strategyManager,
    },
    keypairs: {
      admin: adminKeypair,
      treasury: treasuryKeypair,
      yieldClaimer: yieldClaimerKeypair,
      strategyManager: strategyManagerKeypair,
    },
    wallets: {
      admin: adminWallet,
      treasury: treasuryWallet,
      yieldClaimer: yieldClaimerWallet,
      strategyManager: strategyManagerWallet,
    },
  };
}

export async function createMintWithInitialBalance(args: {
  connection: web3.Connection;
  payer: web3.Keypair;
  recipient: web3.PublicKey;
  decimals: number;
  initialBalance: number;
  tokenProgram?: web3.PublicKey;
}): Promise<{
  mintAuthority: web3.Keypair;
  mint: web3.PublicKey;
  recipientTokenAccount: web3.PublicKey;
}> {
  const mintAuthority = web3.Keypair.generate();
  const mint = await spl.createMint(
    args.connection,
    args.payer,
    mintAuthority.publicKey,
    null,
    args.decimals,
    undefined,
    undefined,
    args.tokenProgram,
  );

  await spl.createAssociatedTokenAccount(
    args.connection,
    args.payer,
    mint,
    args.recipient,
    undefined,
    args.tokenProgram,
    undefined,
    true,
  );

  const recipientTokenAccountPubkey = spl.getAssociatedTokenAddressSync(
    mint,
    args.recipient,
    true,
    args.tokenProgram,
  );

  await spl.mintTo(
    args.connection,
    args.payer,
    mint,
    recipientTokenAccountPubkey,
    mintAuthority,
    args.initialBalance,
    undefined,
    undefined,
    args.tokenProgram,
  );

  return {
    mintAuthority,
    mint,
    recipientTokenAccount: recipientTokenAccountPubkey,
  };
}

export async function expectRejects<T>(promise: Promise<T>, withError: string) {
  await promise.then(
    () => Promise.reject(new Error("Expected to error")),
    (e: web3.SendTransactionError) => {
        if (e.logs) {
            assert.ok(e.logs.some((log) => log.includes(withError)));
        } else {
            assert.fail("No logs found");
        }
    },
  );
}
