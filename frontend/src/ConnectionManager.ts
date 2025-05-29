import SessionSlice from "./state/slices/session";
import Store from "./state/store";

class ConnectionManager {
  #url_status: URL = new URL("http://127.0.0.1:8080/api/status");

  public async start(): Promise<void> {
    let response: Response;
    try {
      response = await fetch(this.#url_status);
    } catch (err) {
      Store.dispatch(SessionSlice.actions.set_error({ name: "N/A", message: "N/A", stack: "N/A", code: "N/A" }));
      return;
    }
  }
}

export default ConnectionManager;
