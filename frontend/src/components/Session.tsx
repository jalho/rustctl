import * as react_redux from "react-redux";
import type * as ffi from "../ffi";
import type * as react from "react";
import type * as session from "../state/slices/session";
import type * as store from "../state/store";

const Session = (): react.JSX.Element => {
  const state: session.State = react_redux.useSelector<store.RootState, session.State>((state) => state.session);

  if (state._tag === "Initializing") {
    return <Initializing />;
  }

  else if (state._tag === "Offline") {
    return <Offline />;
  }

  else if (state._tag === "Unauthorized") {
    return <Unauthorized />;
  }

  else if (state._tag === "AuthorizedWebSocketConnected") {
    return (
      <AuthorizedWebSocketConnected
        websocket_connection_id={state.content.websocket_connection_id}
        remote_state_snapshot_full={state.content.remote_state_snapshot_full}
      />
    );
  }

  else {
    throw new Error("unreachable");
  }
}

const Initializing = (): react.JSX.Element => {
  return (
    <>
      Initializing
    </>
  );
}

const Offline = (): react.JSX.Element => {
  return (
    <>
      Offline
    </>
  );
}

const Unauthorized = (): react.JSX.Element => {
  return (
    <>
      Unauthorized
    </>
  );
}

const AuthorizedWebSocketConnected = (
  props: {
    websocket_connection_id: string,
    remote_state_snapshot_full: ffi.StateSnapshotFull,
  }
): react.JSX.Element => {
  return (
    <>
      <p>
        AuthorizedWebSocketConnected
      </p>
      <p>
        WebSocket connection ID: {props.websocket_connection_id}
      </p>
      <p>
        Remote state snapshot:
        <code>
          {JSON.stringify(props.remote_state_snapshot_full, null, 2)}
        </code>
      </p>
    </>
  );
}

export default Session;
