import * as libreact from "react";
import * as librredux from "react-redux";
import * as libstore from "../store/_mod";
import type * as libredux from "redux";
import type * as libstatus from "../store/status";
import type * as libsync from "../store/sync";

let SOCKET: null | WebSocket;

type Props = { loggedIn: libstatus.LoggedIn, urlWs: URL };

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
    console.debug("TODO: Connect WebSocket");
    SOCKET = new WebSocket(urlWs);
    SOCKET.addEventListener("message", function handleMessage(event) {
      console.debug("TODO: Store in Redux", event);
    });
    return disconnectWebSocket;
  }
}

function disconnectWebSocket() {
  if (SOCKET) {
    SOCKET.close();
  }
  console.debug("TODO: Disconnect WebSocket");
}
