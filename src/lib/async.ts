import { errorLog } from "@/lib/log";

export function spawn(task: Promise<unknown>): void {
  task.catch((rejection: unknown) => {
    errorLog("A spawned task rejected.", { rejection });
  });
}

export type Cancel = () => void;

export function after(delayMs: number, run: () => void): Cancel {
  const handle = setTimeout(run, delayMs);

  return () => clearTimeout(handle);
}

export function debounced<A extends readonly unknown[]>(
  delayMs: number,
  run: (...args: A) => void,
): { readonly call: (...args: A) => void; readonly cancel: Cancel; readonly flush: () => void } {
  let pending: { readonly cancel: Cancel; readonly args: A } | undefined;

  return {
    call: (...args) => {
      pending?.cancel();
      const cancel = after(delayMs, () => {
        pending = undefined;
        run(...args);
      });
      pending = { cancel, args };
    },
    cancel: () => {
      pending?.cancel();
      pending = undefined;
    },
    flush: () => {
      if (pending === undefined) {
        return;
      }

      const { args } = pending;
      pending.cancel();
      pending = undefined;
      run(...args);
    },
  };
}
