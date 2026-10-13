import { debug, info, error as logError, warn } from "@tauri-apps/plugin-log";

export type Fields = Readonly<Record<string, unknown>>;

type Logger = {
  readonly debug: (message: string, fields?: Fields) => void;
  readonly info: (message: string, fields?: Fields) => void;
  readonly warn: (message: string, fields?: Fields) => void;
  readonly error: (message: string, fields?: Fields) => void;
  readonly withContext: (context: Fields) => Logger;
};

function render(message: string, context: Fields, fields: Fields | undefined): string {
  const merged = { ...context, ...fields };

  if (Object.keys(merged).length === 0) {
    return message;
  }

  return `${message} ${JSON.stringify(merged, replacer)}`;
}

function replacer(_key: string, value: unknown): unknown {
  if (value instanceof Error) {
    return { name: value.name, message: value.message };
  }

  return value;
}

function forward(task: Promise<void>): void {
  task.catch((rejection: unknown) => {
    console.error("The log sink rejected.", rejection);
  });
}

function create(context: Fields): Logger {
  return {
    debug: (message, fields) => forward(debug(render(message, context, fields))),
    info: (message, fields) => forward(info(render(message, context, fields))),
    warn: (message, fields) => forward(warn(render(message, context, fields))),
    error: (message, fields) => forward(logError(render(message, context, fields))),
    withContext: (extra) => create({ ...context, ...extra }),
  };
}

const root = create({});

export const debugLog = root.debug;
export const infoLog = root.info;
export const warnLog = root.warn;
export const errorLog = root.error;
export const withContext = root.withContext;
export { root as logger };
