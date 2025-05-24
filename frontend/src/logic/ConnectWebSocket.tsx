import * as libreact from "react";
import * as librredux from "react-redux";
import * as libstore from "../store/_mod";
import type * as libredux from "redux";
import type * as libstatus from "../store/status";
import type * as libsync from "../store/sync";

let SOCKET: null | WebSocket;
let SOCKET_EVENT_LISTENER: any;

type Props = { loggedIn: libstatus.LoggedIn, urlWs: URL };

/**
 * On mount, connect WebSocket and register a Redux state updating event handler
 * for the socket. On unmount, undo both: Unregister the event handler and
 * disconnect the socket.
 */
const ConnectWebSocket = (props: Props): libreact.ReactElement => {
  const state: libsync.State = librredux.useSelector<libstore.RootState, libsync.State>((s) => {
    return s.sync;
  });
  const dispatch = librredux.useDispatch();
  libreact.useEffect(connectWebSocket(dispatch, props.urlWs), [/* on mount */]);

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
    SOCKET_EVENT_LISTENER = function handleMessage(event: any) {
      const payload: libsync.WebSocketStateUpdatePayload = JSON.parse(event.data);
      dispatch(libstore.actions.sync.setState(payload));
    };
    SOCKET.addEventListener("message", SOCKET_EVENT_LISTENER);
    return disconnectWebSocket(dispatch);
  }
}

function disconnectWebSocket(
  dispatch: libreact.Dispatch<libredux.UnknownAction>,
) {
  return function() {
    if (SOCKET) {
      SOCKET.removeEventListener("message", SOCKET_EVENT_LISTENER);
      SOCKET.close();
      dispatch(libstore.actions.sync.setState(null));
    }
  }
}
