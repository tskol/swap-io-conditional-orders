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

describe("Type B Parent Limit Orders", () => {
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
        it("Should create type B parent limit order with TP and SL", async () => {
            const inputAmount = new BN(200000000000);
            const outputAmount = new BN(100000000000);
            const tpOutputAmount = new BN(220000000000);
            const slOutputAmount = new BN(180000000000);

            const { vault: inputVaultAta } = await limoHelper.getVault(inputMint);
            const makerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                maker.publicKey
            );

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const { signature, order, tpOrder, slOrder } = await limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: inputAmount,
                outputAmount: outputAmount,
                orderType: OrderType.LimitParent,
                tpOutputAmount: tpOutputAmount,
                slOutputAmount: slOutputAmount,
            });

            const makerInputAtaBalanceAfter = await provider.connection.getTokenAccountBalance(makerInputAta);
            const inputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const orderAccount = await limoHelper.getOrderAccount(order);
            const tpOrderAccount = await limoHelper.getOrderAccount(tpOrder);
            const slOrderAccount = await limoHelper.getOrderAccount(slOrder);
            const globalConfig = limoHelper.getGlobalConfig();
            const inputTokenProgram = (await provider.connection.getAccountInfo(inputMint))?.owner;
            const outputTokenProgram = (await provider.connection.getAccountInfo(outputMint))?.owner;

            expect(orderAccount.globalConfig).to.deep.equal(globalConfig);
            expect(orderAccount.maker).to.deep.equal(maker.publicKey);
            expect(orderAccount.inputMint).to.deep.equal(inputMint);
            expect(orderAccount.inputMintProgramId).to.deep.equal(inputTokenProgram);
            expect(orderAccount.outputMint).to.deep.equal(outputMint);
            expect(orderAccount.outputMintProgramId).to.deep.equal(outputTokenProgram);
            expect(orderAccount.parentOrder).to.deep.equal(web3.PublicKey.default);
            expect(orderAccount.tpChildOrder).to.deep.equal(tpOrder);
            expect(orderAccount.slChildOrder).to.deep.equal(slOrder);
            expect(orderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.initialInputAmount.toString()).to.equal(inputAmount.toString());
            expect(orderAccount.expectedOutputAmount.toString()).to.equal(outputAmount.toString());
            expect(orderAccount.remainingInputAmount.toString()).to.equal(inputAmount.toString());
            expect(orderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.tipAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.orderType).to.equal(OrderType.LimitParent);
            expect(orderAccount.status).to.equal(OrderStatus.Active);

            expect(tpOrderAccount.globalConfig).to.deep.equal(globalConfig);
            expect(tpOrderAccount.maker).to.deep.equal(maker.publicKey);
            expect(tpOrderAccount.inputMint).to.deep.equal(outputMint);
            expect(tpOrderAccount.inputMintProgramId).to.deep.equal(outputTokenProgram);
            expect(tpOrderAccount.outputMint).to.deep.equal(inputMint);
            expect(tpOrderAccount.outputMintProgramId).to.deep.equal(inputTokenProgram);
            expect(tpOrderAccount.parentOrder).to.deep.equal(order);
            expect(tpOrderAccount.tpChildOrder).to.deep.equal(web3.PublicKey.default);
            expect(tpOrderAccount.slChildOrder).to.deep.equal(web3.PublicKey.default);
            expect(tpOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(tpOrderAccount.initialInputAmount.toString()).to.equal(outputAmount.toString());
            expect(tpOrderAccount.expectedOutputAmount.toString()).to.equal(tpOutputAmount.toString());
            expect(tpOrderAccount.remainingInputAmount.toString()).to.equal(outputAmount.toString());
            expect(tpOrderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(tpOrderAccount.tipAmount.toString()).to.equal(new BN(0).toString());
            expect(tpOrderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            expect(tpOrderAccount.orderType).to.equal(OrderType.LimitTP);
            expect(tpOrderAccount.status).to.equal(OrderStatus.Active);

            expect(slOrderAccount.globalConfig).to.deep.equal(globalConfig);
            expect(slOrderAccount.maker).to.deep.equal(maker.publicKey);
            expect(slOrderAccount.inputMint).to.deep.equal(outputMint);
            expect(slOrderAccount.inputMintProgramId).to.deep.equal(outputTokenProgram);
            expect(slOrderAccount.outputMint).to.deep.equal(inputMint);
            expect(slOrderAccount.outputMintProgramId).to.deep.equal(inputTokenProgram);
            expect(slOrderAccount.parentOrder).to.deep.equal(order);
            expect(slOrderAccount.tpChildOrder).to.deep.equal(web3.PublicKey.default);
            expect(slOrderAccount.slChildOrder).to.deep.equal(web3.PublicKey.default);
            expect(slOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(slOrderAccount.initialInputAmount.toString()).to.equal(outputAmount.toString());
            expect(slOrderAccount.expectedOutputAmount.toString()).to.equal(slOutputAmount.toString());
            expect(slOrderAccount.remainingInputAmount.toString()).to.equal(outputAmount.toString());
            expect(slOrderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(slOrderAccount.tipAmount.toString()).to.equal(new BN(0).toString());
            expect(slOrderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            expect(slOrderAccount.orderType).to.equal(OrderType.LimitSL);
            expect(slOrderAccount.status).to.equal(OrderStatus.Active);

            const expectedMakerInputAtaBalanceAfter = new BN(makerInputAtaBalanceBefore.value.amount).sub(inputAmount).toString();
            expect(makerInputAtaBalanceAfter.value.amount).to.equal(expectedMakerInputAtaBalanceAfter);
            const expectedInputVaultAtaBalanceAfter = new BN(inputVaultAtaBalanceBefore.value.amount).add(inputAmount).toString();
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(expectedInputVaultAtaBalanceAfter);
        });

        it("Should reject create order with zero tp and sl output amounts", async () => {
            const inputAmount = new BN(200000000000);
            const outputAmount = new BN(100000000000);
            const tpOutputAmount = new BN(0);
            const slOutputAmount = new BN(0);

            await expectRejects(limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: inputAmount,
                outputAmount: outputAmount,
                orderType: OrderType.LimitParent,
                tpOutputAmount: tpOutputAmount,
                slOutputAmount: slOutputAmount,
            }), LimoError.OrderParametersInvalid);
        });

        it("Should reject create order with tp less than sl output amounts", async () => {
            const inputAmount = new BN(200000000000);
            const outputAmount = new BN(100000000000);
            const tpOutputAmount = new BN(180000000000);
            const slOutputAmount = new BN(220000000000);

            await expectRejects(limoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: inputAmount,
                outputAmount: outputAmount,
                orderType: OrderType.LimitParent,
                tpOutputAmount: tpOutputAmount,
                slOutputAmount: slOutputAmount,
            }), LimoError.TPSLMinDistanceNotMet);
        });

        it.skip("Children start with zero inventory", async () => {});
    });

    describe("Fill Order", () => {
        let order: web3.PublicKey;
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
                orderType: OrderType.LimitParent,
                tpOutputAmount: tpOutputAmount,
                slOutputAmount: slOutputAmount,
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

        it("Should partially fill type B parent limit order", async () => {
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
            const { vault: outputVaultAta } = await limoHelper.getVault(outputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

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
            const outputVaultAtaBalanceAfter = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const orderAccount = await limoHelper.getOrderAccount(order);
            const tx = await provider.connection.getParsedTransaction(signatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            expect(orderAccount.remainingInputAmount.toString()).to.equal(orderInputAmount.sub(fillInputAmount).toString());
            expect(orderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.toString());
            expect(orderAccount.expectedOutputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(orderAccount.initialInputAmount.toString()).to.equal(orderInputAmount.toString());
            expect(orderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            expect(orderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Active);
            expect(orderAccount.availableChildInputAmount.toString()).to.equal(fillMinOutputAmount.toString());

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(makerInputAtaBalanceBefore.value.amount);
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(makerOutputAtaBalanceBefore.value.amount);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            const expectedInputVaultAtaBalanceAfter = new BN(inputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(expectedInputVaultAtaBalanceAfter);
            const expectedOutputVaultAtaBalanceAfter = new BN(outputVaultAtaBalanceBefore.value.amount).add(fillMinOutputAmount).toString();
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(expectedOutputVaultAtaBalanceAfter);
        });

        it.skip("DEX parent fill credits shared vault", async () => {});

        // Surplus token now sent to availableChildInputAmount. When parent and one of children are filled, the other child set to filled.
        // Surplus input and output tokens withdrawn on order close.
        it.skip("Surplus output bypasses child inventory", async () => {});
    });

    describe("Enforce parent/child inventory accounting", () => {
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

            await limoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await limoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });

            await limoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await limoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });
        });

        it("Child cannot consume more than available", async () => {
            await limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount.div(new BN(2)),
                minOutputAmount: await calcMinOutputAmount(orderInputAmount.div(new BN(2)), order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, tpOrder);

            await expectRejects(limoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            }), LimoError.OrderInputAmountTooLarge);
        });

        it("Child inventory matches parent fills minus child fills", async () => {
            await limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const orderAccountBefore = await limoHelper.getOrderAccount(order);

            expect(orderAccountBefore.status).to.equal(OrderStatus.Filled);
            expect(orderAccountBefore.availableChildInputAmount.toString()).to.equal(orderOutputAmount.toString());

            const fillInputAmount = orderOutputAmount.div(new BN(2));
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, tpOrder);

            await limoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const orderAccount = await limoHelper.getOrderAccount(order);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount).toString());
            
            await expectRejects(limoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
                inputAmount: fillInputAmount.add(new BN(1)),
                minOutputAmount: await calcMinOutputAmount(fillInputAmount.add(new BN(1)), tpOrder),
                tipAmountPermissionlessTaking: new BN(0),
            }), LimoError.OrderInputAmountTooLarge);
        });

        it("No double spend across TP and SL", async () => {
            await limoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const orderAccountBefore = await limoHelper.getOrderAccount(order);

            expect(orderAccountBefore.status).to.equal(OrderStatus.Filled);
            expect(orderAccountBefore.availableChildInputAmount.toString()).to.equal(orderOutputAmount.toString());

            const fillInputAmount = orderOutputAmount.div(new BN(2));
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, tpOrder);

            await limoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const orderAccount = await limoHelper.getOrderAccount(order);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount).toString());

            await expectRejects(limoHelper.takeOrder({
                taker: takerWallet,
                order: slOrder,
                inputAmount: fillInputAmount.add(new BN(1)),
                minOutputAmount: await calcMinOutputAmount(fillInputAmount.add(new BN(1)), slOrder),
                tipAmountPermissionlessTaking: new BN(0),
            }), LimoError.OrderInputAmountTooLarge);
        });
    });
});