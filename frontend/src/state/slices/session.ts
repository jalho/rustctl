import * as reduxjs_toolkit from '@reduxjs/toolkit'
import type * as ffi from '../../ffi';

type Initializing = {
  _tag: "Initializing",
  TODO: "TODO",
};
type Unauthorized = {
  _tag: "Unauthorized",

  /**
   * Timestamp of when the client received the response that indicates that it
   * is not authorized.
   *
   * Datetime string in ISO format. For example: `"2025-05-29T13:34:19.478Z"`.
   */
  checked_at_client_time: string,

  /**
   * HTTP status code of the response that indicated that the client is not
   * authorized.
   */
  rejection_http_status_code: number,
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
type ErrSession = {
  _tag: "ErrSession",
  name: string,
  message: string,
  stack: string,
  code: string,
};
export type State = Initializing | Unauthorized | AuthorizedWebSocketConnected | ErrSession;

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
    set_unauthorized: (_state: State, action: reduxjs_toolkit.PayloadAction<{
      checked_at_client_time: string,
      rejection_http_status_code: number,
    }>) => {
      const updated: Unauthorized = {
        _tag: "Unauthorized",
        checked_at_client_time: action.payload.checked_at_client_time,
        rejection_http_status_code: action.payload.rejection_http_status_code,
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
    set_error: (
      _state: State,
      action: reduxjs_toolkit.PayloadAction<{
        name: string,
        message: string,
        stack: string,
        code: string,
      }>,
    ) => {
      const updated: ErrSession = {
        _tag: "ErrSession",
        name: action.payload.name,
        message: action.payload.message,
        stack: action.payload.stack,
        code: action.payload.code,
      };
      return updated;
    }
  },
});

export default slice;
