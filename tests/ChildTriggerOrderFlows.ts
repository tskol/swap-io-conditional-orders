import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { web3, BN } from "@coral-xyz/anchor";
import { expect } from "chai";
import {
  OrdoHelper,
} from "./helpers/ordo";
import {
  airdrop,
  createMintWithInitialBalance,
  expectRejects,
  generateRandomOrdoAccounts,
} from "./helpers/utils";
import { OrderStatus, OrderType, OrdoError, UpdateGlobalConfigMode, UpdateOrderMode } from "./helpers/constants";
// import { OrdoHelper } from "./helpers/ordo.js";
// import { airdrop, generateRandomOrdoAccounts } from "./helpers/utils.js";

export const STABLE_PRICE_FEED =
  "0x8b1e8e689fbb95ece35155a8b42cb9f1b14208a2f6507866a9a90e8dc955289a";

describe("Type B Child Limit Orders", () => {
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

    const ordoHelper = new OrdoHelper(provider);

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
            initialBalance: 100000000000000
        });
        const { mint: outputMintPubkey, recipientTokenAccount: takerOutputAta } = await createMintWithInitialBalance({
            connection: provider.connection,
            payer: tokenCreator,
            recipient: taker.publicKey,
            decimals: 6,
            initialBalance: 100000000000000
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

        await ordoHelper.initializeGlobalConfig({
            payer: payerWallet,
        });

        await ordoHelper.initializeVault({
            payer: payerWallet,
            mint: inputMint,
        });
        await ordoHelper.initializeVault({
            payer: payerWallet,
            mint: outputMint,
        });

        await ordoHelper.initializeOraclePool({
            payer: payerWallet,
            mint: inputMint,
            feedId: STABLE_PRICE_FEED,
        });
        await ordoHelper.initializeOraclePool({
            payer: payerWallet,
            mint: outputMint,
            feedId: STABLE_PRICE_FEED,
        });
    });
  
    describe("Configure and (pre-fill) update TP/SL levels", () => {
        it("Configure TP/SL on create", async () => {
            const inputAmount = new BN(200000000000);
            const outputAmount = new BN(100000000000);
            const tpOutputAmount = new BN(220000000000);
            const slOutputAmount = new BN(180000000000);

            const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
            const makerInputAta = spl.getAssociatedTokenAddressSync(
                inputMint,
                maker.publicKey
            );

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);

            const { signature, order, tpOrder, slOrder } = await ordoHelper.createOrder({
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

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const tpOrderAccount = await ordoHelper.getOrderAccount(tpOrder);
            const slOrderAccount = await ordoHelper.getOrderAccount(slOrder);
            const globalConfig = ordoHelper.getGlobalConfig();
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
    });

    describe("Execute TP child fills with minimum TP price enforcement", () => {
        let order: web3.PublicKey;
        let tpOrder: web3.PublicKey;
        let slOrder: web3.PublicKey;
        const orderInputAmount = new BN(100000000000);
        const orderOutputAmount = new BN(200000000000);
        const tpOutputAmount = new BN(120000000000);
        const slOutputAmount = new BN(80000000000);
        const activeDurationSeconds = new BN(10);

        async function calcMinOutputAmount(inputAmount: BN, order: web3.PublicKey): Promise<BN> {
            const orderAccount = await ordoHelper.getOrderAccount(order);
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
            const { signature, order: orderPubkey, tpOrder: tpOrderPubkey, slOrder: slOrderPubkey } = await ordoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: orderInputAmount,
                outputAmount: orderOutputAmount,
                orderType: OrderType.LimitParent,
                tpOutputAmount: tpOutputAmount,
                slOutputAmount: slOutputAmount,
                activeDurationSeconds: activeDurationSeconds,
            });
            order = orderPubkey;
            tpOrder = tpOrderPubkey;
            slOrder = slOrderPubkey;

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });
        });

        it("TP fill at exact TP price", async () => {
            const { signatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, tpOrder);

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
            const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
            const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const { signatures: tpSignatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
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

            const tx = await provider.connection.getParsedTransaction(tpSignatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const tpOrderAccount = await ordoHelper.getOrderAccount(tpOrder);
            const slOrderAccount = await ordoHelper.getOrderAccount(slOrder);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(tpOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount).toString());
            expect(tpOrderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.toString());
            expect(tpOrderAccount.expectedOutputAmount.toString()).to.equal(tpOutputAmount.toString());
            expect(tpOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            // expect(tpOrderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(tpOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(tpOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(slOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(slOrderAccount.expectedOutputAmount.toString()).to.equal(slOutputAmount.toString());
            expect(slOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            // expect(slOrderAccount.lastUpdatedTimestamp.toString()).to.not.equal((tx?.blockTime ?? 0).toString());
            expect(slOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(slOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(new BN(makerInputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).toString());
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(makerOutputAtaBalanceBefore.value.amount);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(inputVaultAtaBalanceBefore.value.amount);
            const expectedOutputVaultAtaBalanceAfter = new BN(outputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(expectedOutputVaultAtaBalanceAfter);
        });

        it("TP overpay is allowed", async () => {
            const { signatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = (await calcMinOutputAmount(fillInputAmount, tpOrder)).add(new BN(100000));

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
            const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
            const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const { signatures: tpSignatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
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

            const tx = await provider.connection.getParsedTransaction(tpSignatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const tpOrderAccount = await ordoHelper.getOrderAccount(tpOrder);
            const slOrderAccount = await ordoHelper.getOrderAccount(slOrder);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(tpOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount).toString());
            expect(tpOrderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.toString());
            expect(tpOrderAccount.expectedOutputAmount.toString()).to.equal(tpOutputAmount.toString());
            expect(tpOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            // expect(tpOrderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(tpOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(tpOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(slOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(slOrderAccount.expectedOutputAmount.toString()).to.equal(slOutputAmount.toString());
            expect(slOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            // expect(slOrderAccount.lastUpdatedTimestamp.toString()).to.not.equal((tx?.blockTime ?? 0).toString());
            expect(slOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(slOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(new BN(makerInputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).toString());
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(makerOutputAtaBalanceBefore.value.amount);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(inputVaultAtaBalanceBefore.value.amount);
            const expectedOutputVaultAtaBalanceAfter = new BN(outputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(expectedOutputVaultAtaBalanceAfter);
        });

        it("TP underpay is rejected", async () => {
            const { signatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = (await calcMinOutputAmount(fillInputAmount, tpOrder)).sub(new BN(1));

            await expectRejects(
                ordoHelper.takeOrder({
                    taker: takerWallet,
                    order: tpOrder,
                    inputAmount: fillInputAmount,
                    minOutputAmount: fillMinOutputAmount,
                    tipAmountPermissionlessTaking: new BN(0),
                }),
                OrdoError.OrderOutputAmountInvalid,
            );
        });

        it("TP cannot exceed remaining child inventory", async () => {
            const { signatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount.div(new BN(2)),
                minOutputAmount: await calcMinOutputAmount(orderInputAmount.div(new BN(2)), order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = (await calcMinOutputAmount(orderInputAmount.div(new BN(2)), order)).add(new BN(1));
            const fillMinOutputAmount = (await calcMinOutputAmount(fillInputAmount, tpOrder));

            await expectRejects(
                ordoHelper.takeOrder({
                    taker: takerWallet,
                    order: tpOrder,
                    inputAmount: fillInputAmount,
                    minOutputAmount: fillMinOutputAmount,
                    tipAmountPermissionlessTaking: new BN(0),
                }),
                OrdoError.OrderInputAmountTooLarge,
            );
        });

        it("Should reject fill order when order is expired", async () => {
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateCounterparty,
                value: Array.from(taker.publicKey.toBuffer()),
            });

            const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
            await new Promise(resolve => setTimeout(resolve, waitMs));

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, tpOrder);

            await expectRejects(
                ordoHelper.takeOrder({
                    taker: takerWallet,
                    order: tpOrder,
                    inputAmount: fillInputAmount,
                    minOutputAmount: fillMinOutputAmount,
                    tipAmountPermissionlessTaking: new BN(0),
                }),
                OrdoError.OrderExpired,
            );
        });

        it("Should get keeper fee", async () => {
            const keeperFeeBps = new BN(1000);
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateKeeperTakeFeeBps,
                value: Array.from(keeperFeeBps.toArray("le", 2)),
            });

            const { signatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, tpOrder);

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
            const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
            const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const { signatures: tpSignatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
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

            const tx = await provider.connection.getParsedTransaction(tpSignatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            const expectedKeeperFee = fillMinOutputAmount.mul(keeperFeeBps).div(new BN(10000));

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const tpOrderAccount = await ordoHelper.getOrderAccount(tpOrder);
            const slOrderAccount = await ordoHelper.getOrderAccount(slOrder);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(tpOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount).toString());
            expect(tpOrderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.toString());
            expect(tpOrderAccount.expectedOutputAmount.toString()).to.equal(tpOutputAmount.toString());
            expect(tpOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            // expect(tpOrderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(tpOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(tpOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(slOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(slOrderAccount.expectedOutputAmount.toString()).to.equal(slOutputAmount.toString());
            expect(slOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            // expect(slOrderAccount.lastUpdatedTimestamp.toString()).to.not.equal((tx?.blockTime ?? 0).toString());
            expect(slOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(slOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(new BN(makerInputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).sub(expectedKeeperFee).toString());
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).add(expectedKeeperFee).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(makerOutputAtaBalanceBefore.value.amount);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(inputVaultAtaBalanceBefore.value.amount);
            const expectedOutputVaultAtaBalanceAfter = new BN(outputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(expectedOutputVaultAtaBalanceAfter);

            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateKeeperTakeFeeBps,
                value: Array.from(new BN(0).toArray("le", 2)),
            });
        });
    });

    describe("Execute SL child fills with oracle validation", () => {
        let order: web3.PublicKey;
        let tpOrder: web3.PublicKey;
        let slOrder: web3.PublicKey;
        const orderInputAmount = new BN(100000000000);
        const orderOutputAmount = new BN(100000000000);
        const tpOutputAmount = new BN(120000000000);
        const slOutputAmount = new BN(90000000000);
        const activeDurationSeconds = new BN(10);

        async function calcMinOutputAmount(inputAmount: BN, order: web3.PublicKey): Promise<BN> {
            const orderAccount = await ordoHelper.getOrderAccount(order);
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
            const { signature, order: orderPubkey, tpOrder: tpOrderPubkey, slOrder: slOrderPubkey } = await ordoHelper.createOrder({
                maker: makerWallet,
                inputMint: inputMint,
                outputMint: outputMint,
                inputAmount: orderInputAmount,
                outputAmount: orderOutputAmount,
                orderType: OrderType.LimitParent,
                tpOutputAmount: tpOutputAmount,
                slOutputAmount: slOutputAmount,
                activeDurationSeconds: activeDurationSeconds,
            });
            order = orderPubkey;
            tpOrder = tpOrderPubkey;
            slOrder = slOrderPubkey;

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });
        });

        it("SL executes inside SL band", async () => {
            await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, slOrder);

            // It is impossible to change the price in Oracle,
            // so for the test we will change SlMaxUpwardDeviationBps
            // to allow the SL order to execute.
            // value = 1000 means 10% deviation
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateSlMaxUpwardDeviationBps,
                value: [232, 3],
            });

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
            const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
            const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const { signatures: tpSignatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: slOrder,
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

            const tx = await provider.connection.getParsedTransaction(tpSignatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const tpOrderAccount = await ordoHelper.getOrderAccount(tpOrder);
            const slOrderAccount = await ordoHelper.getOrderAccount(slOrder);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(slOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount).toString());
            expect(slOrderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.toString());
            expect(slOrderAccount.expectedOutputAmount.toString()).to.equal(slOutputAmount.toString());
            expect(slOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            // expect(slOrderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(slOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(slOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(tpOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(tpOrderAccount.expectedOutputAmount.toString()).to.equal(tpOutputAmount.toString());
            expect(tpOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            // expect(tpOrderAccount.lastUpdatedTimestamp.toString()).to.not.equal((tx?.blockTime ?? 0).toString());
            expect(tpOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(tpOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(new BN(makerInputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).toString());
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(makerOutputAtaBalanceBefore.value.amount);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(inputVaultAtaBalanceBefore.value.amount);
            const expectedOutputVaultAtaBalanceAfter = new BN(outputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(expectedOutputVaultAtaBalanceAfter);
        });

        it("SL rejected if oracle too high", async () => {
            await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, slOrder);

            // It is impossible to change the price in Oracle,
            // so for the test we will change SlMaxUpwardDeviationBps
            // to allow the SL order to execute.
            // value = 999 means 9.99% deviation
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateSlMaxUpwardDeviationBps,
                value: [231, 3],
            });

            await expectRejects(
                ordoHelper.takeOrder({
                    taker: takerWallet,
                    order: slOrder,
                    inputAmount: fillInputAmount,
                    minOutputAmount: fillMinOutputAmount,
                    tipAmountPermissionlessTaking: new BN(0),
                }),
                OrdoError.PriceTooHigh,
            );
        });

        it("SL underpay rejected", async () => {
            await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = (await calcMinOutputAmount(fillInputAmount, slOrder)).sub(new BN(1));

            // It is impossible to change the price in Oracle,
            // so for the test we will change SlMaxUpwardDeviationBps
            // to allow the SL order to execute.
            // value = 1000 means 10% deviation
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateSlMaxUpwardDeviationBps,
                value: [232, 3],
            });

            await expectRejects(
                ordoHelper.takeOrder({
                    taker: takerWallet,
                    order: slOrder,
                    inputAmount: fillInputAmount,
                    minOutputAmount: fillMinOutputAmount,
                    tipAmountPermissionlessTaking: new BN(0),
                }),
                OrdoError.OrderOutputAmountInvalid,
            );
        });

        // We cannot test this because we cannot change the data in Oracle.
        it("SL with stale oracle rejected", async () => {});

        it("Should reject fill order when order is expired", async () => {
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateCounterparty,
                value: Array.from(taker.publicKey.toBuffer()),
            });

            const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
            await new Promise(resolve => setTimeout(resolve, waitMs));

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, slOrder);

            await expectRejects(
                ordoHelper.takeOrder({
                    taker: takerWallet,
                    order: slOrder,
                    inputAmount: fillInputAmount,
                    minOutputAmount: fillMinOutputAmount,
                    tipAmountPermissionlessTaking: new BN(0),
                }),
                OrdoError.OrderExpired,
            );
        });

        it("Should get keeper fee", async () => {
            const keeperFeeBps = new BN(1000);
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateKeeperTakeFeeBps,
                value: Array.from(keeperFeeBps.toArray("le", 2)),
            });

            await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount;
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, slOrder);

            // It is impossible to change the price in Oracle,
            // so for the test we will change SlMaxUpwardDeviationBps
            // to allow the SL order to execute.
            // value = 1000 means 10% deviation
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateSlMaxUpwardDeviationBps,
                value: [232, 3],
            });

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
            const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
            const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

            const makerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerInputAta);
            const takerInputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerInputAta);
            const makerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(makerOutputAta);
            const takerOutputAtaBalanceBefore = await provider.connection.getTokenAccountBalance(takerOutputAta);
            const inputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(inputVaultAta);
            const outputVaultAtaBalanceBefore = await provider.connection.getTokenAccountBalance(outputVaultAta);

            const { signatures: tpSignatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: slOrder,
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

            const tx = await provider.connection.getParsedTransaction(tpSignatures[0], { commitment: "confirmed", maxSupportedTransactionVersion: 0 });

            const expectedKeeperFee = fillMinOutputAmount.mul(keeperFeeBps).div(new BN(10000));

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const tpOrderAccount = await ordoHelper.getOrderAccount(tpOrder);
            const slOrderAccount = await ordoHelper.getOrderAccount(slOrder);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(slOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount).toString());
            expect(slOrderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.toString());
            expect(slOrderAccount.expectedOutputAmount.toString()).to.equal(slOutputAmount.toString());
            expect(slOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.numberOfFills.toString()).to.equal(new BN(1).toString());
            // expect(slOrderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
            expect(slOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(slOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(tpOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.filledOutputAmount.toString()).to.equal(new BN(0).toString());
            expect(tpOrderAccount.expectedOutputAmount.toString()).to.equal(tpOutputAmount.toString());
            expect(tpOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.numberOfFills.toString()).to.equal(new BN(0).toString());
            // expect(tpOrderAccount.lastUpdatedTimestamp.toString()).to.not.equal((tx?.blockTime ?? 0).toString());
            expect(tpOrderAccount.status).to.equal(OrderStatus.Filled);
            expect(tpOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());

            expect(makerInputAtaBalanceAfter.value.amount).to.equal(new BN(makerInputAtaBalanceBefore.value.amount).add(fillMinOutputAmount).sub(expectedKeeperFee).toString());
            const expectedTakerInputAtaBalanceAfter = new BN(takerInputAtaBalanceBefore.value.amount).sub(fillMinOutputAmount).add(expectedKeeperFee).toString();
            expect(takerInputAtaBalanceAfter.value.amount).to.equal(expectedTakerInputAtaBalanceAfter);
            expect(makerOutputAtaBalanceAfter.value.amount).to.equal(makerOutputAtaBalanceBefore.value.amount);
            const expectedTakerOutputAtaBalanceAfter = new BN(takerOutputAtaBalanceBefore.value.amount).add(fillInputAmount).toString();
            expect(takerOutputAtaBalanceAfter.value.amount).to.equal(expectedTakerOutputAtaBalanceAfter);
            expect(inputVaultAtaBalanceAfter.value.amount).to.equal(inputVaultAtaBalanceBefore.value.amount);
            const expectedOutputVaultAtaBalanceAfter = new BN(outputVaultAtaBalanceBefore.value.amount).sub(fillInputAmount).toString();
            expect(outputVaultAtaBalanceAfter.value.amount).to.equal(expectedOutputVaultAtaBalanceAfter);

            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateKeeperTakeFeeBps,
                value: Array.from(new BN(0).toArray("le", 2)),
            });
        });
    });

    describe("Aggregate realized input from TP/SL", () => {
        let order: web3.PublicKey;
        let tpOrder: web3.PublicKey;
        let slOrder: web3.PublicKey;
        const orderInputAmount = new BN(100000000000);
        const orderOutputAmount = new BN(200000000000);
        const tpOutputAmount = new BN(120000000000);
        const slOutputAmount = new BN(90000000000);

        async function calcMinOutputAmount(inputAmount: BN, order: web3.PublicKey): Promise<BN> {
            const orderAccount = await ordoHelper.getOrderAccount(order);
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
            const { signature, order: orderPubkey, tpOrder: tpOrderPubkey, slOrder: slOrderPubkey } = await ordoHelper.createOrder({
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

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: order,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: tpOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });

            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdatePermissionless,
                value: new BN(1).toBuffer(),
            });
            await ordoHelper.updateOrder({
                maker: makerWallet,
                order: slOrder,
                mode: UpdateOrderMode.UpdateCounterparty,
                value: taker.publicKey.toBuffer(),
            });
        });

        it("TP realized input tracked correctly", async () => {
            const { signatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            const fillInputAmount = orderOutputAmount.div(new BN(10));
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, tpOrder);

            const { signatures: tpSignatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const { signatures: tpSignatures2 } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const { signatures: tpSignatures3 } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: tpOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const tpOrderAccount = await ordoHelper.getOrderAccount(tpOrder);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount.mul(new BN(3))).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(tpOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount.mul(new BN(3))).toString());
            expect(tpOrderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.mul(new BN(3)).toString());
            expect(tpOrderAccount.expectedOutputAmount.toString()).to.equal(tpOutputAmount.toString());
            expect(tpOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(tpOrderAccount.numberOfFills.toString()).to.equal(new BN(3).toString());
            expect(tpOrderAccount.status).to.equal(OrderStatus.Active);
            expect(tpOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
        });

        it.skip("SL realized input tracked correctly", async () => {
            await ordoHelper.takeOrder({
                taker: takerWallet,
                order: order,
                inputAmount: orderInputAmount,
                minOutputAmount: await calcMinOutputAmount(orderInputAmount, order),
                tipAmountPermissionlessTaking: new BN(0),
            });

            // It is impossible to change the price in Oracle,
            // so for the test we will change SlMaxUpwardDeviationBps
            // to allow the SL order to execute.
            // value = 1000 means 10% deviation
            await ordoHelper.updateGlobalConfig({
                payer: payerWallet,
                mode: UpdateGlobalConfigMode.UpdateSlMaxUpwardDeviationBps,
                value: [232, 3],
            });

            const fillInputAmount = orderOutputAmount.div(new BN(10));
            const fillMinOutputAmount = await calcMinOutputAmount(fillInputAmount, slOrder);

            const { signatures: tpSignatures } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: slOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const { signatures: tpSignatures2 } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: slOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const { signatures: tpSignatures3 } = await ordoHelper.takeOrder({
                taker: takerWallet,
                order: slOrder,
                inputAmount: fillInputAmount,
                minOutputAmount: fillMinOutputAmount,
                tipAmountPermissionlessTaking: new BN(0),
            });

            const orderAccount = await ordoHelper.getOrderAccount(order);
            const slOrderAccount = await ordoHelper.getOrderAccount(slOrder);

            expect(orderAccount.availableChildInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount.mul(new BN(3))).toString());
            expect(orderAccount.status).to.equal(OrderStatus.Filled);

            expect(slOrderAccount.remainingInputAmount.toString()).to.equal(orderOutputAmount.sub(fillInputAmount.mul(new BN(3))).toString());
            expect(slOrderAccount.filledOutputAmount.toString()).to.equal(fillMinOutputAmount.mul(new BN(3)).toString());
            expect(slOrderAccount.expectedOutputAmount.toString()).to.equal(slOutputAmount.toString());
            expect(slOrderAccount.initialInputAmount.toString()).to.equal(orderOutputAmount.toString());
            expect(slOrderAccount.numberOfFills.toString()).to.equal(new BN(3).toString());
            expect(slOrderAccount.status).to.equal(OrderStatus.Active);
            expect(slOrderAccount.availableChildInputAmount.toString()).to.equal(new BN(0).toString());
        });

        // Cancel deletes the order account, so we cannot test this.
        it("Cancel does not erase realized input", async () => {});
    });
});