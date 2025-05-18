import * as librtk from "@reduxjs/toolkit";
import * as libstatus from "./status";
import * as libsync from "./sync";

const sync = librtk.createSlice({
  name: libsync.reduxSliceName,
  initialState: libsync.reduxSliceInitialState,
  reducers: {
    setState: (_state, action) => {
      return action.payload;
    },
  },
});

const status = librtk.createSlice({
  name: libstatus.reduxSliceName,
  initialState: libstatus.reduxSliceInitialState,
  reducers: {
    setLoggedIn: (_state, action: { payload: { sessionId: libsync.Uuid } }) => {
      return { sessionId: action.payload.sessionId } satisfies libstatus.LoggedIn;
    },
    setLoggedOut: (_state) => {
      return libstatus.PreLogin.LoggedOut;
    },
    setOffline: (_state) => {
      return libstatus.PreLogin.Offline;
    },
  },
});

export const reducers = {
  status: status.reducer,
  sync: sync.reducer,
};
export type Reducers = typeof reducers;

export const actions = {
  status: status.actions,
  sync: sync.actions,
};

export function init(reducers: Reducers) {
  const store = librtk.configureStore({ reducer: reducers });
  return store;
}

export type Store = ReturnType<typeof init>;
export type RootState = ReturnType<Store["getState"]>;
