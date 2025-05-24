export const reduxSliceName = "sync" as const;

export type SteamId = string;

export type Uuid = string;

export type Player = {
  id: SteamId;
  coordinates: { x: number; y: number; z: number };
  display_name: string;
  country: string;
};

export type ClientIdentity = {
  Anonymous: {
    session_id: Uuid;
  }
}

export type Client = {
  connected_at: string;
  identity: ClientIdentity;
};

/** State updates received from the backend over a WebSocket. */
export type WebSocketStateUpdatePayload = {
  clients: Record<Uuid, Client>;
  game: {
    _type: "Running";
    data: {
      players: Record<SteamId, Player>;
    };
  };
};

/** Redux slice state. */
export type State = null | WebSocketStateUpdatePayload;

export const reduxSliceInitialState: State = null;
