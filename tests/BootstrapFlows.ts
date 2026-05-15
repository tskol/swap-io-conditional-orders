import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { web3, BN } from "@coral-xyz/anchor";
import { expect } from "chai";
import { OrdoHelper } from "./support/ordo-client";
import {
  createFlowActors,
  createFlowProvider,
  ensureLocalValidator,
  STABLE_PRICE_FEED,
} from "./support/flow-environment";

describe("BootstrapFlows", () => {
  const { connection, provider } = createFlowProvider();
  const { payer, payerWallet } = createFlowActors();

  const ordoHelper = new OrdoHelper(provider);

  before(async function () {
    await ensureLocalValidator(connection);
  });

  describe("Initialize Global Config", () => {
    it("Should initialize global config", async () => {
      const { signature, globalConfig, pdaAuthority, pdaAuthorityBump } =
        await ordoHelper.initializeGlobalConfig({
          payer: payerWallet,
        });

      const configAccount = await ordoHelper.getGlobalConfigAccount();

      expect(configAccount.flashTakeOrderBlocked).to.equal(0);
      expect(configAccount.newOrdersBlocked).to.equal(0);
      expect(configAccount.ordersTakingBlocked).to.equal(0);
      expect(configAccount.hostFeeBps).to.equal(0);
      expect(configAccount.orderCloseDelaySeconds.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(
        configAccount.pdaAuthorityPreviousLamportsBalance.toString(),
      ).to.equal(new BN(0).toString());
      expect(configAccount.totalTipAmount.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(configAccount.hostTipAmount.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(configAccount.pdaAuthority).to.deep.equal(pdaAuthority);
      expect(configAccount.pdaAuthorityBump.toString()).to.equal(
        new BN(pdaAuthorityBump).toString(),
      );
      expect(configAccount.adminAuthority).to.deep.equal(payer.publicKey);
      expect(configAccount.adminAuthorityCached).to.deep.equal(payer.publicKey);
      expect(configAccount.emergencyMode).to.equal(0);
      expect(configAccount.ataCreationCost.toString()).to.equal(
        new BN(0).toString(),
      );
      expect(configAccount.tpSlEnabled).to.equal(1);
      expect(configAccount.oracleMaxStalenessSeconds.toString()).to.equal(
        new BN(30).toString(),
      );
      expect(configAccount.createOrderFeeBps).to.equal(0);
      expect(configAccount.slMaxUpwardDeviationBps).to.equal(0);
      expect(configAccount.tpSlMinDistanceBps).to.equal(0);
      expect(configAccount.parentFillFeeKeeperBps).to.equal(0);
      expect(configAccount.parentFillFeeProtocolBps).to.equal(0);
      expect(configAccount.tpSlChildFeeKeeperBps).to.equal(0);
      expect(configAccount.tpSlChildFeeProtocolBps).to.equal(0);
      expect(configAccount.txnFeeCost.toString()).to.equal(
        new BN(0).toString(),
      );
    });
  });

  describe("Initialize Vault", () => {
    const mintAuthority = web3.Keypair.generate();

    it("Should initialize vault", async () => {
      const decimals = 9;
      const mint = await spl.createMint(
        connection,
        payer,
        mintAuthority.publicKey,
        null,
        decimals,
      );

      const { signature, vault, feeVault } = await ordoHelper.initializeVault({
        payer: payerWallet,
        mint,
      });
      const { pdaAuthority } = await ordoHelper.getPdaAuthority(
        ordoHelper.getGlobalConfig(),
      );

      const vaultAta = await spl.getAccount(provider.connection, vault);
      const feeVaultAta = await spl.getAccount(provider.connection, feeVault);

      expect(vaultAta.isInitialized).to.equal(true);
      expect(feeVaultAta.isInitialized).to.equal(true);

      expect(vaultAta.mint).to.deep.equal(mint);
      expect(feeVaultAta.mint).to.deep.equal(mint);

      expect(vaultAta.owner).to.deep.equal(pdaAuthority);
      expect(feeVaultAta.owner).to.deep.equal(pdaAuthority);
    });
  });

  describe("Initialize Oracle Pool", () => {
    const mintAuthority = web3.Keypair.generate();

    it("Should initialize oracle pool", async () => {
      const decimals = 9;
      const mint = await spl.createMint(
        connection,
        payer,
        mintAuthority.publicKey,
        null,
        decimals,
      );

      const { signature, oraclePool } = await ordoHelper.initializeOraclePool({
        payer: payerWallet,
        mint,
        feedId: STABLE_PRICE_FEED,
      });

      const oraclePoolAccount = await ordoHelper.getOraclePoolAccount(mint);

      // Remove '0x' prefix if present and decode hex to number array
      const feedIdHex = STABLE_PRICE_FEED.startsWith("0x")
        ? STABLE_PRICE_FEED.slice(2)
        : STABLE_PRICE_FEED;
      const feedIdArray = Array.from(anchor.utils.bytes.hex.decode(feedIdHex));

      expect(oraclePoolAccount.globalConfig).to.deep.equal(
        ordoHelper.getGlobalConfig(),
      );
      expect(oraclePoolAccount.oracleFeedId).to.deep.equal(feedIdArray);
      expect(oraclePoolAccount.tokenMint).to.deep.equal(mint);
    });
  });
});
