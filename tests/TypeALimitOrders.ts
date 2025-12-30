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

describe("Type A Limit Orders", () => {
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

        const { mint: inputMintPubkey } = await createMintWithInitialBalance({
            connection: provider.connection,
            payer: tokenCreator,
            recipient: maker.publicKey,
            decimals: 6,
            initialBalance: 1000000000000
        });
        const { mint: outputMintPubkey } = await createMintWithInitialBalance({
            connection: provider.connection,
            payer: tokenCreator,
            recipient: taker.publicKey,
            decimals: 6,
            initialBalance: 1000000000000
        });
        inputMint = inputMintPubkey;
        outputMint = outputMintPubkey;

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
  
    describe("Create Order", () => {
        it("Should create type A limit order", async () => {
            const inputAmount = new BN(100000000000);
            const outputAmount = new BN(100000000000);

            const { vault: inputVaultAta } = await limoHelper.getVault(inputMint);
            const makerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                maker.publicKey
            );

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const { signature, order } = await limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: inputAmount,
                outputAmount: outputAmount,
                orderType: OrderType.Vanilla,
            });

            const makerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerInputAta);
            const inputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const orderAccount = await limoHelper.getOrderAccount(order);
            const globalConfig = await limoHelper.getGlobalConfig();
            const inputTokenProgram = (await provider.connection.getAccountInfo(inputMint))?.owner;
            const outputTokenProgram = (await provider.connection.getAccountInfo(outputMint))?.owner;

            expect(orderAccount.globalConfig).to.deep.equal(globalConfig);
            expect(orderAccount.maker).to.deep.equal(maker.publicKey);
            expect(orderAccount.inputMint).to.deep.equal(inputMint);
            expect(orderAccount.inputMintProgramId).to.deep.equal(inputTokenProgram);
            expect(orderAccount.outputMint).to.deep.equal(outputMint);
            expect(orderAccount.outputMintProgramId).to.deep.equal(outputTokenProgram);
            expect(orderAccount.parentOrder).to.deep.equal(web3.PublicKey.default);
            expect(orderAccount.tpChildOrder).to.deep.equal(web3.PublicKey.default);
            expect(orderAccount.slChildOrder).to.deep.equal(web3.PublicKey.default);
            expect(orderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.initialInputAmount.toString()).to.equal(inputAmount.toString());
            expect(orderAccount.expectedOutputAmount.toString()).to.equal(outputAmount.toString());
            expect(orderAccount.remainingInputAmount.toString()).to.equal(inputAmount.toString());
            expect(orderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.tipAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.orderType).to.equal(OrderType.Vanilla);
            expect(orderAccount.status).to.equal(OrderStatus.Active);

            const expectedMakerInputAtaBalanceAfter = new BN(makerInputAtaBalanceBefore.value.amount).sub(inputAmount).toString();
            expect(makerInputAtaBalanceAfter.value.amount).to.equal(expectedMakerInputAtaBalanceAfter);
            const expectedInputVaultAtaBalanceAfter = new BN(inputVaultAtaBalanceBefore.value.amount).add(inputAmount).toString();
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(expectedInputVaultAtaBalanceAfter);
        });

        it("Should reject create order with invalid input amount", async () => {
            const inputAmount = new BN(0);
            const outputAmount = new BN(100000000000);

            await expectRejects(limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: inputAmount,
                outputAmount: outputAmount,
                orderType: OrderType.Vanilla,
            }), LimoError.OrderInputAmountInvalid);
        });

        it("Should reject create order with invalid output amount", async () => {
            const inputAmount = new BN(100000000000);
            const outputAmount = new BN(0);

            await expectRejects(limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: inputAmount,
                outputAmount: outputAmount,
                orderType: OrderType.Vanilla,
            }), LimoError.OrderOutputAmountInvalid);
        });

        it("Should reject create order when new orders are blocked", async () => {
            await limoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateBlockNewOrders,
                value: [1],
            });

            const inputAmount = new BN(100000000000);
            const outputAmount = new BN(100000000000);

            await expectRejects(limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: inputAmount,
                outputAmount: outputAmount,
                orderType: OrderType.Vanilla,
            }), LimoError.CreatingNewOrdersBlocked);

            await limoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateBlockNewOrders,
                value: [0],
            });
        });
    });

    describe("Fill Order", () => {
        let order: web3.PublicKey;
        const orderInputAmount = new BN(100000000000);
        const orderOutputAmount = new BN(200000000000);

        async function calcMinOutputAmount(inputAmount: BN, order: web3.PublicKey): Promise<BN> {
            const orderAccount = await limoHelper.getOrderAccount(order);
            const numerator = new BN(inputAmount).mul(orderAccount.expectedOutputAmount);
            const denominator = orderAccount.initialInputAmount;
            return numerator.add(denominator).sub(new BN(1)).div(denominator);
        }

        before(async () => {
            await spl.createAssociatedTokenAccountIdempotent(
                provider.connection,
                taker,
                inputMint,
                taker.publicKey
            );

            await spl.createAssociatedTokenAccountIdempotent(
                provider.connection,
                maker,
                outputMint,
                maker.publicKey
            );
        });

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

        it("Should partially fill type A limit order", async () => {
            const fillInputAmount = orderInputAmount.div(new BN(2));
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, order);

            const makerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                maker.publicKey
            );
            const makerOutputAta = spl.getAssociatedTokenAddressSync(
                outputMint,
                maker.publicKey
            );
            const takerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                taker.publicKey
            );
            const takerOutputAta = spl.getAssociatedTokenAddressSync(
                outputMint,
                taker.publicKey
            );
            const { vault: inputVaultAta } = await limoHelper.getVault(inputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const { signatures } = await limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const makerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const orderAccount = await limoHelper.getOrderAccount(order);
            const tx = await provider.connection.getParsedTransaction(signatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            expect(orderAccount.remainingInputAmount.toString()).to.equal(orderInputAmount.sub(fillInputAmount).toString());
            expect(orderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.toString());
            expect(orderAccount.expectedOutputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(orderAccount.initialInputAmount.toString()).to.equal(orderInputAmount.toString());
            expect(orderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            expect(orderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Active);

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(makerInputAtaBalanceBefore.value.amount);
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            const expectedMakerOutputAtaBalanceAfter = new BN(makerOutputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).toString();
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(expectedMakerOutputAtaBalanceAfter);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            const expectedInputVaultAtaBalanceAfter = new BN(inputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(expectedInputVaultAtaBalanceAfter);
        });

        it("Should reject overfill order", async () => {
            const fillInputAmount = orderInputAmount.add(new BN(1));
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, order);

            await expectRejects(limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            }), LimoError.OrderInputAmountTooLarge);
        });

        it("Should reject underfill order", async () => {
            const fillInputAmount = orderInputAmount;
            const fillMinOutputAmount = (await calcMinOutputAmount(fillInputAmount, order)).sub(new BN(1));

            await expectRejects(limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            }), LimoError.OrderOutputAmountInvalid);
        });

        it("Should fully fill type A limit order", async () => {
            const fillInputAmount = orderInputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, order);

            const makerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                maker.publicKey
            );
            const makerOutputAta = spl.getAssociatedTokenAddressSync(
                outputMint,
                maker.publicKey
            );
            const takerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                taker.publicKey
            );
            const takerOutputAta = spl.getAssociatedTokenAddressSync(
                outputMint,
                taker.publicKey
            );
            const { vault: inputVaultAta } = await limoHelper.getVault(inputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const { signatures } = await limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const makerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const orderAccount = await limoHelper.getOrderAccount(order);
            const tx = await provider.connection.getParsedTransaction(signatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            expect(orderAccount.remainingInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.filledOutputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(orderAccount.expectedOutputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(orderAccount.initialInputAmount.toString()).to.equal(orderInputAmount.toString());
            expect(orderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            expect(orderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(makerInputAtaBalanceBefore.value.amount);
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            const expectedMakerOutputAtaBalanceAfter = new BN(makerOutputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).toString();
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(expectedMakerOutputAtaBalanceAfter);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            const expectedInputVaultAtaBalanceAfter = new BN(inputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(expectedInputVaultAtaBalanceAfter);
        });
    });

    describe("Fill Order via DEX aggregator", () => {
        it.skip("DEX parent fill meets min-out", async () => {});

        it.skip("DEX insufficient output reverts", async () => {});

        it.skip("DEX route failure reverts", async () => {});
    });

    describe("Lifecycle and invariants", () => {
        let order: web3.PublicKey;
        const orderInputAmount = new BN(100000000000);
        const orderOutputAmount = new BN(200000000000);

        before(async () => {
            await spl.createAssociatedTokenAccountIdempotent(
                provider.connection,
                taker,
                inputMint,
                taker.publicKey
            );

            await spl.createAssociatedTokenAccountIdempotent(
                provider.connection,
                maker,
                outputMint,
                maker.publicKey
            );
        });

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

        // The order account is deleted when the order is closed, so it is not possible to fill a closed order
        it.skip("Should reject fill order when order is closed", async () => {});

        it("Should reject fill order when order is filled", async () => {
            await limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: orderOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            await expectRejects(limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: orderOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            }), LimoError.OrderNotActive);
        });
    });
});