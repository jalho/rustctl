import * as libreact from "react";
import * as librredux from "react-redux";
import * as libstore from "../store/_mod";
import type * as libredux from "redux";
import * as libstatus from "../store/status";
import type * as libsync from "../store/sync";

type Callback = (args: any) => any;

let SOCKET: WebSocket;
let SOCKET_EVENT_LISTENER_MESSAGE: Callback;
let SOCKET_EVENT_LISTENER_CLOSE: Callback;
let SOCKET_EVENT_LISTENER_ERROR: Callback;

type Props = { loggedIn: libstatus.LoggedIn, urlWs: URL };

/**
 * On mount, connect WebSocket and register Redux state updating event handlers
 * for the socket. On unmount, undo both: Unregister the event handlers and
 * disconnect the socket.
 */
const ConnectWebSocket = (props: Props): libreact.ReactElement => {
  const state: libsync.State = librredux.useSelector<libstore.RootState, libsync.State>((s) => {
    return s.sync;
  });
  const dispatch = librredux.useDispatch();

  libreact.useEffect(
    connectWebSocket(dispatch, props.urlWs),
    [/* on mount, connect */],
  );

  if (state === null) {
    return (
      <>
        Session ID: {props.loggedIn.sessionId} -- Connecting WebSocket...
      </>
    );
  } else {
    const stateUpdatePayload: libsync.WebSocketStateUpdatePayload = state;
    return (
      <code>
        {JSON.stringify(stateUpdatePayload, null, 2)}
      </code>
    );
  }
};

export default ConnectWebSocket;

function connectWebSocket(
  dispatch: libreact.Dispatch<libredux.UnknownAction>,
  urlWs: URL,
) {
  return function() {
    SOCKET = new WebSocket(urlWs);

    SOCKET_EVENT_LISTENER_MESSAGE = function handleMessage(event: { data: string }) {
      const payload: libsync.WebSocketStateUpdatePayload = JSON.parse(event.data);
      dispatch(libstore.actions.sync.setSyncState(payload));
    };
    SOCKET_EVENT_LISTENER_CLOSE = function handleClose() {
      dispatch(libstore.actions.sync.setSyncReset());
      dispatch(libstore.actions.status.setOffline());
    };
    SOCKET_EVENT_LISTENER_ERROR = function handleError() {
      dispatch(libstore.actions.sync.setSyncReset());
      dispatch(libstore.actions.status.setOffline());
    };

    SOCKET.addEventListener("message", SOCKET_EVENT_LISTENER_MESSAGE);
    SOCKET.addEventListener("close", SOCKET_EVENT_LISTENER_CLOSE);
    SOCKET.addEventListener("error", SOCKET_EVENT_LISTENER_ERROR);

    // on unmount, disconnect
    return disconnectWebSocket(dispatch);
  }
}

function disconnectWebSocket(
  dispatch: libreact.Dispatch<libredux.UnknownAction>,
) {
  return function() {
    if (SOCKET) {
      SOCKET.removeEventListener("message", SOCKET_EVENT_LISTENER_MESSAGE);
      SOCKET.removeEventListener("close", SOCKET_EVENT_LISTENER_CLOSE);
      SOCKET.removeEventListener("error", SOCKET_EVENT_LISTENER_ERROR);
      SOCKET.close();
      dispatch(libstore.actions.sync.setSyncReset());
    }
  }
}
