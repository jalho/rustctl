import * as libreact from "react";
import * as libredux from "react-redux";
import * as libstatus from "../store/status";
import * as libstore from "../store/_mod";
import * as libutil from "../util";
import type * as libsync from "../store/sync";

const StatusCheck = (props: { urlCheckStatus: string, urlLogIn: string }): null | libreact.ReactElement => {
  const state: libstatus.State = libredux.useSelector<libstore.RootState, libstatus.State>((s) => {
    return s.status;
  });
  const dispatch = libredux.useDispatch();

  switch (state) {

    case libstatus.PreLogin.Initializing: {
      const sessionId: null | libsync.Uuid = libutil.getSessionId(document.cookie);

      if (sessionId) {
        checkStatus(props.urlCheckStatus).then((status) => {

          if (status === SessionStatus.Offline) {
            dispatch(libstore.actions.status.setOffline());
          }

          else if (status === SessionStatus.OnlineSessionInvalid) {
            dispatch(libstore.actions.status.setLoggedOut());
          }

          else if (status === SessionStatus.OnlineSessionValid) {
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

      return <>Initializing...</>;
    }

    case libstatus.PreLogin.LoggedOut: {
      logIn(props.urlLogIn).then((result) => {
        if (result === SessionStatus.Offline) {
          dispatch(libstore.actions.status.setOffline());
        } else {
          dispatch(libstore.actions.status.setLoggedIn({ sessionId: result.sessionId }));
        }
      });

      return <>Logging in...</>;
    }

    case libstatus.PreLogin.Offline: {
      return <>Offline</>;
    }

    default: {
      return <>TODO: Connect WebSocket...</>;
    }

  }
};

enum SessionStatus {
  Offline = "Offline",
  OnlineSessionValid = "OnlineSessionValid",
  OnlineSessionInvalid = "OnlineSessionInvalid",
}

async function checkStatus(url: string): Promise<SessionStatus> {
  try {
    const response = await fetch(url);
    if (response.ok) {
      return SessionStatus.OnlineSessionValid;
    } else {
      return SessionStatus.OnlineSessionInvalid;
    }
  } catch (_) {
    return SessionStatus.Offline;
  }
}

async function logIn(url: string): Promise<{ sessionId: libsync.Uuid } | SessionStatus.Offline> {
  let response: Response;

  try {
    response = await fetch(url);
  } catch (_) {
    return SessionStatus.Offline;
  }

  if (response.ok) {
    const sessionId: null | libsync.Uuid = libutil.getSessionId(document.cookie);
    if (sessionId) {
      return { sessionId };
    } else {
      throw new Error("BUG: login response OK but no cookie set");
    }
  } else {
    throw new Error("BUG: login rejected by remote with status " + response.status);
  }
}

export default StatusCheck;
