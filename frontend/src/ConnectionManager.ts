import SessionSlice from "./state/slices/session";
import Store from "./state/store";

class ConnectionManager {
  #url_status: URL = new URL("http://localhost:8080/api/status");

  public async start(): Promise<void> {
    let response: Response;
    let response_instant: string;
    try {
      response = await fetch(this.#url_status);
      response_instant = new Date().toISOString();
    } catch (err) {
      /**
       * Fetch API error: Not necessarily "offline", but could be. A
       * non-exhaustive list of some possible causes:
       * - not connected to the internet (the "offline" case)
       * - DNS issues
       * - CORS issues
       * - TLS issues
       *
       * In other words, we just cannot reliably detect what's wrong exactly.
       * That's just a downside of the platform (web) I suppose.
       */
      const error = err as DOMException;
      Store.dispatch(SessionSlice.actions.set_error({
        name: display(error.name),
        message: display(error.message),
        stack: display(error.stack),
        code: display(error.code),
      }));
      return;
    }

    if (!response.ok) {
      Store.dispatch(SessionSlice.actions.set_unauthorized({
        checked_at_client_time: response_instant,
        rejection_http_status_code: response.status,
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
