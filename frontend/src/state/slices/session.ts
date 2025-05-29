import * as reduxjs_toolkit from '@reduxjs/toolkit'
import type * as ffi from '../../ffi';

/**
 * The initial state of the program.
 */
type Initializing = {
  _tag: "Initializing",
  TODO: "TODO",
};

/**
 * Backend is reachable, and per status query response the client is considered
 * unauthorized.
 */
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

/**
 * Client is authorized and has a healthy WebSocket connection established to
 * the backend.
 */
type AuthorizedWebSocketConnected = {
  _tag: "AuthorizedWebSocketConnected",

  /**
   * Snapshot of full remote state expected to be received and re-rendered on
   * a regular interval.
   */
  remote_state_snapshot_full: ffi.StateSnapshotFull,

  /**
   * Timestamp of when the client received the state snapshot over a WebSocket.
   *
   * Datetime string in ISO format. For example: `"2025-05-29T13:34:19.478Z"`.
   */
  received_at_client_time: string,
};

type SessionDisconnected = {
  _tag: "SessionDisconnected",

  /**
   * How the session was disconnected, i.e. the WebSocket connection.
   */
  websocket_close: {
    /**
     * Timestamp of when the client WebSocket emitted close event was picked up.
     *
     * Datetime string in ISO format. For example: `"2025-05-29T13:34:19.478Z"`.
     */
    closed_at_client_time: string,

    /**
     * WebSocket standard thing.
     */
    was_clean: boolean,

    /**
     * WebSocket standard thing.
     */
    code: number,
  }
}

/**
 * Something is not right (and cannot or will not be automatically corrected).
 */
type ErrSession = {
  _tag: "ErrSession",
  error_chain: Array<{ name: string, message: string, stack: string }>,
};
export type State = Initializing | Unauthorized | AuthorizedWebSocketConnected | SessionDisconnected | ErrSession;

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
      action: reduxjs_toolkit.PayloadAction<{
        received_at_client_time: string,
        remote_state_snapshot_full: ffi.StateSnapshotFull,
      }>,
    ) => {
      const updated: AuthorizedWebSocketConnected = {
        _tag: "AuthorizedWebSocketConnected",
        received_at_client_time: action.payload.received_at_client_time,
        remote_state_snapshot_full: action.payload.remote_state_snapshot_full,
      };
      return updated;
    },
    set_error: (
      _state: State,
      action: reduxjs_toolkit.PayloadAction<{
        error_chain: Array<{ name: string, message: string, stack: string }>,
      }>,
    ) => {
      const updated: ErrSession = {
        _tag: "ErrSession",
        error_chain: action.payload.error_chain,
      };
      return updated;
    },
    set_session_disconnected: (
      _state: State,
      action: reduxjs_toolkit.PayloadAction<{
        websocket_close: {
          closed_at_client_time: string,
          was_clean: boolean,
          code: number,
        }
      }>,
    ) => {
      const updated: SessionDisconnected = {
        _tag: "SessionDisconnected",
        websocket_close: {
          closed_at_client_time: action.payload.websocket_close.closed_at_client_time,
          was_clean: action.payload.websocket_close.was_clean,
          code: action.payload.websocket_close.code,
        }
      };
      return updated;
    },
  },
});

export default slice;
