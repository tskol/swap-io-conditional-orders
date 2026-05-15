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
  STABLE_PRICE_FEED,
} from "./support/flow-environment";
import {
  OrderStatus,
  OrderType,
  OrdoError,
  UpdateGlobalConfigMode,
  UpdateOrderMode,
} from "./support/ordo-constants";

describe("Safe cancellation and full unwind", () => {
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
      makerInputBalance: 1000000000000,
      takerOutputBalance: 1000000000000,
      seedCounterpartyBalances: {
        inputAmount: 100000000000,
        outputAmount: 100000000000,
      },
    });
    inputMint = flow.inputMint;
    outputMint = flow.outputMint;
  });

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

  describe("Cancel Type A order & refund remaining input", () => {
    let order: web3.PublicKey;
    const orderInputAmount = new BN(100000000000);
    const orderOutputAmount = new BN(200000000000);
    const activeDurationSeconds = new BN(10);

    beforeEach(async () => {
      const { signature, order: orderPubkey } = await ordoHelper.createOrder({
        maker: makerWallet,
        inputMint: inputMint,
        outputMint: outputMint,
        inputAmount: orderInputAmount,
        outputAmount: orderOutputAmount,
        orderType: OrderType.Vanilla,
        activeDurationSeconds: activeDurationSeconds,
      });

      order = orderPubkey;

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateCounterparty,
        value: Array.from(taker.publicKey.toBuffer()),
      });
    });

    it("Cancel active Type A after cooldown", async () => {
      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(outputVaultAta);

      const { signature } = await ordoHelper.closeOrder({
        closer: makerWallet,
        order: order,
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(outputVaultAta);

      const orderAccountInfo = await provider.connection.getAccountInfo(order);

      const tx = await provider.connection.getParsedTransaction(signature, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerInputAtaBalanceBefore.value.amount)
          .add(orderInputAmount)
          .toString(),
      );
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        makerOutputAtaBalanceBefore.value.amount,
      );
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(inputVaultAtaBalanceBefore.value.amount)
          .sub(orderInputAmount)
          .toString(),
      );
      expect(outputVaultAtaBalanceAfter.value.amount).to.equal(
        outputVaultAtaBalanceBefore.value.amount,
      );

      expect(orderAccountInfo).to.be.null;
    });

    it("Cancel Type A before cooldown rejected", async () => {
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
        value: [10],
      });

      await expectRejects(
        ordoHelper.closeOrder({
          closer: makerWallet,
          order: order,
        }),
        OrdoError.NotEnoughTimePassedSinceLastUpdate,
      );

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
        value: [0],
      });
    });

    /// After cancel, order account deleted, so we can't cancel it again.
    it.skip("Cancel already Cancelled/Closed rejected", async () => {});

    it("Cancel Type A after active duration expired by taker", async () => {
      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      await ordoHelper.closeOrder({
        closer: takerWallet,
        order: order,
      });

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      expect(orderAccountInfo).to.be.null;
    });

    it("Cancel Type A after active duration expired by maker", async () => {
      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      await ordoHelper.closeOrder({
        closer: makerWallet,
        order: order,
      });

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      expect(orderAccountInfo).to.be.null;
    });

    it("Should be rejected if closer is taker and active duration is not expired", async () => {
      await expectRejects(
        ordoHelper.closeOrder({
          closer: takerWallet,
          order: order,
        }),
        OrdoError.InvalidAccount,
      );
    });

    it("Cancel Type A with keeper close fee", async () => {
      const keeperCloseFeeBps = new BN(1000);
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(keeperCloseFeeBps.toArray("le", 2)),
      });

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const closerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const closerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      await ordoHelper.closeOrder({
        closer: takerWallet,
        order: order,
      });

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(new BN(0).toArray("le", 2)),
      });

      const expectedKeeperCloseFee = orderInputAmount
        .mul(keeperCloseFeeBps)
        .div(new BN(10000));

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const orderAccountInfo = await provider.connection.getAccountInfo(order);

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerInputAtaBalanceBefore.value.amount)
          .add(orderInputAmount)
          .sub(expectedKeeperCloseFee)
          .toString(),
      );
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        makerOutputAtaBalanceBefore.value.amount,
      );
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(inputVaultAtaBalanceBefore.value.amount)
          .sub(orderInputAmount)
          .toString(),
      );
      expect(outputVaultAtaBalanceAfter.value.amount).to.equal(
        outputVaultAtaBalanceBefore.value.amount,
      );
      expect(closerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(closerInputAtaBalanceBefore.value.amount)
          .add(expectedKeeperCloseFee)
          .toString(),
      );
      expect(closerOutputAtaBalanceAfter.value.amount).to.equal(
        closerOutputAtaBalanceBefore.value.amount,
      );

      expect(orderAccountInfo).to.be.null;
    });

    it("Cancel Type A without keeper close fee if not enough input amount to cover the fee", async () => {
      const keeperCloseFeeBps = new BN(5001);
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(keeperCloseFeeBps.toArray("le", 2)),
      });
      const fillInputAmount = orderInputAmount.div(new BN(2));
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );
      await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const closerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const closerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      await ordoHelper.closeOrder({
        closer: takerWallet,
        order: order,
      });

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(new BN(0).toArray("le", 2)),
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const orderAccountInfo = await provider.connection.getAccountInfo(order);

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerInputAtaBalanceBefore.value.amount)
          .add(fillInputAmount)
          .toString(),
      );
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        makerOutputAtaBalanceBefore.value.amount,
      );
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(inputVaultAtaBalanceBefore.value.amount)
          .sub(fillInputAmount)
          .toString(),
      );
      expect(outputVaultAtaBalanceAfter.value.amount).to.equal(
        outputVaultAtaBalanceBefore.value.amount,
      );
      expect(closerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(closerInputAtaBalanceBefore.value.amount).toString(),
      );
      expect(closerOutputAtaBalanceAfter.value.amount).to.equal(
        closerOutputAtaBalanceBefore.value.amount,
      );

      expect(orderAccountInfo).to.be.null;
    });
  });

  describe("Cancel Type B order & unwind parent + child vaults", () => {
    let order: web3.PublicKey;
    let tpOrder: web3.PublicKey;
    let slOrder: web3.PublicKey;
    const orderInputAmount = new BN(100000000000);
    const orderOutputAmount = new BN(200000000000);
    const tpOutputAmount = new BN(120000000000);
    const slOutputAmount = new BN(80000000000);
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

    beforeEach(async () => {
      const {
        signature,
        order: orderPubkey,
        tpOrder: tpOrderPubkey,
        slOrder: slOrderPubkey,
      } = await ordoHelper.createOrder({
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

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateCounterparty,
        value: Array.from(taker.publicKey.toBuffer()),
      });
    });

    it("Cancel Type B with parent+child balances", async () => {
      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

      const fillInputAmount = orderInputAmount.div(new BN(2));
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(outputVaultAta);

      const { signature } = await ordoHelper.closeOrder({
        closer: makerWallet,
        order: order,
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(outputVaultAta);

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      const tpOrderAccountInfo =
        await provider.connection.getAccountInfo(tpOrder);
      const slOrderAccountInfo =
        await provider.connection.getAccountInfo(slOrder);

      const tx = await provider.connection.getParsedTransaction(signature, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerInputAtaBalanceBefore.value.amount)
          .add(orderInputAmount.sub(fillInputAmount))
          .toString(),
      );
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerOutputAtaBalanceBefore.value.amount)
          .add(fillMinOutputAmount)
          .toString(),
      );
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(inputVaultAtaBalanceBefore.value.amount)
          .sub(orderInputAmount.sub(fillInputAmount))
          .toString(),
      );
      expect(outputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(outputVaultAtaBalanceBefore.value.amount)
          .sub(fillMinOutputAmount)
          .toString(),
      );

      expect(orderAccountInfo).to.be.null;
      expect(tpOrderAccountInfo).to.be.null;
      expect(slOrderAccountInfo).to.be.null;
    });

    it("Cancel Type A before cooldown rejected", async () => {
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
        value: [10],
      });

      await expectRejects(
        ordoHelper.closeOrder({
          closer: makerWallet,
          order: order,
        }),
        OrdoError.NotEnoughTimePassedSinceLastUpdate,
      );

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateOrderCloseDelaySeconds,
        value: [0],
      });
    });

    /// After cancel, order account deleted, so we can't cancel it again.
    it.skip("Cancel already Cancelled/Closed rejected", async () => {});

    it("Cancel Type B after active duration expired by taker", async () => {
      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      await ordoHelper.closeOrder({
        closer: takerWallet,
        order: order,
      });

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      const tpOrderAccountInfo =
        await provider.connection.getAccountInfo(tpOrder);
      const slOrderAccountInfo =
        await provider.connection.getAccountInfo(slOrder);

      expect(orderAccountInfo).to.be.null;
      expect(tpOrderAccountInfo).to.be.null;
      expect(slOrderAccountInfo).to.be.null;
    });

    it("Cancel Type B after active duration expired by maker", async () => {
      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      await ordoHelper.closeOrder({
        closer: makerWallet,
        order: order,
      });

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      const tpOrderAccountInfo =
        await provider.connection.getAccountInfo(tpOrder);
      const slOrderAccountInfo =
        await provider.connection.getAccountInfo(slOrder);

      expect(orderAccountInfo).to.be.null;
      expect(tpOrderAccountInfo).to.be.null;
      expect(slOrderAccountInfo).to.be.null;
    });

    it("Should be rejected if closer is taker and active duration is not expired", async () => {
      await expectRejects(
        ordoHelper.closeOrder({
          closer: takerWallet,
          order: order,
        }),
        OrdoError.InvalidAccount,
      );
    });

    it("Cancel Type B with keeper close fee from parent", async () => {
      const keeperCloseFeeBps = new BN(1000);
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(keeperCloseFeeBps.toArray("le", 2)),
      });

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const closerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const closerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

      const fillInputAmount = orderInputAmount.div(new BN(2));
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      const { signature } = await ordoHelper.closeOrder({
        closer: takerWallet,
        order: order,
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const expectedKeeperCloseFee = orderInputAmount
        .mul(keeperCloseFeeBps)
        .div(new BN(10000));

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      const tpOrderAccountInfo =
        await provider.connection.getAccountInfo(tpOrder);
      const slOrderAccountInfo =
        await provider.connection.getAccountInfo(slOrder);

      const tx = await provider.connection.getParsedTransaction(signature, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerInputAtaBalanceBefore.value.amount)
          .add(
            orderInputAmount.sub(fillInputAmount).sub(expectedKeeperCloseFee),
          )
          .toString(),
      );
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerOutputAtaBalanceBefore.value.amount)
          .add(fillMinOutputAmount)
          .toString(),
      );
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(inputVaultAtaBalanceBefore.value.amount)
          .sub(orderInputAmount.sub(fillInputAmount))
          .toString(),
      );
      expect(outputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(outputVaultAtaBalanceBefore.value.amount)
          .sub(fillMinOutputAmount)
          .toString(),
      );
      expect(closerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(closerInputAtaBalanceBefore.value.amount)
          .add(expectedKeeperCloseFee)
          .toString(),
      );
      expect(closerOutputAtaBalanceAfter.value.amount).to.equal(
        closerOutputAtaBalanceBefore.value.amount,
      );

      expect(orderAccountInfo).to.be.null;
      expect(tpOrderAccountInfo).to.be.null;
      expect(slOrderAccountInfo).to.be.null;

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(new BN(0).toArray("le", 2)),
      });
    });

    it("Cancel Type B with keeper close fee from child if not enough input amount to cover the fee", async () => {
      const keeperCloseFeeBps = new BN(5001);
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(keeperCloseFeeBps.toArray("le", 2)),
      });

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const closerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const closerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

      const fillInputAmount = orderInputAmount.mul(new BN(3)).div(new BN(4));
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      const { signature } = await ordoHelper.closeOrder({
        closer: takerWallet,
        order: order,
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const expectedKeeperCloseFee = orderOutputAmount
        .mul(keeperCloseFeeBps)
        .div(new BN(10000));

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      const tpOrderAccountInfo =
        await provider.connection.getAccountInfo(tpOrder);
      const slOrderAccountInfo =
        await provider.connection.getAccountInfo(slOrder);

      const tx = await provider.connection.getParsedTransaction(signature, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerInputAtaBalanceBefore.value.amount)
          .add(orderInputAmount.sub(fillInputAmount))
          .toString(),
      );
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerOutputAtaBalanceBefore.value.amount)
          .add(fillMinOutputAmount)
          .sub(expectedKeeperCloseFee)
          .toString(),
      );
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(inputVaultAtaBalanceBefore.value.amount)
          .sub(orderInputAmount.sub(fillInputAmount))
          .toString(),
      );
      expect(outputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(outputVaultAtaBalanceBefore.value.amount)
          .sub(fillMinOutputAmount)
          .toString(),
      );
      expect(closerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(closerInputAtaBalanceBefore.value.amount).toString(),
      );
      expect(closerOutputAtaBalanceAfter.value.amount).to.equal(
        new BN(closerOutputAtaBalanceBefore.value.amount)
          .add(expectedKeeperCloseFee)
          .toString(),
      );

      expect(orderAccountInfo).to.be.null;
      expect(tpOrderAccountInfo).to.be.null;
      expect(slOrderAccountInfo).to.be.null;

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(new BN(0).toArray("le", 2)),
      });
    });

    it("Cancel Type B without keeper close fee from parent and child if not enough input amount to cover the fee", async () => {
      const keeperCloseFeeBps = new BN(5001);
      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(keeperCloseFeeBps.toArray("le", 2)),
      });

      const makerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        maker.publicKey,
      );
      const makerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        maker.publicKey,
      );
      const closerInputAta = spl.getAssociatedTokenAddressSync(
        inputMint,
        taker.publicKey,
      );
      const closerOutputAta = spl.getAssociatedTokenAddressSync(
        outputMint,
        taker.publicKey,
      );
      const { vault: inputVaultAta } = await ordoHelper.getVault(inputMint);
      const { vault: outputVaultAta } = await ordoHelper.getVault(outputMint);

      const fillInputAmount = orderInputAmount.div(new BN(2));
      const fillMinOutputAmount = await calcMinOutputAmount(
        fillInputAmount,
        order,
      );

      await ordoHelper.takeOrder({
        taker: takerWallet,
        order: order,
        inputAmount: fillInputAmount,
        minOutputAmount: fillMinOutputAmount,
        tipAmountPermissionlessTaking: new BN(0),
      });

      const makerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceBefore =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const waitMs = (activeDurationSeconds.toNumber() + 1) * 1000;
      await new Promise((resolve) => setTimeout(resolve, waitMs));

      const { signature } = await ordoHelper.closeOrder({
        closer: takerWallet,
        order: order,
      });

      const makerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerInputAta);
      const makerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(makerOutputAta);
      const inputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(inputVaultAta);
      const outputVaultAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(outputVaultAta);
      const closerInputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerInputAta);
      const closerOutputAtaBalanceAfter =
        await provider.connection.getTokenAccountBalance(closerOutputAta);

      const orderAccountInfo = await provider.connection.getAccountInfo(order);
      const tpOrderAccountInfo =
        await provider.connection.getAccountInfo(tpOrder);
      const slOrderAccountInfo =
        await provider.connection.getAccountInfo(slOrder);

      const tx = await provider.connection.getParsedTransaction(signature, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      expect(makerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerInputAtaBalanceBefore.value.amount)
          .add(orderInputAmount.sub(fillInputAmount))
          .toString(),
      );
      expect(makerOutputAtaBalanceAfter.value.amount).to.equal(
        new BN(makerOutputAtaBalanceBefore.value.amount)
          .add(fillMinOutputAmount)
          .toString(),
      );
      expect(inputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(inputVaultAtaBalanceBefore.value.amount)
          .sub(orderInputAmount.sub(fillInputAmount))
          .toString(),
      );
      expect(outputVaultAtaBalanceAfter.value.amount).to.equal(
        new BN(outputVaultAtaBalanceBefore.value.amount)
          .sub(fillMinOutputAmount)
          .toString(),
      );
      expect(closerInputAtaBalanceAfter.value.amount).to.equal(
        new BN(closerInputAtaBalanceBefore.value.amount).toString(),
      );
      expect(closerOutputAtaBalanceAfter.value.amount).to.equal(
        new BN(closerOutputAtaBalanceBefore.value.amount).toString(),
      );

      expect(orderAccountInfo).to.be.null;
      expect(tpOrderAccountInfo).to.be.null;
      expect(slOrderAccountInfo).to.be.null;

      await ordoHelper.updateGlobalConfig({
        payer: payerWallet,
        mode: UpdateGlobalConfigMode.UpdateKeeperCloseFeeBps,
        value: Array.from(new BN(0).toArray("le", 2)),
      });
    });
  });
});
