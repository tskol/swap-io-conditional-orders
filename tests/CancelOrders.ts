import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { web3, BN } from "@coral-xyz/anchor";
import { expect } from "chai";
import {
  LimoHelper,
} from "./helpers/limo";
import {
  airdrop,
  createMintWithInitialBalance,
  expectRejects,
  generateRandomLimoAccounts,
} from "./helpers/utils";
import { OrderStatus, OrderType, LimoError, UpdateGlobalConfigMode, UpdateOrderMode } from "./helpers/constants";
// import { LimoHelper } from "./helpers/limo.js";
// import { airdrop, generateRandomLimoAccounts } from "./helpers/utils.js";

export const STABLE_PRICE_FEED =
  "0x8b1e8e689fbb95ece35155a8b42cb9f1b14208a2f6507866a9a90e8dc955289a";

const USDC_MINT = new web3.PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

export async function initializeLimo(
  provider: anchor.AnchorProvider,
  payer: web3.Keypair,
) {
  const mainAccounts = generateRandomLimoAccounts();
  const payerWallet = new anchor.Wallet(payer);

  const limoHelper = new LimoHelper(provider);
  
  await limoHelper.initializeGlobalConfig({
    payer: payerWallet,
  });

//   await airdrop(mainAccounts.publicKeys.admin);

  return {
    limoHelper,
    mainAccounts,
  };
}

