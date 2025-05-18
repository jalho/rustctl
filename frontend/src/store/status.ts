export const reduxSliceName = "status" as const;

/** Redux slice state. */
export enum State {
  Initializing = "Initializing",
  LoggedIn = "LoggedIn",
  LoggedOut = "LoggedOut",
  Offline = "Offline",
};

export const reduxSliceInitialState: State = State.Initializing;
