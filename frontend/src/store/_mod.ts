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
    setLoggedIn: (_state, _action) => {
      return libstatus.State.LoggedIn;
    },
    setLoggedOut: (_state, _action) => {
      return libstatus.State.LoggedOut;
    },
    setOffline: (_state, _action) => {
      return libstatus.State.Offline;
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
