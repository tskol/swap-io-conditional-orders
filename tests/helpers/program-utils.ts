import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { Idl, web3 } from "@coral-xyz/anchor";
import { TransactionSender } from "./transaction-sender";

export type EventName<T extends Idl> = keyof anchor.IdlEvents<T>;
export type EventFields<
  T extends Idl,
  U extends EventName<T>,
> = anchor.IdlEvents<T>[U];

export class ProgramUtils<I extends Idl> extends TransactionSender {
  public readonly program: anchor.Program<I>;
  private readonly tulipConfigEventFilter: web3.PublicKey;

  constructor(
    connection: web3.Connection,
    program: anchor.Program<I>,
    tulipConfig: web3.PublicKey,
  ) {
    super(connection);
    this.program = program;
    this.tulipConfigEventFilter = tulipConfig;
  }

  protected nativeMint(tokenProgram: web3.PublicKey): web3.PublicKey {
    if (tokenProgram.equals(spl.TOKEN_PROGRAM_ID)) {
      return spl.NATIVE_MINT;
    } else if (tokenProgram.equals(spl.TOKEN_2022_PROGRAM_ID)) {
      return spl.NATIVE_MINT_2022;
    } else {
      throw new Error("Unsupported token program");
    }
  }

  protected async ownerOf(account: web3.PublicKey): Promise<web3.PublicKey> {
    const accountInfo = await this.connection.getAccountInfo(account);
    if (!accountInfo) {
      throw new Error(`Account ${account.toBase58()} not found`);
    }
    return accountInfo.owner;
  }

  // async extractEvent<T extends EventName<I>, R>(
  //   signature: string,
  //   eventName: T,
  //   callback: (event: anchor.EventData<anchor.IdlEventFields, Record<string, never>>) => R,
  //   eventIndex = 0,
  // ): Promise<R> {
  //   const events = await this.getAllEvents(signature);
  //   const indicesMap = new Map<string, number>();
  //   for (const event of events) {
  //     if (!indicesMap.has(event.name)) {
  //       indicesMap.set(event.name, 0);
  //     }

  //     const currentIndex = indicesMap.get(event.name);
  //     if (event.name === eventName && currentIndex === eventIndex) {
  //       return callback(event.data);
  //     }
  //     indicesMap.set(event.name, currentIndex + 1);
  //   }

  //   throw new Error(`Event "${String(eventName)}" not found`);
  // }

  async getAllEvents(signature: string): Promise<anchor.Event[]> {
    const txConfig = {
      maxSupportedTransactionVersion: 0,
    };
    const transaction = await this.connection.getTransaction(
      signature,
      txConfig,
    );
    const programIdIndex =
      transaction.transaction.message.staticAccountKeys.findIndex((p) =>
        p.equals(this.program.programId),
      );

    const events: anchor.Event[] = [];
    for (const innerIx of transaction.meta.innerInstructions) {
      for (const ix of innerIx.instructions) {
        if (programIdIndex !== ix.programIdIndex) {
          continue;
        }

        const rawData = anchor.utils.bytes.bs58.decode(ix.data);
        const base64Data = anchor.utils.bytes.base64.encode(
          rawData.subarray(8),
        );
        const event = this.program.coder.events.decode(base64Data);
        if (event != null) {
          const data = event.data as {
            config?: web3.PublicKey;
            tulipConfig?: web3.PublicKey;
          };

          // Only listen to events for this config instance.
          // Ignore events with other configs.
          if (
            data.config?.equals(this.tulipConfigEventFilter) ||
            data.tulipConfig?.equals(this.tulipConfigEventFilter)
          ) {
            events.push(event);
          }
        }
      }
    }

    return events;
  }
}
