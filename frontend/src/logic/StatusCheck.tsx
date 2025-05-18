import * as libreact from "react";
import * as libredux from "react-redux";
import * as libstatus from "../store/status";
import * as libstore from "../store/_mod";
import * as libutil from "../util";
import type * as libsync from "../store/sync";

const StatusCheck = (props: { url: string }): null | libreact.ReactElement => {
  const state: libstatus.State = libredux.useSelector<libstore.RootState, libstatus.State>((s) => {
    return s.status;
  });
  const dispatch = libredux.useDispatch();

  switch (state) {

    case libstatus.PreLogin.Initializing: {
      const sessionId: null | libsync.Uuid = libutil.getSessionId(document.cookie);

      if (sessionId) {
        checkStatus(props.url).then((status) => {

          if (status === Status.Offline) {
            dispatch(libstore.actions.status.setOffline());
          }

          else if (status === Status.OnlineSessionInvalid) {
            dispatch(libstore.actions.status.setLoggedOut());
          }

          else if (status === Status.OnlineSessionValid) {
            dispatch(libstore.actions.status.setLoggedIn({ sessionId }));
          }

          else {
            throw new Error("unhandled case: " + status);
          }

        });
      }

      else {
        dispatch(libstore.actions.status.setLoggedOut());
      }

      return <>Initializing</>;
    }

    case libstatus.PreLogin.LoggedOut: {
      return <>LoggedOut</>;
    }

    case libstatus.PreLogin.Offline: {
      return <>Offline</>;
    }

    default: {
      return <>TODO: Connect WebSocket...</>;
    }

  }
};

enum Status {
  Offline,
  OnlineSessionValid,
  OnlineSessionInvalid,
}

async function checkStatus(url: string): Promise<Status> {
  try {
    const response = await fetch(url);
    if (response.ok) {
      return Status.OnlineSessionValid;
    } else {
      return Status.OnlineSessionInvalid;
    }
  } catch (err) {
    console.debug(err);
    return Status.Offline;
  }
}

export default StatusCheck;
