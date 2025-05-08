import * as ReactDOM from "react-dom/client";
import { configureStore } from "@reduxjs/toolkit";
import { createSlice } from "@reduxjs/toolkit";
import { ErrBadBuild } from "./views/ErrBadBuild";
import { ErrOffline } from "./views/ErrOffline";
import { Main } from "./views/Main";
import { Provider, useDispatch, useSelector } from "react-redux";
import { useEffect } from "react";

const root: ReactDOM.Root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

enum Connection {
  Connecting = "Connecting",
  ErrBadBuild = "ErrBadBuild",
  ErrOffline = "ErrOffline",
}

export type SteamID = string;

export type Player = {
  id: SteamID;
  coordinates: { x: number; y: number; z: number };
  display_name: string;
  country: string;
};

/** State updates received from the backend over a WebSocket. */
export type TWebSocketStateUpdatePayload = {
  game: {
    _type: 'Running';
    data: {
      time_of_day: number;
      players: Record<SteamID, Player>;
    };
  };
};

/** State stored in Redux. */
type TGlobalState = Connection | TWebSocketStateUpdatePayload;

const websocketSlice = createSlice({
  name: "websocket",
  initialState: Connection.Connecting as TGlobalState,
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
  const state: TGlobalState = useSelector((state: { websocket: TGlobalState }) => state.websocket);

  useEffect(() => {
    let socketUrl: string;
    if (import.meta.env.MODE === "development") {
      const backendHost = import.meta.env.VITE_BACKEND_HOST;
      if (!backendHost) {
        dispatch(websocketSlice.actions.setState(Connection.ErrBadBuild));
        return;
      } else {
        socketUrl = `ws://${backendHost}/sock`;
      }
    } else {
      socketUrl = "/sock";
    }

    const socket = new WebSocket(socketUrl);

    socket.onmessage = (event) => {
      const payload: TWebSocketStateUpdatePayload = JSON.parse(event.data);
      dispatch(websocketSlice.actions.setState(payload));
    };

    socket.onerror = () => {
      dispatch(websocketSlice.actions.setState(Connection.ErrOffline));
    };

    socket.onclose = () => {
      dispatch(websocketSlice.actions.setState(Connection.ErrOffline));
    };

    return () => {
      socket.close();
    };
  }, [dispatch]);

  if (state === Connection.Connecting) {
    return <div>Connecting...</div>;
  } else if (state === Connection.ErrBadBuild) {
    return <ErrBadBuild />;
  } else if (state === Connection.ErrOffline) {
    return <ErrOffline />;
  } else {
    return <Main players={state.game.data.players} />;
  }

};

root.render(
  <Provider store={store}>
    <WebSocketConnector />
  </Provider>
);
