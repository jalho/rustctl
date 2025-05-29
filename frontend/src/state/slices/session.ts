import * as reduxjs_toolkit from '@reduxjs/toolkit'
import type * as ffi from '../../ffi';

type Initializing = {
  _tag: "Initializing",
  TODO: "TODO",
};
type Offline = {
  _tag: "Offline",
  TODO: "TODO",
};
type Unauthorized = {
  _tag: "Unauthorized",
  TODO: "TODO",
};
type AuthorizedWebSocketConnected = {
  _tag: "AuthorizedWebSocketConnected",

  /**
   * Backend assigned UUID associated with WebSocket connection.
   */
  websocket_connection_id: string,

  /**
   * Snapshot of full remote state expected to be received and re-rendered on
   * a regular interval.
   */
  remote_state_snapshot_full: ffi.StateSnapshotFull,
};
export type State = Initializing | Offline | Unauthorized | AuthorizedWebSocketConnected;

const initial_state: State = {
  _tag: "Initializing",
  TODO: "TODO",
} satisfies Initializing;

const slice = reduxjs_toolkit.createSlice({
  name: "session" as const,
  initialState: initial_state as State,
  reducers: {
    set_initializing: (_state: State) => {
      const updated: Initializing = {
        _tag: "Initializing",
        TODO: "TODO",
      };
      return updated;
    },
    set_offline: (_state: State) => {
      const updated: Offline = {
        _tag: "Offline",
        TODO: "TODO",
      };
      return updated;
    },
    set_unauthorized: (_state: State) => {
      const updated: Unauthorized = {
        _tag: "Unauthorized",
        TODO: "TODO",
      };
      return updated;
    },
    set_authorized_websocket_connected: (
      _state: State,
      action,
    ) => {
      const updated: AuthorizedWebSocketConnected = {
        _tag: "AuthorizedWebSocketConnected",
          websocket_connection_id: action.payload.websocket_connection_id,
          remote_state_snapshot_full: action.payload.remote_state_snapshot_full,
      };
      return updated;
    },
  },
});

export default slice;
