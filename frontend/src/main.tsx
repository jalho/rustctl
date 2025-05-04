import * as ReactDOM from "react-dom/client";
import { configureStore } from "@reduxjs/toolkit";
import { createSlice } from "@reduxjs/toolkit";
import { ErrBadBuild } from "./views/ErrBadBuild";
import { ErrOffline } from "./views/ErrOffline";
import { Main } from "./views/Main";
import { Provider, useDispatch, useSelector } from "react-redux";
import { useEffect } from "react";

const root: ReactDOM.Root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

enum WebSocketState {
  Connecting = "Connecting",
  ErrBadBuild = "ErrBadBuild",
  ErrOffline = "ErrOffline",
}

type TWebSocketStateUpdatePayload = {};

const websocketSlice = createSlice({
  name: "websocket",
  initialState: WebSocketState.Connecting as WebSocketState | TWebSocketStateUpdatePayload,
  reducers: {
    setState: (state, action) => {
      return action.payload;
    },
  },
});

const store = configureStore({
  reducer: {
    websocket: websocketSlice.reducer,
  },
});

const WebSocketConnector = () => {
  const dispatch = useDispatch();
  const websocketState = useSelector((state: any) => state.websocket);

  useEffect(() => {
    const backendHost = import.meta.env.VITE_BACKEND_HOST;
    if (!backendHost) {
      dispatch(websocketSlice.actions.setState(WebSocketState.ErrBadBuild));
      return;
    }

    const socketUrl =
      import.meta.env.MODE === "development"
        ? `ws://${backendHost}/sock`
        : `/sock`;

    const socket = new WebSocket(socketUrl);

    socket.onmessage = (event) => {
      const payload: TWebSocketStateUpdatePayload = JSON.parse(event.data);
      dispatch(websocketSlice.actions.setState(payload));
    };

    socket.onerror = () => {
      dispatch(websocketSlice.actions.setState(WebSocketState.ErrOffline));
    };

    socket.onclose = () => {
      dispatch(websocketSlice.actions.setState(WebSocketState.ErrOffline));
    };

    return () => {
      socket.close();
    };
  }, [dispatch]);

  if (websocketState === WebSocketState.Connecting) {
    return <div>Connecting...</div>;
  }

  if (websocketState === WebSocketState.ErrBadBuild) {
    return <ErrBadBuild />;
  }

  if (websocketState === WebSocketState.ErrOffline) {
    return <ErrOffline />;
  }

  return <Main />;
};

root.render(
  <Provider store={store}>
    <WebSocketConnector />
  </Provider>
);