describe("Safe cancellation and full unwind", () => {
    const commitment: web3.Commitment = "confirmed";
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
  
    const payer = web3.Keypair.generate();
    const payerWallet = new anchor.Wallet(payer);
    const maker = web3.Keypair.generate();
    const makerWallet = new anchor.Wallet(maker);
    const taker = web3.Keypair.generate();
    const takerWallet = new anchor.Wallet(taker);
    const tokenCreator = web3.Keypair.generate();
    const tokenCreatorWallet = new anchor.Wallet(tokenCreator);

    const limoHelper = new LimoHelper(provider);

    let inputMint: web3.PublicKey;
    let outputMint: web3.PublicKey;
  
    before(async function() {
        // Check if validator is running
        try {
            await connection.getVersion();
        } catch (error: any) {
            if (error.message?.includes("ECONNREFUSED") || error.message?.includes("fetch failed")) {
                throw new Error(
                    "Local Solana validator is not running. " +
                    "Please start it with: solana-test-validator " +
                    "or use 'anchor test' which handles this automatically."
                );
            }
            throw error;
        }
        await airdrop(payer.publicKey, 10 * web3.LAMPORTS_PER_SOL);
        await airdrop(maker.publicKey, 10 * web3.LAMPORTS_PER_SOL);
        await airdrop(taker.publicKey, 10 * web3.LAMPORTS_PER_SOL);
        await airdrop(tokenCreator.publicKey, 10 * web3.LAMPORTS_PER_SOL);

        const { mint: inputMintPubkey, recipientTokenAccount: makerInputAta } = await createMintWithInitialBalance({
            connection: provider.connection,
            payer: tokenCreator,
            recipient: maker.publicKey,
            decimals: 6,
            initialBalance: 1000000000000
        });
        const { mint: outputMintPubkey, recipientTokenAccount: takerOutputAta } = await createMintWithInitialBalance({
            connection: provider.connection,
            payer: tokenCreator,
            recipient: taker.publicKey,
            decimals: 6,
            initialBalance: 1000000000000
        });
        inputMint = inputMintPubkey;
        outputMint = outputMintPubkey;

        const takerInputAta = await spl.createAssociatedTokenAccountIdempotent(
            provider.connection,
            tokenCreator,
            inputMint,
            taker.publicKey,
        );
        const makerOutputAta = await spl.createAssociatedTokenAccountIdempotent(
            provider.connection,
            tokenCreator,
            outputMint,
            maker.publicKey,
        );

        const inputAmount = 100000000000;
        const outputAmount = 100000000000;

        await spl.transfer(
            provider.connection,
            maker,
            makerInputAta,
            takerInputAta,
            maker.publicKey,
            inputAmount,
        )
        await spl.transfer(
            provider.connection,
            taker,
            takerOutputAta,
            makerOutputAta,
            taker.publicKey,
            outputAmount,
        )

        await limoHelper.initializeGlobalConfig({
            payer: payerWallet,
        });

        await limoHelper.initializeVault({
            payer: payerWallet,
            mint: inputMint,
        });
        await limoHelper.initializeVault({
            payer: payerWallet,
            mint: outputMint,
        });

        await limoHelper.initializeOraclePool({
            payer: payerWallet,
            mint: inputMint,
            feedId: STABLE_PRICE_FEED,
        });
        await limoHelper.initializeOraclePool({
            payer: payerWallet,
            mint: outputMint,
            feedId: STABLE_PRICE_FEED,
        });
    });
  
    describe("Cancel Type A order & refund remaining input", () => {
        let order: web3.PublicKey;
        const orderInputAmount = new BN(100000000000);
        const orderOutputAmount = new BN(200000000000);

        beforeEach(async () => {
            const { signature, order: orderPubkey } = await limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: orderInputAmount,
                outputAmount: orderOutputAmount,
                orderType: OrderType.Vanilla,
            });

            order = orderPubkey;

            await limoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await limoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });
        });

        it("Cancel active Type A after cooldown", async () => {
            const makerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                maker.publicKey
            );
            const makerOutputAta = spl.getAssociatedTokenAddressSync(
                outputMint,
                maker.publicKey
            );
            const { vault: inputVaultAta } = await limoHelper.getVault(inputMint);
            const { vault: outputVaultAta } = await limoHelper.getVault(outputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const { signature } = await limoHelper.closeOrder({
                maker: makerWallet,
                order: order,
            });

            const makerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerInputAta);
            const makerOutputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const inputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const orderAccountInfo = await provider.connection.getAccountInfo(order);

            const tx = await provider.connection.getParsedTransaction(signature, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(new BN(makerInputAtaBalanceBefore.value.amount).add(orderInputAmount).toString());
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(makerOutputAtaBalanceBefore.value.amount);
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(new BN(inputVaultAtaBalanceBefore.value.amount).sub(orderInputAmount).toString());
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(outputVaultAtaBalanceBefore.value.amount);

            expect(orderAccountInfo).to.be.null;
        });

        it("Cancel Type A before cooldown rejected", async () => {
            await limoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
                value: [10],
            });

            await expectRejects(limoHelper.closeOrder({
                maker: makerWallet,
                order: order,
            }), LimoError.NotEnoughTimePassedSinceLastUpdate);

            await limoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
                value: [0],
            });
        });

        /// After cancel, order account deleted, so we can't cancel it again.
        it.skip("Cancel already Cancelled/Closed rejected", async () => {});
    });

    describe("Cancel Type B order & unwind parent + child vaults", () => {
        let order: web3.PublicKey;
        let tpOrder: web3.PublicKey;
        let slOrder: web3.PublicKey;
        const orderInputAmount = new BN(100000000000);
        const orderOutputAmount = new BN(200000000000);
        const tpOutputAmount = new BN(120000000000);
        const slOutputAmount = new BN(80000000000);

        async function calcMinOutputAmount(inputAmount: BN, order: web3.PublicKey): Promise<BN> {
            const orderAccount = await limoHelper.getOrderAccount(order);
            const numerator = new BN(inputAmount).mul(orderAccount.expectedOutputAmount);
            const denominator = orderAccount.initialInputAmount;
            return numerator.add(denominator).sub(new BN(1)).div(denominator);
        }

        beforeEach(async () => {
            const { signature, order: orderPubkey, tpOrder: tpOrderPubkey, slOrder: slOrderPubkey } = await limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: orderInputAmount,
                outputAmount: orderOutputAmount,
                orderType: OrderType.LimitParent,
                tpOutputAmount: tpOutputAmount,
                slOutputAmount: slOutputAmount,
            });

            order = orderPubkey;
            tpOrder = tpOrderPubkey;
            slOrder = slOrderPubkey;

            await limoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await limoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });
        });

        it("Cancel Type B with parent+child balances", async () => {
            const makerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                maker.publicKey
            );
            const makerOutputAta = spl.getAssociatedTokenAddressSync(
                outputMint,
                maker.publicKey
            );
            const { vault: inputVaultAta } = await limoHelper.getVault(inputMint);
            const { vault: outputVaultAta } = await limoHelper.getVault(outputMint);

            const fillInputAmount = orderInputAmount.div(new BN(2));
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, order);

            await limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const { signature } = await limoHelper.closeOrder({
                maker: makerWallet,
                order: order,
            });

            const makerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerInputAta);
            const makerOutputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const inputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const orderAccountInfo = await provider.connection.getAccountInfo(order);
            const tpOrderAccountInfo = await provider.connection.getAccountInfo(tpOrder);
            const slOrderAccountInfo = await provider.connection.getAccountInfo(slOrder);

            const tx = await provider.connection.getParsedTransaction(signature, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(new BN(makerInputAtaBalanceBefore.value.amount).add(orderInputAmount.sub(fillInputAmount)).toString());
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(new BN(makerOutputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).toString());
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(new BN(inputVaultAtaBalanceBefore.value.amount).sub(orderInputAmount.sub(fillInputAmount)).toString());
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(new BN(outputVaultAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).toString());

            expect(orderAccountInfo).to.be.null;
            expect(tpOrderAccountInfo).to.be.null;
            expect(slOrderAccountInfo).to.be.null;
        });

        it("Cancel Type A before cooldown rejected", async () => {
            await limoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
                value: [10],
            });

            await expectRejects(limoHelper.closeOrder({
                maker: makerWallet,
                order: order,
            }), LimoError.NotEnoughTimePassedSinceLastUpdate);

            await limoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
                value: [0],
            });
        });

        /// After cancel, order account deleted, so we can't cancel it again.
        it.skip("Cancel already Cancelled/Closed rejected", async () => {});
    });
});