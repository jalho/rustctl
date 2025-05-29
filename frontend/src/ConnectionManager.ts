import SessionSlice from "./state/slices/session";
import Store from "./state/store";

class ConnectionManager {
  #url_status: URL = new URL("http://localhost:8080/api/status");

  public async start(): Promise<void> {
    let response: Response;
    try {
      response = await fetch(this.#url_status);
    } catch (err) {
      const error = err as DOMException;
      Store.dispatch(SessionSlice.actions.set_error({
        name: display(error.name),
        message: display(error.message),
        stack: display(error.stack),
        code: display(error.code),
      }));
      return;
    }
  }
}

export default ConnectionManager;

/**
 * Display a contentful string (length >=1), or an integer.
 */
function display(n: unknown, fallback = "N/A"): string {
  if (typeof n === "string" && n.length > 0) {
    return n;
  } else if (typeof n === "number" && Number.isInteger(n)) {
    return n.toString();
  } else {
    return fallback;
  }
}
