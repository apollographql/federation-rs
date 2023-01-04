declare namespace Deno {
  namespace core {
    function opAsync(opName: string, ...args: any[]): Promise<any>;
    const ops: Record<string, (...args: unknown[]) => any>;
    function initializeAsyncOps(): void;
  }
}

// No async function can be called if this hasn't been done.
// It needs to be done at runtime and cannot be snapshotted.
Deno.core.initializeAsyncOps();

let logFunction: (message: string) => void;
declare let logger: {
  trace: typeof logFunction;
  debug: typeof logFunction;
  info: typeof logFunction;
  warn: typeof logFunction;
  error: typeof logFunction;
};

enum CommandKind {
  Trace = "Trace",
  Debug = "Debug",
  Info = "Info",
  Warn = "Warn",
  Error = "Error",
  Exit = "Exit",
}

type Payload = {
  kind: CommandKind;
  message?: string;
};

type Command = {
  id: string;
  payload: Payload;
};

type CommandResult = {
  id: string;
  payload: boolean;
};

const send = async (result: CommandResult): Promise<void> => {
  await Deno.core.opAsync("send", result);
};
const receive = async (): Promise<Command> =>
  await Deno.core.opAsync("receive");

async function run() {
  while (true) {
    try {
      const event = await receive();
      const {
        id,
        payload: { kind, message },
      } = event;
      switch (kind) {
        case CommandKind.Trace:
          logger.trace(message);
          await send({ id, payload: true });
          break;
        case CommandKind.Debug:
          logger.debug(message);
          await send({ id, payload: true });
          break;
        case CommandKind.Info:
          logger.info(message);
          await send({ id, payload: true });
          break;
        case CommandKind.Warn:
          logger.warn(message);
          await send({ id, payload: true });
          break;
        case CommandKind.Error:
          logger.error(message);
          await send({ id, payload: true });
          break;
        case CommandKind.Exit:
          await send({ id, payload: true });
          return;
        default:
          logger.error(`unknown message received: ${JSON.stringify(event)}\n`);
          break;
      }
    } catch (e) {
      logger.error(`an unknown error occured ${e}\n`);
    }
  }
}

run();
