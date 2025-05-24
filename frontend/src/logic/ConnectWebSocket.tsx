import type * as libreact from "react";
import type * as libstatus from "../store/status";

const ConnectWebSocket = (props: { loggedIn: libstatus.LoggedIn }): libreact.ReactElement => {
  return (
    <div>
      <p>TODO: Connect WebSocket...</p>
      <p>Logged in session ID: {props.loggedIn.sessionId}</p>
    </div>
  );
};

export default ConnectWebSocket;
