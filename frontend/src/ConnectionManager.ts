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
      const web_api_err = err as DOMException;
      const fetch_api_err = new FetchAPIError(
        "Fetch API failed: maybe offline, or maybe TLS, CORS or DNS related issue -- Who knows!",
        { cause: web_api_err },
      );

      Store.dispatch(SessionSlice.actions.set_error({
        error_chain: collect_causes_enumerable(fetch_api_err),
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

function collect_causes_enumerable<Error>(root: Error): Array<{ name: string, message: string, stack: string }> {
  if (!(root instanceof Error)) {
    return [];
  }

  const collected: ReturnType<typeof collect_causes_enumerable> = [];

  let current: any = root;
  while (current) {
    if (current instanceof Error) {
      collected.push({
        name: display(current.name),
        message: display(current.message),
        stack: display(current.stack),
      });
      current = current.cause;
    } else {
      break;
    }
  }

  return collected;
}

/**
 * Fetch API error: Not necessarily "offline", but could be. A non-exhaustive
 * list of some possible causes:
 * - not connected to the internet (the "offline" case)
 * - DNS issues
 * - CORS issues
 * - TLS issues
 *
 * In other words, we just cannot reliably detect what's wrong exactly. That's
 * just a downside of the platform (web) I suppose.
 */
class FetchAPIError extends Error { }
