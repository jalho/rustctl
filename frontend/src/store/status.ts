import type * as libsync from "./sync";

export const reduxSliceName = "status" as const;

/** Redux slice state. */
export type State = PreLogin | LoggedIn;

export enum PreLogin {
  Initializing = "Initializing",
  LoggedOut = "LoggedOut",
  Offline = "Offline",
};

export type LoggedIn = {
  sessionId: libsync.Uuid;
}

export const reduxSliceInitialState: State = PreLogin.Initializing;
