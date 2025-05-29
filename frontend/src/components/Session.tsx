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

const ErrSession = (
  props: {
    error_chain: Array<{ name: string; message: string; stack: string }>;
  }
): react.JSX.Element => {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
      {props.error_chain.map(
        (
          error: { name: string; message: string; stack: string },
          index: number
        ): react.JSX.Element => {
          return (
            <div
              key={index}
              style={{
                border: "1px solid #fca5a5",
                backgroundColor: "#fef2f2",
                padding: "12px",
                borderRadius: "12px",
                color: "#7f1d1d",
                fontSize: "14px",
                fontFamily: "monospace"
              }}
            >
              <div style={{ fontWeight: "bold", marginBottom: "4px" }}>
                {error.name}
              </div>
              <div style={{ marginBottom: "8px" }}>{error.message}</div>
              <pre
                style={{
                  whiteSpace: "pre-wrap",
                  overflowX: "auto",
                  fontSize: "12px",
                  color: "#991b1b",
                  margin: 0
                }}
              >
                {error.stack}
              </pre>
            </div>
          );
        }
      )}
    </div>
  );
};

const Initializing = (): react.JSX.Element => {
  return (
    <>
      Initializing
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
