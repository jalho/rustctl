import * as libredux from "react-redux";
import * as libstatus from "../store/status";
import * as libstore from "../store/_mod";
import * as libutil from "../util";
import ConnectWebSocket from "./ConnectWebSocket";
import type * as libreact from "react";
import type * as libsync from "../store/sync";

const StatusCheck = (props: { urlCheckStatus: URL, urlLogIn: URL }): null | libreact.ReactElement => {
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
            throw new Error("BUG: status check: unhandled case: " + status);
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
      const loggedIn: libstatus.LoggedIn = state;
      return <ConnectWebSocket loggedIn={loggedIn} />;
    }

  }
};

enum SessionStatus {
  Offline = "Offline",
  OnlineSessionValid = "OnlineSessionValid",
  OnlineSessionInvalid = "OnlineSessionInvalid",
}

async function checkStatus(url: URL): Promise<SessionStatus> {
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

async function logIn(url: URL): Promise<{ sessionId: libsync.Uuid } | SessionStatus.Offline> {
  let response: Response;

  try {
    response = await fetch(url, { credentials: "include" });
  } catch (_) {
    return SessionStatus.Offline;
  }

  if (response.ok) {
    const sessionId: null | libsync.Uuid = await reread(5, 1000, () => libutil.getSessionId(document.cookie));
    if (sessionId) {
      return { sessionId };
    } else {
      throw new Error("BUG: login response OK but no cookie set");
    }
  } else {
    throw new Error("BUG: login rejected by remote with status " + response.status);
  }
}

// TODO: Remove reread? (Is the cookie immediately readable when response has arrived?)
async function reread<T>(attemptsMax: number, intervalMs: number, reader: () => T): Promise<null | T> {
  for (let i = 0; i < attemptsMax; i++) {
    const read = reader();
    if (read) {
      return read;
    } else {
      await sleepMs(intervalMs);
    }
  }
  return null;
}

function sleepMs(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}

export default StatusCheck;
