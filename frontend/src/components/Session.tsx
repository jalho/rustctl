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

  else if (state._tag === "Unauthorized") {
    return (
      <Unauthorized
        checked_at_client_time={state.checked_at_client_time}
        rejection_http_status_code={state.rejection_http_status_code}
      />
    );
  }

  else if (state._tag === "AuthorizedWebSocketConnected") {
    return (
      <AuthorizedWebSocketConnected
        websocket_connection_id={state.websocket_connection_id}
        remote_state_snapshot_full={state.remote_state_snapshot_full}
      />
    );
  }

  else if (state._tag === "ErrSession") {
    return (
      <ErrSession
        error_chain={state.error_chain}
      />
    );
  }

  else {
    throw new Error("unreachable");
  }
}

const ErrSession = (props: { error_chain: Array<{ name: string, message: string, stack: string }> }): react.JSX.Element => {
  return (
    <>
      <code>
        {JSON.stringify(props.error_chain, null, 2)}
      </code>
    </>
  );
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

const Unauthorized = (props: {
  checked_at_client_time: string,
  rejection_http_status_code: number,
}): react.JSX.Element => {
  return (
    <>
      <b>Unauthorized: </b>
      <code>at {props.checked_at_client_time}: HTTP status {props.rejection_http_status_code}</code>
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
