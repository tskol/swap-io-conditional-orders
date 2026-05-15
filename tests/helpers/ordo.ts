import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import * as memo from "@solana/spl-memo";
import IDL from "../../target/idl/ordo.json";
import { BN, web3 } from "@coral-xyz/anchor";
import { SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import { Ordo } from "../../target/types/ordo";
import { findLargestTokenAccount, generateRandomOrdoAccounts } from "./utils";
import {
  Price,
  PriceServiceConnection,
} from "@pythnetwork/price-service-client";
import {
  InstructionWithEphemeralSigners,
  // PythSolanaReceiver,
} from "@pythnetwork/pyth-solana-receiver";
import { TransactionSender } from "./transaction-sender";
import { ProgramUtils } from "./program-utils";
import { Wallet } from "@coral-xyz/anchor";
import { 
  GLOBAL_CONFIG_SIZE,
  ORDER_SIZE,
  OrderType,
  PROGRAM_ID,
  PYTH_ENDPOINT,
  ESCROW_VAULT_SEED,
  FEE_VAULT_SEED,
  GLOBAL_AUTH_SEED,
  ORACLE_POOL_SEED,
  UPDATE_GLOBAL_CONFIG_BYTE_SIZE,
  UpdateOrderMode,
} from "./constants";

export type GlobalConfigAccount = anchor.IdlAccounts<Ordo>["globalConfig"];
export type OraclePoolAccount = anchor.IdlAccounts<Ordo>["oraclePoolsState"];
export type OrderAccount = anchor.IdlAccounts<Ordo>["order"];

export class OrdoHelper extends TransactionSender {
    public readonly program: anchor.Program<Ordo>;
    public readonly provider: anchor.AnchorProvider;
    public readonly pythEndpoint: string;
    public readonly pythConnection: PriceServiceConnection;

    private globalConfig: web3.PublicKey;
  
    constructor(
        provider: anchor.AnchorProvider,
        pythEndpoint = PYTH_ENDPOINT,
    ) {
        super(provider.connection);
        this.provider = provider;
        this.pythEndpoint = pythEndpoint;
        this.program = new anchor.Program(IDL as anchor.Idl, PROGRAM_ID, provider) as anchor.Program<Ordo>;
        this.pythConnection = new PriceServiceConnection(pythEndpoint, {
          priceFeedRequestConfig: {
            binary: true,
          },
        });
    }

    async getPdaAuthority(globalConfig?: web3.PublicKey): Promise<{ pdaAuthority: web3.PublicKey, bump: number }> {
      const globalConfigPubkey = globalConfig ?? this.globalConfig;
      const [pdaAuthority, bump] = web3.PublicKey.findProgramAddressSync(
          [
              anchor.utils.bytes.utf8.encode(GLOBAL_AUTH_SEED),
              globalConfigPubkey.toBuffer(),
          ],
          this.program.programId
      );
      return { pdaAuthority, bump };
    }

    async getVault(mint: web3.PublicKey, globalConfig?: web3.PublicKey): Promise<{ vault: web3.PublicKey, bump: number }> {
      const globalConfigPubkey = globalConfig ?? this.globalConfig;
      const [vault, bump] = web3.PublicKey.findProgramAddressSync(
          [
            anchor.utils.bytes.utf8.encode(ESCROW_VAULT_SEED),
            globalConfigPubkey.toBuffer(),
            mint.toBuffer(),
          ],
          this.program.programId
        );
        return { vault, bump };
    }

    async getFeeVault(mint: web3.PublicKey, globalConfig?: web3.PublicKey): Promise<{ feeVault: web3.PublicKey, bump: number }> {
      const globalConfigPubkey = globalConfig ?? this.globalConfig;
      const [feeVault, bump] = web3.PublicKey.findProgramAddressSync(
          [
            anchor.utils.bytes.utf8.encode(FEE_VAULT_SEED),
            globalConfigPubkey.toBuffer(),
            mint.toBuffer(),
          ],
          this.program.programId
        );
        return { feeVault, bump };
    }

    async getOraclePool(mint: web3.PublicKey, globalConfig?: web3.PublicKey): Promise<{ oraclePool: web3.PublicKey, bump: number }> {
      const globalConfigPubkey = globalConfig ?? this.globalConfig;
      const [oraclePool, bump] = web3.PublicKey.findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode(ORACLE_POOL_SEED),
          globalConfigPubkey.toBuffer(),
          mint.toBuffer(),
        ],
        this.program.programId
      );
      return { oraclePool, bump };
    }

    getGlobalConfig(): web3.PublicKey {
        return this.globalConfig;
    }

    setGlobalConfig(globalConfig: web3.PublicKey) {
      this.globalConfig = globalConfig;
    }

    async getGlobalConfigAccount(): Promise<GlobalConfigAccount> {
      return await this.program.account.globalConfig.fetch(this.globalConfig);
    }

    async getOraclePoolAccount(mint: web3.PublicKey): Promise<OraclePoolAccount> {
      const { oraclePool } = await this.getOraclePool(mint);
      return await this.program.account.oraclePoolsState.fetch(oraclePool);
    }

    async getOrderAccount(order: web3.PublicKey): Promise<OrderAccount> {
      return await this.program.account.order.fetch(order);
    }

    async initializeGlobalConfig(args: {
            payer: Wallet;
      }): Promise<{
        signature: string;
        globalConfig: web3.PublicKey;
        pdaAuthority: web3.PublicKey;
        pdaAuthorityBump: number;
      }> {
        // Generate keypair for globalConfig (zero account)
        const globalConfig = web3.Keypair.generate();
        
        // Derive pdaAuthority from globalConfig
        const { pdaAuthority, bump: pdaAuthorityBump } = await this.getPdaAuthority(globalConfig.publicKey);

        // Create the globalConfig account with zero initialization
        const createAccountIx = web3.SystemProgram.createAccount({
            fromPubkey: args.payer.publicKey,
            newAccountPubkey: globalConfig.publicKey,
            lamports: await this.provider.connection.getMinimumBalanceForRentExemption(GLOBAL_CONFIG_SIZE),
            space: GLOBAL_CONFIG_SIZE,
            programId: this.program.programId,
        });

        const ix = await this.program.methods.initializeGlobalConfig()
        .accounts({
            adminAuthority: args.payer.publicKey,
            pdaAuthority: pdaAuthority,
            globalConfig: globalConfig.publicKey,
        }).instruction()

        const { signature } = await this.sendTransaction(
            args.payer,
            [createAccountIx, ix],
            [globalConfig], // globalConfig keypair needs to sign the createAccount instruction
        );

        this.globalConfig = globalConfig.publicKey;

        return { signature, globalConfig: globalConfig.publicKey, pdaAuthority, pdaAuthorityBump };
      }

      async initializeVault(args: {
        payer: Wallet;
        mint: web3.PublicKey;
        globalConfig?: web3.PublicKey;
      }): Promise<{
        signature: string;
        vault: web3.PublicKey;
        feeVault: web3.PublicKey;
      }> {
        const globalConfig = args.globalConfig ?? this.globalConfig;

        const { pdaAuthority } = await this.getPdaAuthority(globalConfig);

        const { vault } = await this.getVault(args.mint, globalConfig);
        const { feeVault } = await this.getFeeVault(args.mint, globalConfig);

        const tokenProgram = (await this.provider.connection.getAccountInfo(args.mint))?.owner;
        const ix = await this.program.methods.initializeVault()
        .accounts({
          payer: args.payer.publicKey,
          globalConfig: globalConfig,
          mint: args.mint,
          tokenProgram: tokenProgram,
          pdaAuthority: pdaAuthority,
          vault: vault,
          feeVault: feeVault,
        })
        .instruction();

        const { signature } = await this.sendTransaction(
            args.payer,
            [ix],
        );

        return { signature, vault, feeVault };
      }

      async initializeOraclePool(args: {
        payer: Wallet;
        mint: web3.PublicKey;
        globalConfig?: web3.PublicKey;
        feedId: string;
      }): Promise<{
        signature: string;
        oraclePool: web3.PublicKey;
      }> {
        const globalConfig = args.globalConfig ?? this.globalConfig;
        
        const { oraclePool } = await this.getOraclePool(args.mint);
        const ix = await this.program.methods.initializeOraclePool(
          args.feedId
        )
        .accounts({
          adminAuthority: args.payer.publicKey,
          globalConfig: globalConfig,
          tokenMint: args.mint,
          oraclePool: oraclePool,
        })
        .instruction();

        const { signature } = await this.sendTransaction(args.payer, [ix]);
        return { signature, oraclePool };
      }

      async updateGlobalConfig(args: {
        payer: Wallet;
        globalConfig?: web3.PublicKey;
        mode: number;
        value: number[];
      }): Promise<{
        signature: string;
      }> {
        const globalConfig = args.globalConfig ?? this.globalConfig;

        const valueSize = args.value.length;
        if (valueSize > UPDATE_GLOBAL_CONFIG_BYTE_SIZE) {
          throw new Error(`Value size is too large, max is ${UPDATE_GLOBAL_CONFIG_BYTE_SIZE}`);
        } else if (valueSize < UPDATE_GLOBAL_CONFIG_BYTE_SIZE) {
          args.value = args.value.concat(new Array(UPDATE_GLOBAL_CONFIG_BYTE_SIZE - valueSize).fill(0));
        }
        
        const ix = await this.program.methods.updateGlobalConfig(
          args.mode,
          args.value,
        )
        .accounts({
          adminAuthority: args.payer.publicKey,
          globalConfig: globalConfig,
        })
        .instruction();

        const { signature } = await this.sendTransaction(args.payer, [ix]);
        return { signature };
      }

      async createOrder(args: {
        maker: Wallet;
        inputMint: web3.PublicKey;
        outputMint: web3.PublicKey;
        inputAmount: anchor.BN;
        outputAmount: anchor.BN;
        orderType: number;
        tpOutputAmount?: anchor.BN;
        slOutputAmount?: anchor.BN;
        activeDurationSeconds?: anchor.BN;
        globalConfig?: web3.PublicKey;
      }): Promise<{
        signature: string;
        order: web3.PublicKey;
        tpOrder?: web3.PublicKey;
        slOrder?: web3.PublicKey;
      }> {
        const globalConfig = args.globalConfig ?? this.globalConfig;
        const { pdaAuthority } = await this.getPdaAuthority(globalConfig);

        const keypairs: web3.Keypair[] = [];

        // Create keypairs for orders
        const order = web3.Keypair.generate();
        keypairs.push(order);
        let tpOrder: web3.Keypair | undefined;
        let slOrder: web3.Keypair | undefined;
        if (args.tpOutputAmount && args.tpOutputAmount.gt(new anchor.BN(0))) {
          tpOrder = web3.Keypair.generate();
          keypairs.push(tpOrder);
        }
        if (args.slOutputAmount && args.slOutputAmount.gt(new anchor.BN(0))) {
          slOrder = web3.Keypair.generate();
          keypairs.push(slOrder);
        }

        // Get vaults
        const { vault: inputVault } = await this.getVault(args.inputMint, globalConfig);
        const { feeVault: inputFeeVault } = await this.getFeeVault(args.inputMint, globalConfig);
        const { vault: outputVault } = await this.getVault(args.outputMint, globalConfig);

        // Get token programs
        const inputMintInfo = await this.provider.connection.getAccountInfo(args.inputMint);
        const outputMintInfo = await this.provider.connection.getAccountInfo(args.outputMint);
        const inputTokenProgram = inputMintInfo?.owner;
        const outputTokenProgram = outputMintInfo?.owner;

        // Get maker ATA
        const makerAta = spl.getAssociatedTokenAddressSync(
          args.inputMint,
          args.maker.publicKey,
          false,
          inputTokenProgram,
        );

        const ixs: anchor.web3.TransactionInstruction[] = [];

        // Create account instructions for orders
        const createOrderAccountIx = web3.SystemProgram.createAccount({
          fromPubkey: args.maker.publicKey,
          newAccountPubkey: order.publicKey,
          lamports: await this.provider.connection.getMinimumBalanceForRentExemption(ORDER_SIZE),
          space: ORDER_SIZE,
          programId: this.program.programId,
        });
        ixs.push(createOrderAccountIx);

        if (tpOrder) {
          const createTpOrderAccountIx = web3.SystemProgram.createAccount({
            fromPubkey: args.maker.publicKey,
            newAccountPubkey: tpOrder.publicKey,
            lamports: await this.provider.connection.getMinimumBalanceForRentExemption(ORDER_SIZE),
            space: ORDER_SIZE,
            programId: this.program.programId,
          });
          ixs.push(createTpOrderAccountIx);
        }

        if (slOrder) {
          const createSlOrderAccountIx = web3.SystemProgram.createAccount({
            fromPubkey: args.maker.publicKey,
            newAccountPubkey: slOrder.publicKey,
            lamports: await this.provider.connection.getMinimumBalanceForRentExemption(ORDER_SIZE),
            space: ORDER_SIZE,
            programId: this.program.programId,
          });
          ixs.push(createSlOrderAccountIx);
        }

        // Create order instruction
        const ix = await this.program.methods.createOrder(
          args.inputAmount,
          args.outputAmount,
          args.orderType,
          args.tpOutputAmount ?? new anchor.BN(0),
          args.slOutputAmount ?? new anchor.BN(0),
          args.activeDurationSeconds ?? new anchor.BN(0),
        )
        .accounts({
          maker: args.maker.publicKey,
          globalConfig: globalConfig,
          pdaAuthority: pdaAuthority,
          order: order.publicKey,
          tpOrder: tpOrder ? tpOrder.publicKey : null,
          slOrder: slOrder ? slOrder.publicKey : null,
          inputMint: args.inputMint,
          outputMint: args.outputMint,
          makerAta: makerAta,
          inputVault: inputVault,
          inputFeeVault: inputFeeVault,
          outputVault: outputVault,
          inputTokenProgram: inputTokenProgram,
          outputTokenProgram: outputTokenProgram,
          systemProgram: web3.SystemProgram.programId,
        })
        .instruction();

        ixs.push(ix);

        const { signature } = await this.sendTransaction(
          args.maker,
          ixs,
          keypairs,
        );

        const result: {
          signature: string;
          order: web3.PublicKey;
          tpOrder?: web3.PublicKey;
          slOrder?: web3.PublicKey;
        } = { signature, order: order.publicKey };

        // Only include tp/sl orders if they were actually used
        if (args.tpOutputAmount && args.tpOutputAmount.gt(new anchor.BN(0))) {
          result.tpOrder = tpOrder.publicKey;
        }
        if (args.slOutputAmount && args.slOutputAmount.gt(new anchor.BN(0))) {
          result.slOrder = slOrder.publicKey;
        }

        return result;
      }

      async updateOrder(args: {
        maker: Wallet;
        order: web3.PublicKey;
        mode: UpdateOrderMode;
        value: Buffer;
      }): Promise<{
        signature: string;
      }> {
        const ix = await this.program.methods.updateOrder(
          args.mode,
          args.value,
        )
        .accounts({
          maker: args.maker.publicKey,
          globalConfig: this.globalConfig,
          order: args.order,
        })
        .instruction();

        const { signature } = await this.sendTransaction(args.maker, [ix]);
        return { signature };
      }

      async takeOrder(args: {
        taker: Wallet;
        order: web3.PublicKey;
        inputAmount: anchor.BN;
        minOutputAmount: anchor.BN;
        tipAmountPermissionlessTaking: anchor.BN;
        globalConfig?: web3.PublicKey;
      }): Promise<{
        signatures: string[];
      }> {
        const globalConfig = args.globalConfig ?? this.globalConfig;
        const { pdaAuthority } = await this.getPdaAuthority(globalConfig);

        const orderAccount = await this.getOrderAccount(args.order);
        const inputMint = orderAccount.inputMint;
        const outputMint = orderAccount.outputMint;
        const maker = orderAccount.maker;
        const parentOrder = orderAccount.parentOrder;

        let brotherOrder: web3.PublicKey | undefined = undefined;
        if (parentOrder.toBase58() != web3.PublicKey.default.toBase58()) {
          const parentOrderAccount = await this.getOrderAccount(parentOrder);
          brotherOrder = parentOrderAccount.tpChildOrder.toBase58() == args.order.toBase58() ? parentOrderAccount.slChildOrder : parentOrderAccount.tpChildOrder;
        }

        const { vault: inputVault } = await this.getVault(inputMint, globalConfig);
        const { feeVault: outputFeeVault } = await this.getFeeVault(outputMint, globalConfig);
        const { vault: outputVault } = await this.getVault(outputMint, globalConfig);
        const { oraclePool: inputOraclePool } = await this.getOraclePool(inputMint);
        const { oraclePool: outputOraclePool } = await this.getOraclePool(outputMint);

        const inputMintInfo = await this.provider.connection.getAccountInfo(inputMint);
        const outputMintInfo = await this.provider.connection.getAccountInfo(outputMint);
        const inputTokenProgram = inputMintInfo?.owner;
        const outputTokenProgram = outputMintInfo?.owner;

        const takerInputAta = spl.getAssociatedTokenAddressSync(
          inputMint,
          args.taker.publicKey,
          false,
          inputTokenProgram,
        );
        const takerOutputAta = spl.getAssociatedTokenAddressSync(
          outputMint,
          args.taker.publicKey,
          false,
          outputTokenProgram,
        );
        const makerOutputAta = spl.getAssociatedTokenAddressSync(
          outputMint,
          maker,
          false,
          outputTokenProgram,
        );

        if (orderAccount.orderType === OrderType.LimitSL) {
          const PythSolanaReceiver = (await import(
            "@pythnetwork/pyth-solana-receiver"
          )).PythSolanaReceiver;
          const pyth = new PythSolanaReceiver({
            connection: this.provider.connection,
            wallet: args.taker,
          });

          const inputOraclePoolAccount = await this.getOraclePoolAccount(inputMint);
          const outputOraclePoolAccount = await this.getOraclePoolAccount(outputMint);

          const inputFeedId = "0x" + (inputOraclePoolAccount.oracleFeedId.map((x) => {
            let hex = x.toString(16);
            if (hex.length < 2) {
              hex = "0" + hex;
            }
            return hex;
          }).join(''));
          const outputFeedId = "0x" + (outputOraclePoolAccount.oracleFeedId.map((x) => {
            let hex = x.toString(16);
            if (hex.length < 2) {
              hex = "0" + hex;
            }
            return hex;
          }).join(''));

          const priceUpdateData = await this.pythConnection.getLatestVaas([inputFeedId, outputFeedId]);
          const builder = pyth.newTransactionBuilder({ closeUpdateAccounts: true });
          await builder.addPostPriceUpdates(priceUpdateData);

          await builder.addPriceConsumerInstructions(
            async (
              getPriceUpdateAccount: (priceFeedId: string) => web3.PublicKey,
            ) => {
              const inputPriceUpdate = getPriceUpdateAccount(inputFeedId);
              const outputPriceUpdate = getPriceUpdateAccount(outputFeedId);
              return [
                {
                  instruction: await this.program.methods.takeOrder(
                    args.inputAmount,
                    args.minOutputAmount,
                    args.tipAmountPermissionlessTaking,
                  )
                  .accounts({
                    taker: args.taker.publicKey,
                    maker,
                    globalConfig,
                    pdaAuthority,
                    order: args.order,
                    parentOrder: parentOrder.toBase58() == web3.PublicKey.default.toBase58() ? null : parentOrder,
                    brotherOrder: brotherOrder ? brotherOrder : null,
                    inputMint,
                    outputMint,
                    inputVault,
                    outputVault,
                    outputFeeVault,
                    outputOraclePool,
                    inputOraclePool,
                    inputPriceUpdate: inputPriceUpdate,
                    outputPriceUpdate: outputPriceUpdate,
                    takerInputAta,
                    takerOutputAta,
                    intermediaryOutputTokenAccount: null,
                    makerOutputAta,
                    sysvarInstructions: SYSVAR_INSTRUCTIONS_PUBKEY,
                    // expressRelay: EXPRESS_RELAY_ID,
                    // expressRelayMetadata: EXPRESS_RELAY_METADATA_PUBKEY,
                    // permission: null,
                    // configRouter: EXPRESS_RELAY_CONFIG_ROUTER_PUBKEY,
                    inputTokenProgram,
                    outputTokenProgram
                  })
                  .instruction(),
                  signers: [],
                },
              ];
            },
          );
      
          const txs = await builder.buildVersionedTransactions({});

          const signatures = [];
          for (const tx of txs) {
            tx.tx.sign(tx.signers);
          }

          const signedTxs = await args.taker.signAllTransactions(txs.map((t) => t.tx));

          for (const signedTx of signedTxs) {
            const signature = await this.connection.sendTransaction(signedTx);
            const blockhash = await this.connection.getLatestBlockhash();
            await this.connection.confirmTransaction({ signature, ...blockhash });
            signatures.push(signature);
          }

          return { signatures };
        } else {
          const ix = await this.program.methods.takeOrder(
            args.inputAmount,
            args.minOutputAmount,
            args.tipAmountPermissionlessTaking,
          )
          .accounts({
            taker: args.taker.publicKey,
            maker,
            globalConfig,
            pdaAuthority,
            order: args.order,
            parentOrder: parentOrder.toBase58() == web3.PublicKey.default.toBase58() ? null : parentOrder,
            brotherOrder: brotherOrder ? brotherOrder : null,
            inputMint,
            outputMint,
            inputVault,
            outputVault,
            outputFeeVault,
            outputOraclePool,
            inputOraclePool,
            inputPriceUpdate: null,
            outputPriceUpdate: null,
            takerInputAta,
            takerOutputAta,
            intermediaryOutputTokenAccount: null,
            makerOutputAta,
            sysvarInstructions: SYSVAR_INSTRUCTIONS_PUBKEY,
            // expressRelay: EXPRESS_RELAY_ID,
            // expressRelayMetadata: EXPRESS_RELAY_METADATA_PUBKEY,
            // permission: null,
            // configRouter: EXPRESS_RELAY_CONFIG_ROUTER_PUBKEY,
            inputTokenProgram,
            outputTokenProgram
          })
          .instruction();
  
          const { signature } = await this.sendTransaction(args.taker, [ix]);
          return { signatures: [signature] };
        }
      }

      async closeOrder(args: {
        closer: Wallet;
        order: web3.PublicKey;
        globalConfig?: web3.PublicKey;
      }): Promise<{
        signature: string;
      }> {
        const globalConfig = args.globalConfig ?? this.globalConfig;
        const { pdaAuthority } = await this.getPdaAuthority(globalConfig);

        const orderAccount = await this.getOrderAccount(args.order);
        const inputMint = orderAccount.inputMint;
        const outputMint = orderAccount.outputMint;
        const orderType = orderAccount.orderType;
        const tpChildOrder = orderAccount.tpChildOrder;
        const slChildOrder = orderAccount.slChildOrder;
        const maker = orderAccount.maker;

        // Get vaults
        const { vault: inputVault } = await this.getVault(inputMint, globalConfig);
        const { vault: outputVault } = await this.getVault(outputMint, globalConfig);

        // Get token programs
        const inputMintInfo = await this.provider.connection.getAccountInfo(inputMint);
        const outputMintInfo = await this.provider.connection.getAccountInfo(outputMint);
        const inputTokenProgram = inputMintInfo?.owner;
        const outputTokenProgram = outputMintInfo?.owner;

        // Get maker ATAs
        const makerInputAta = spl.getAssociatedTokenAddressSync(
          inputMint,
          maker,
          false,
          inputTokenProgram,
        );
        const makerOutputAta = spl.getAssociatedTokenAddressSync(
          outputMint,
          maker,
          false,
          outputTokenProgram,
        );
        const closerInputAta = spl.getAssociatedTokenAddressSync(
          inputMint,
          args.closer.publicKey,
          false,
          inputTokenProgram,
        );
        const closerOutputAta = spl.getAssociatedTokenAddressSync(
          outputMint,
          args.closer.publicKey,
          false,
          outputTokenProgram,
        );

        // Determine if child orders are needed (LimitParent = 1)
        const tpChildOrderPubkey = orderType === 1 && tpChildOrder.toBase58() !== web3.PublicKey.default.toBase58() 
          ? tpChildOrder 
          : null;
        const slChildOrderPubkey = orderType === 1 && slChildOrder.toBase58() !== web3.PublicKey.default.toBase58() 
          ? slChildOrder 
          : null;

        const ix = await this.program.methods.closeOrderAndClaimTip()
          .accounts({
            closer: args.closer.publicKey,
            maker: maker,
            order: args.order,
            tpChildOrder: tpChildOrderPubkey,
            slChildOrder: slChildOrderPubkey,
            globalConfig: globalConfig,
            pdaAuthority: pdaAuthority,
            inputMint: inputMint,
            outputMint: outputMint,
            makerInputAta: makerInputAta,
            makerOutputAta: makerOutputAta,
            closerInputAta: closerInputAta,
            closerOutputAta: closerOutputAta,
            inputVault: inputVault,
            outputVault: outputVault,
            inputTokenProgram: inputTokenProgram,
            outputTokenProgram: outputTokenProgram,
            systemProgram: web3.SystemProgram.programId,
          })
          .instruction();

        const { signature } = await this.sendTransaction(args.closer, [ix]);
        return { signature };
      }

      async calcOrderMinOutputAmount(inputAmount: BN, order: web3.PublicKey): Promise<BN> {
        const orderAccount = await this.getOrderAccount(order);
        const numerator = new BN(inputAmount).mul(orderAccount.expectedOutputAmount);
        const denominator = orderAccount.initialInputAmount;
        return numerator.add(denominator).sub(new BN(1)).div(denominator);
    }
}