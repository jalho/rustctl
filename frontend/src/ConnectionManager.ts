import Store from "./state/store";

class ConnectionManager {
  public start(): void {
    console.debug("TODO: Keep WebSocket connected", Store.getState());
  }
}

export default ConnectionManager;
