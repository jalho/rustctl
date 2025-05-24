import * as libredux from "react-redux";
import * as libstore from "../store/_mod";
import type * as libreact from "react";
import type * as libstatus from "../store/status";
import type * as libsync from "../store/sync";

type Props = { loggedIn: libstatus.LoggedIn, urlWs: URL };

const ConnectWebSocket = (props: Props): libreact.ReactElement => {
  const state: libsync.State = libredux.useSelector<libstore.RootState, libsync.State>((s) => {
    return s.sync;
  });
  const dispatch = libredux.useDispatch();
  /*
   *  dispatch(libstore.actions.status.setOffline());
   */

  if (state === null) {
    const ws = new WebSocket(props.urlWs);
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
