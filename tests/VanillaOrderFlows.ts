import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { web3, BN } from "@coral-xyz/anchor";
import { expect } from "chai";
import { OrdoHelper } from "./support/ordo-client";
import { expectRejects } from "./support/token-fixtures";
import {
  bootstrapOrderFlow,
  createFlowActors,
  createFlowProvider,
  ensureLocalValidator,
} from "./support/flow-environment";
import {
  OrderStatus,
  OrderType,
  OrdoError,
  UpdateGlobalConfigMode,
  UpdateOrderMode,
} from "./support/ordo-constants";
import { ensureChildFlowTokenAccounts } from "./support/ordo/child-flow";
import { createExecutableVanillaFillOrder } from "./support/ordo/fill-flow";
import { waitForOrderExpiry } from "./support/ordo/close-flow";

describe("Type A Limit Orders", () => {
  const { connection, provider } = createFlowProvider();
  const {
    payer,
    payerWallet,
    maker,
    makerWallet,
    taker,
    takerWallet,
    tokenCreator,
    tokenCreatorWallet,
  } = createFlowActors();

  const ordoHelper = new OrdoHelper(provider);

  let inputMint: web3.PublicKey;
  let outputMint: web3.PublicKey;

  before(async function () {
    await ensureLocalValidator(connection);
    const flow = await bootstrapOrderFlow({
      provider,
      ordoHelper,
      payer,
      payerWallet,
      maker,
      taker,
      tokenCreator,
      makerInputBalance: 100000000000000,
      takerOutputBalance: 100000000000000,
    });
    inputMint = flow.inputMint;
    outputMint = flow.outputMint;
  });

  describe("Create Order", () => {
    it("Should create type A limit order", async () => {
      const inputAmount = new BN(100000000000);
      const outputAmount = new BN(100000000000);

      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const { signature, order } = await ordoHelper.createOrder({
        maker: makerWallet,
        inputMint: inputMint,
        outputMint: outputMint,
        inputAmount: inputAmount,
        outputAmount: outputAmount,
        orderType: OrderType.Vanilla,
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const orderAccount = await ordoHelper.getOrderAccount(order);
      const globalConfig = await ordoHelper.getGlobalConfig();
      const inputTokenProgram = (
        await provider.connection.getAccountInfo(inputMint)
      )?.owner;
      const outputTokenProgram = (
        await provider.connection.getAccountInfo(outputMint)
      )?.owner;

      expect(orderAccount.globalConfig).to.deep.equal(globalConfig);
      expect(orderAccount.maker).to.deep.equal(maker.publicKey);
      expect(orderAccount.inputMint).to.deep.equal(inputMint);
      expect(orderAccount.inputMintProgramId).to.deep.equal(inputTokenProgram);
      expect(orderAccount.outputMint).to.deep.equal(outputMint);
      expect(orderAccount.outputMintProgramId).to.deep.equal(
        outputTokenProgram,
      );
      expect(orderAccount.parentOrder).to.deep.equal(web3.PublicKey.default);
      expect(orderAccount.tpChildOrder).to.deep.equal(web3.PublicKey.default);
      expect(orderAccount.slChildOrder).to.deep.equal(web3.PublicKey.default);
      expect(orderAccount.availableChildInputAmount.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(orderAccount.initialInputAmount.toString()).to.equal(
        inputAmount.toString(),
      );
      expect(orderAccount.expectedOutputAmount.toString()).to.equal(
        outputAmount.toString(),
      );
      expect(orderAccount.remainingInputAmount.toString()).to.equal(
        inputAmount.toString(),
      );
      expect(orderAccount.filledOutputAmount.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(orderAccount.tipAmount.toString()).to.equal(new BN(0).toString());
      expect(orderAccount.numberOfFills.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(orderAccount.orderType).to.equal(OrderType.Vanilla);
      expect(orderAccount.status).to.equal(OrderStatus.Active);

      const expectedMakerInputAtaBalanceAfter = new BN(
        makerInputAtaBalanceBefore.value.amount,
      )
        .sub(inputAmount)
        .toString();
      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        expectedMakerInputAtaBalanceAfter,
      );
      const expectedInputVaultAtaBalanceAfter = new BN(
        inputVaultAtaBalanceBefore.value.amount,
      )
        .add(inputAmount)
        .toString();
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        expectedInputVaultAtaBalanceAfter,
      );
    });

    it("Should reject create order with invalid input amount", async () => {
      const inputAmount = new BN(0);
      const outputAmount = new BN(100000000000);

      await expectRejects(
        ordoHelper.createOrder({
          maker: makerWallet,
          inputMint: inputMint,
          outputMint: outputMint,
          inputAmount: inputAmount,
          outputAmount: outputAmount,
          orderType: OrderType.Vanilla,
        }),
        OrdoError.OrderInputAmountInvalid,
      );
    });

    it("Should reject create order with invalid output amount", async () => {
      const inputAmount = new BN(100000000000);
      const outputAmount = new BN(0);

      await expectRejects(
        ordoHelper.createOrder({
          maker: makerWallet,
          inputMint: inputMint,
          outputMint: outputMint,
          inputAmount: inputAmount,
          outputAmount: outputAmount,
          orderType: OrderType.Vanilla,
        }),
        OrdoError.OrderOutputAmountInvalid,
      );
    });

    it("Should reject create order when new orders are blocked", async () => {
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateBlockNewOrders,
        value: [1],
      });

      const inputAmount = new BN(100000000000);
      const outputAmount = new BN(100000000000);

      await expectRejects(
        ordoHelper.createOrder({
          maker: makerWallet,
          inputMint: inputMint,
          outputMint: outputMint,
          inputAmount: inputAmount,
          outputAmount: outputAmount,
          orderType: OrderType.Vanilla,
        }),
        OrdoError.CreatingNewOrdersBlocked,
      );

      await ordoHelper.updateGlobalConfig({
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
    const activeDurationSeconds = new BN(10);

    async function calcMinOutputAmount(
      inputAmount: BN,
      order: web3.PublicKey,
    ): Promise<BN> {
      const orderAccount = await ordoHelper.getOrderAccount(order);
      const numerator = new BN(inputAmount).mul(
        orderAccount.expectedOutputAmount,
      );
      const denominator = orderAccount.initialInputAmount;
      return numerator.add(denominator).sub(new BN(1)).div(denominator);
    }

    before(async () => {
      await ensureChildFlowTokenAccounts({
        connection: provider.connection,
        taker,
        maker,
        inputMint,
        outputMint,
      });
    });

    beforeEach(async () => {
      order = await createExecutableVanillaFillOrder({
        ordoHelper,
        maker: makerWallet,
        taker,
        inputMint,
        outputMint,
        inputAmount: orderInputAmount,
        outputAmount: orderOutputAmount,
        activeDurationSeconds,
      });
    });

    it("Should partially fill type A limit order", async () => {
      const fillInputAmount = orderInputAmount.div(new BN(2));
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const takerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const takerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const { signatures } = await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const orderAccount = await ordoHelper.getOrderAccount(order);
      const tx = await provider.connection.getParsedTransaction(signatures[0], {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(orderAccount.remainingInputAmount.toString()).to.equal(
        orderInputAmount.sub(fillInputAmount).toString(),
      );
      expect(orderAccount.filledOutputAmount.toString()).to.equal(
        fillMinOutputAmount.toString(),
      );
      expect(orderAccount.expectedOutputAmount.toString()).to.equal(
        orderOutputAmount.toString(),
      );
      expect(orderAccount.initialInputAmount.toString()).to.equal(
        orderInputAmount.toString(),
      );
      expect(orderAccount.numberOfFills.toString()).to.equal(
        new BN(1).toString(),
      );
      // expect(orderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
      expect(orderAccount.status).to.equal(OrderStatus.Active);

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        makerInputAtaBalanceBefore.value.amount,
      );
      const expectedTakerInputAtaBalanceAfter = new BN(
        takerInputAtaBalanceBefore.value.amount,
      )
        .add(fillInputAmount)
        .toString();
      expect(takerInputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerInputAtaBalanceAfter,
      );
      const expectedMakerOutputAtaBalanceAfter = new BN(
        makerOutputAtaBalanceBefore.value.amount,
      )
        .add(fillMinOutputAmount)
        .toString();
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedMakerOutputAtaBalanceAfter,
      );
      const expectedTakerOutputAtaBalanceAfter = new BN(
        takerOutputAtaBalanceBefore.value.amount,
      )
        .sub(fillMinOutputAmount)
        .toString();
      expect(takerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerOutputAtaBalanceAfter,
      );
      const expectedInputVaultAtaBalanceAfter = new BN(
        inputVaultAtaBalanceBefore.value.amount,
      )
        .sub(fillInputAmount)
        .toString();
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        expectedInputVaultAtaBalanceAfter,
      );
    });

    it("Should reject overfill order", async () => {
      const fillInputAmount = orderInputAmount.add(new BN(1));
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      await expectRejects(
        ordoHelper.takeOrder({
          taker: takerWallet,
          order: order,
          inputAmount: fillInputAmount,
          minOutputAmount: fillMinOutputAmount,
          tipAmountPermissionlessTaking: new BN(0),
        }),
        OrdoError.OrderInputAmountTooLarge,
      );
    });

    it("Should reject underfill order", async () => {
      const fillInputAmount = orderInputAmount;
      const fillMinOutputAmount = (
        await calcMinOutputAmount(fillInputAmount, order)
      ).sub(new BN(1));

      await expectRejects(
        ordoHelper.takeOrder({
          taker: takerWallet,
          order: order,
          inputAmount: fillInputAmount,
          minOutputAmount: fillMinOutputAmount,
          tipAmountPermissionlessTaking: new BN(0),
        }),
        OrdoError.OrderOutputAmountInvalid,
      );
    });

    it("Should fully fill type A limit order", async () => {
      const fillInputAmount = orderInputAmount;
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const takerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const takerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const { signatures } = await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const orderAccount = await ordoHelper.getOrderAccount(order);
      const tx = await provider.connection.getParsedTransaction(signatures[0], {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(orderAccount.remainingInputAmount.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(orderAccount.filledOutputAmount.toString()).to.equal(
        orderOutputAmount.toString(),
      );
      expect(orderAccount.expectedOutputAmount.toString()).to.equal(
        orderOutputAmount.toString(),
      );
      expect(orderAccount.initialInputAmount.toString()).to.equal(
        orderInputAmount.toString(),
      );
      expect(orderAccount.numberOfFills.toString()).to.equal(
        new BN(1).toString(),
      );
      // expect(orderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
      expect(orderAccount.status).to.equal(OrderStatus.Filled);

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        makerInputAtaBalanceBefore.value.amount,
      );
      const expectedTakerInputAtaBalanceAfter = new BN(
        takerInputAtaBalanceBefore.value.amount,
      )
        .add(fillInputAmount)
        .toString();
      expect(takerInputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerInputAtaBalanceAfter,
      );
      const expectedMakerOutputAtaBalanceAfter = new BN(
        makerOutputAtaBalanceBefore.value.amount,
      )
        .add(fillMinOutputAmount)
        .toString();
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedMakerOutputAtaBalanceAfter,
      );
      const expectedTakerOutputAtaBalanceAfter = new BN(
        takerOutputAtaBalanceBefore.value.amount,
      )
        .sub(fillMinOutputAmount)
        .toString();
      expect(takerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerOutputAtaBalanceAfter,
      );
      const expectedInputVaultAtaBalanceAfter = new BN(
        inputVaultAtaBalanceBefore.value.amount,
      )
        .sub(fillInputAmount)
        .toString();
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        expectedInputVaultAtaBalanceAfter,
      );
    });

    it("Should fully fill type A limit order without counterparty", async () => {
      await ordoHelper.updateOrder({
        maker: makerWallet,
        order: order,
        mode: UpdateOrderMode.UpdatePermissionless,
        value: new BN(0).toBuffer(),
      });
      await ordoHelper.updateOrder({
        maker: makerWallet,
        order: order,
        mode: UpdateOrderMode.UpdateCounterparty,
        value: web3.PublicKey.default.toBuffer(),
      });
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateCounterparty,
        value: Array.from(taker.publicKey.toBuffer()),
      });

      const fillInputAmount = orderInputAmount;
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const takerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const takerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const { signatures } = await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const orderAccount = await ordoHelper.getOrderAccount(order);
      const tx = await provider.connection.getParsedTransaction(signatures[0], {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(orderAccount.remainingInputAmount.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(orderAccount.filledOutputAmount.toString()).to.equal(
        orderOutputAmount.toString(),
      );
      expect(orderAccount.expectedOutputAmount.toString()).to.equal(
        orderOutputAmount.toString(),
      );
      expect(orderAccount.initialInputAmount.toString()).to.equal(
        orderInputAmount.toString(),
      );
      expect(orderAccount.numberOfFills.toString()).to.equal(
        new BN(1).toString(),
      );
      // expect(orderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
      expect(orderAccount.status).to.equal(OrderStatus.Filled);

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        makerInputAtaBalanceBefore.value.amount,
      );
      const expectedTakerInputAtaBalanceAfter = new BN(
        takerInputAtaBalanceBefore.value.amount,
      )
        .add(fillInputAmount)
        .toString();
      expect(takerInputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerInputAtaBalanceAfter,
      );
      const expectedMakerOutputAtaBalanceAfter = new BN(
        makerOutputAtaBalanceBefore.value.amount,
      )
        .add(fillMinOutputAmount)
        .toString();
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedMakerOutputAtaBalanceAfter,
      );
      const expectedTakerOutputAtaBalanceAfter = new BN(
        takerOutputAtaBalanceBefore.value.amount,
      )
        .sub(fillMinOutputAmount)
        .toString();
      expect(takerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerOutputAtaBalanceAfter,
      );
      const expectedInputVaultAtaBalanceAfter = new BN(
        inputVaultAtaBalanceBefore.value.amount,
      )
        .sub(fillInputAmount)
        .toString();
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        expectedInputVaultAtaBalanceAfter,
      );
    });

    it("Should reject fill order when order is expired", async () => {
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateCounterparty,
        value: Array.from(taker.publicKey.toBuffer()),
      });

      await waitForOrderExpiry(activeDurationSeconds);

      await expectRejects(
        ordoHelper.takeOrder({
          taker: takerWallet,
          order: order,
          inputAmount: orderInputAmount,
          minOutputAmount: orderOutputAmount,
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

      const fillInputAmount = orderInputAmount;
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const takerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const takerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const { signatures } = await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const takerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const takerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(takerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);

      const orderAccount = await ordoHelper.getOrderAccount(order);
      const tx = await provider.connection.getParsedTransaction(signatures[0], {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      const expectedKeeperFee = orderOutputAmount
        .mul(keeperFeeBps)
        .div(new BN(10000));

      expect(orderAccount.remainingInputAmount.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(orderAccount.filledOutputAmount.toString()).to.equal(
        orderOutputAmount.toString(),
      );
      expect(orderAccount.expectedOutputAmount.toString()).to.equal(
        orderOutputAmount.toString(),
      );
      expect(orderAccount.initialInputAmount.toString()).to.equal(
        orderInputAmount.toString(),
      );
      expect(orderAccount.numberOfFills.toString()).to.equal(
        new BN(1).toString(),
      );
      // expect(orderAccount.lastUpdatedTimestamp.toString()).to.equal((tx?.blockTime ?? 0).toString());
      expect(orderAccount.status).to.equal(OrderStatus.Filled);

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        makerInputAtaBalanceBefore.value.amount,
      );
      const expectedTakerInputAtaBalanceAfter = new BN(
        takerInputAtaBalanceBefore.value.amount,
      )
        .add(fillInputAmount)
        .toString();
      expect(takerInputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerInputAtaBalanceAfter,
      );
      const expectedMakerOutputAtaBalanceAfter = new BN(
        makerOutputAtaBalanceBefore.value.amount,
      )
        .add(fillMinOutputAmount)
        .sub(expectedKeeperFee)
        .toString();
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedMakerOutputAtaBalanceAfter,
      );
      const expectedTakerOutputAtaBalanceAfter = new BN(
        takerOutputAtaBalanceBefore.value.amount,
      )
        .sub(fillMinOutputAmount)
        .add(expectedKeeperFee)
        .toString();
      expect(takerOutputAtaBalanceAfter.value.amount).to.equal(
        expectedTakerOutputAtaBalanceAfter,
      );
      const expectedInputVaultAtaBalanceAfter = new BN(
        inputVaultAtaBalanceBefore.value.amount,
      )
        .sub(fillInputAmount)
        .toString();
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        expectedInputVaultAtaBalanceAfter,
      );

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperTakeFeeBps,
        value: Array.from(new BN(0).toArray("le", 2)),
      });
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
      await ensureChildFlowTokenAccounts({
        connection: provider.connection,
        taker,
        maker,
        inputMint,
        outputMint,
      });
    });

    beforeEach(async () => {
      order = await createExecutableVanillaFillOrder({
        ordoHelper,
        maker: makerWallet,
        taker,
        inputMint,
        outputMint,
        inputAmount: orderInputAmount,
        outputAmount: orderOutputAmount,
        activeDurationSeconds: new BN(0),
      });
    });

    // The order account is deleted when the order is closed, so it is not possible to fill a closed order
    it.skip("Should reject fill order when order is closed", async () => {});

    it("Should reject fill order when order is filled", async () => {
      await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: orderInputAmount,
        minOutputAmount: orderOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      await expectRejects(
        ordoHelper.takeOrder({
          taker: takerWallet,
          order: order,
          inputAmount: orderInputAmount,
          minOutputAmount: orderOutputAmount,
          tipAmountPermissionlessTaking: new BN(0),
        }),
        OrdoError.OrderNotActive,
      );
    });
  });
});
