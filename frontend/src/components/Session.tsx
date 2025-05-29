import * as react_redux from "react-redux";
import slice, { State as SessionState } from "../state/slices/session";
import type * as react from "react";
import type * as store from "../state/store";

const Session = (): react.JSX.Element => {
  const state: SessionState = react_redux.useSelector<store.RootState, SessionState>((state) => state.session);

  if (state._tag === "Initializing") {
    return <Initializing />;
  } else if (state._tag === "Offline") {
    return <Offline />;
  } else if (state._tag === "Unauthorized") {
    return <Unauthorized />;
  } else if (state._tag === "AuthorizedWebSocketConnected") {
    return <AuthorizedWebSocketConnected />;
  } else {
    throw new Error("unreachable");
  }
}

const Initializing = (): react.JSX.Element => {
  return (
    <>
      Initializing
    </>
  );
}

const Offline = (): react.JSX.Element => {
  return (
    <>
      Offline
    </>
  );
}

const Unauthorized = (): react.JSX.Element => {
  return (
    <>
      Unauthorized
    </>
  );
}

const AuthorizedWebSocketConnected = (): react.JSX.Element => {
  return (
    <>
      AuthorizedWebSocketConnected
    </>
  );
}

export default Session;
