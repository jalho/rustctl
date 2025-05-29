import * as react_redux from "react-redux";
import ConnectionManager, { collect_causes_enumerable, FetchAPIError } from "../ConnectionManager";
import SessionSlice from "../state/slices/session";
import Store from "../state/store";
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
        received_at_client_time={state.received_at_client_time}
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

  else if (state._tag === "SessionDisconnected") {
    return (
      <SessionDisconnected
        closed_at_client_time={state.websocket_close.closed_at_client_time}
        was_clean={state.websocket_close.was_clean}
        code={state.websocket_close.code}
      />
    );
  }

  else {
    throw new Error("unreachable");
  }
}

const SessionDisconnected = (props: {
  closed_at_client_time: string,
  was_clean: boolean,
  code: number,
}): react.JSX.Element => {
  return (
    <div style={{
      padding: "16px",
      borderRadius: "12px",
      boxShadow: "0 2px 8px rgba(0, 0, 0, 0.1)",
      backgroundColor: "#fef2f2",
      border: "1px solid #fca5a5",
      maxWidth: "400px",
      margin: "32px auto",
      fontFamily: "sans-serif"
    }}>
      <h2 style={{
        fontSize: "18px",
        fontWeight: "600",
        color: "#b91c1c",
        marginBottom: "8px"
      }}>
        WebSocket Disconnected
      </h2>
      <div style={{ fontSize: "14px", color: "#7f1d1d", lineHeight: "1.5" }}>
        <div>
          <span style={{ fontWeight: "500" }}>Time:</span> {props.closed_at_client_time}
        </div>
        <div>
          <span style={{ fontWeight: "500" }}>Closed Cleanly:</span> {props.was_clean ? "Yes" : "No"}
        </div>
        <div>
          <span style={{ fontWeight: "500" }}>Code:</span> {props.code}
        </div>
      </div>
    </div>
  );
};

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
    <div style={{ fontFamily: "sans-serif", lineHeight: "1.5", padding: "1rem" }}>
      <div style={{ fontWeight: "bold", marginBottom: "0.5rem" }}>
        Unauthorized:
      </div>
      <div style={{ fontFamily: "monospace", marginBottom: "1rem" }}>
        at {props.checked_at_client_time}: HTTP status {props.rejection_http_status_code}
      </div>
      <button
        style={{
          padding: "0.5rem 1rem",
          fontSize: "1rem",
          fontWeight: "bold",
          border: "1px solid #ccc",
          borderRadius: "4px",
          backgroundColor: "#f5f5f5",
          cursor: "pointer",
        }}
        onClick={async (): Promise<void> => {
          let response: Response;
          try {
            response = await fetch(ConnectionManager.URL_LOGIN);
          } catch (err: unknown) {
            const web_api_err: DOMException = err as DOMException;
            const fetch_api_err: FetchAPIError = new FetchAPIError(web_api_err);
            Store.dispatch(SessionSlice.actions.set_error({
              error_chain: collect_causes_enumerable(fetch_api_err),
            }));
            return;
          }

          if (!response.ok) {
            const err: Error = new Error("login failed: unexpected status " + response.status);
            Store.dispatch(SessionSlice.actions.set_error({
              error_chain: collect_causes_enumerable(err),
            }));
            return;
          } else {
            ConnectionManager.restart();
          }
        }}
      >
        Log in
      </button>
    </div>
  );
};

const AuthorizedWebSocketConnected = (
  props: {
    received_at_client_time: string,
    remote_state_snapshot_full: ffi.StateSnapshotFull,
  }
): react.JSX.Element => {
  return (
    <>
      <p>
        AuthorizedWebSocketConnected
      </p>
      <p>
        State update received at: {props.received_at_client_time}
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
