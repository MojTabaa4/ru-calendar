import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Listens to an app event; returns a function that stops listening. No-op outside Tauri. */
export function onAppEvent<T>(name: string, handler: (payload: T) => void): () => void {
  if (!isTauri()) {
    return () => {};
  }
  let stop: (() => void) | undefined;
  let stopped = false;
  void listen<T>(name, (event) => handler(event.payload)).then((unlisten) => {
    if (stopped) {
      unlisten();
    } else {
      stop = unlisten;
    }
  });
  return () => {
    stopped = true;
    stop?.();
  };
}
