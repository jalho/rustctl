import * as reduxjs_toolkit from '@reduxjs/toolkit'

type Initializing = {
  _tag: "Initializing",
  content: {
    TODO: "TODO",
  }
};
type Offline = {
  _tag: "Offline",
  content: {
    TODO: "TODO",
  }
};
type Unauthorized = {
  _tag: "Unauthorized",
  content: {
    TODO: "TODO",
  }
};
type AuthorizedWebSocketConnected = {
  _tag: "AuthorizedWebSocketConnected",
  content: {
    session_id: string,
  }
};
export type State = Initializing | Offline | Unauthorized | AuthorizedWebSocketConnected;

const initial_state: State = {
  _tag: "Initializing",
  content: {
    TODO: "TODO",
  }
} satisfies Initializing;

function set_initializing(state: State): void {
  state = {
    _tag: "Initializing",
    content: {
      TODO: "TODO",
    }
  } satisfies Initializing;
}

function set_offline(state: State): void {
  state = {
    _tag: "Offline",
    content: {
      TODO: "TODO",
    }
  } satisfies Offline;
}

function set_unauthorized(state: State): void {
  state = {
    _tag: "Unauthorized",
    content: {
      TODO: "TODO",
    }
  } satisfies Unauthorized;
}

function set_authorized_websocket_connected(state: State, action: { payload: { session_id: string } }): void {
  state = {
    _tag: "AuthorizedWebSocketConnected",
    content: {
      session_id: action.payload.session_id,
    }
  } satisfies AuthorizedWebSocketConnected;
}

const slice = reduxjs_toolkit.createSlice({
  name: "session" as const,
  initialState: initial_state,
  reducers: {
    set_initializing,
    set_offline,
    set_unauthorized,
    set_authorized_websocket_connected,
  },
});

export default slice;
