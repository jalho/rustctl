import * as ReactDOM from "react-dom/client";
import { configureStore } from "@reduxjs/toolkit";
import { createSlice } from "@reduxjs/toolkit";
import { ErrBadBuild } from "./views/ErrBadBuild";
import { ErrOffline } from "./views/ErrOffline";
import { Main } from "./views/Main";
import { Provider, useDispatch, useSelector } from "react-redux";
import { useEffect } from "react";

const root: ReactDOM.Root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

enum ErrorType {
  None = "None",
  BadBuild = "BadBuild",
  Offline = "Offline",
}

const websocketSlice = createSlice({
  name: "websocket",
  initialState: {
    loading: true,
    error: ErrorType.None,
  },
  reducers: {
    setLoading: (state, action) => {
      state.loading = action.payload;
    },
    setError: (state, action) => {
      state.error = action.payload;
    },
  },
});

const messageSlice = createSlice({
  name: "message",
  initialState: null as any,
  reducers: {
    setMessage: (state, action) => {
      return action.payload;
    },
  },
});

const store = configureStore({
  reducer: {
    websocket: websocketSlice.reducer,
    message: messageSlice.reducer,
  },
});

const WebSocketConnector = () => {
  const dispatch = useDispatch();
  const { loading, error } = useSelector((state: any) => state.websocket);
  const message = useSelector((state: any) => state.message);

  useEffect(() => {
    const backendHost = import.meta.env.VITE_BACKEND_HOST;
    if (!backendHost) {
      dispatch(websocketSlice.actions.setError(ErrorType.BadBuild));
      dispatch(websocketSlice.actions.setLoading(false));
      return;
    }

    const socketUrl =
      import.meta.env.MODE === "development"
        ? `ws://${backendHost}/sock`
        : `/sock`;

    const socket = new WebSocket(socketUrl);

    socket.onopen = () => {
      dispatch(websocketSlice.actions.setLoading(false));
    };

    socket.onmessage = (event) => {
      const payload = JSON.parse(event.data);
      dispatch(messageSlice.actions.setMessage(payload));
    };

    socket.onerror = () => {
      dispatch(websocketSlice.actions.setError(ErrorType.Offline));
      dispatch(websocketSlice.actions.setLoading(false));
    };

    socket.onclose = () => {
      dispatch(websocketSlice.actions.setError(ErrorType.Offline));
      dispatch(websocketSlice.actions.setLoading(false));
    };

    return () => {
      socket.close();
    };
  }, [dispatch]);

  if (loading) {
    return <div>Connecting...</div>;
  }

  if (error === ErrorType.BadBuild) {
    return <ErrBadBuild />;
  }

  if (error === ErrorType.Offline) {
    return <ErrOffline />;
  }

  if (message === null) {
    return <div>Waiting for WebSocket connection...</div>;
  }

  return <Main />;
};

root.render(
  <Provider store={store}>
    <WebSocketConnector />
  </Provider>
);
