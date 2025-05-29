import SessionSlice from "./state/slices/session";
import Store from "./state/store";

class ConnectionManager {
  static WEBSOCKET: null | WebSocket = null;
  static WS_EVENT_HANDLER_CLOSE: any = null;
  static WS_EVENT_HANDLER_ERROR: any = null;
  static WS_EVENT_HANDLER_MESSAGE: any = null;
  static WS_EVENT_HANDLER_OPEN: any = null;

  /*
   * TODO: Use different host for API in "release" build... (Plan is to deploy
   *       the UI and the backend in different clouds...)
   */
  static URL_STATUS: string = "/api/status";
  static URL_LOGIN: string = "/api/login";
  static URL_WEBSOCKET: string = "/api/websocket";

  static async restart(): Promise<void> {
    this.#abort_close_free_all();
    Store.dispatch(SessionSlice.actions.set_initializing());

    let response: Response;
    let response_instant: string;
    try {
      response = await fetch(this.URL_STATUS);
      response_instant = new Date().toISOString();
    } catch (err) {
      const web_api_err = err as DOMException;
      const fetch_api_err = new FetchAPIError(web_api_err);
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

    try {
      this.WEBSOCKET = new WebSocket(this.URL_WEBSOCKET);
    } catch (err) {
      this.#abort_close_free_all();
      const web_api_err = err as DOMException;
      const websocket_instantiation_error = new WebSocketInstantiationError(web_api_err);
      Store.dispatch(SessionSlice.actions.set_error({
        error_chain: collect_causes_enumerable(websocket_instantiation_error),
      }));
      return;
    }

    this.WS_EVENT_HANDLER_ERROR = (event: unknown) => {
      this.#abort_close_free_all();
      const websocket_emitted_error = new WebSocketEmittedError(event);
      Store.dispatch(SessionSlice.actions.set_error({
        error_chain: collect_causes_enumerable(websocket_emitted_error),
      }));
    };
    this.WEBSOCKET.addEventListener("error", this.WS_EVENT_HANDLER_ERROR);

    this.WS_EVENT_HANDLER_CLOSE = (event: any) => {
      const closed_at_instant: string = new Date().toISOString();
      this.#abort_close_free_all();
      Store.dispatch(SessionSlice.actions.set_session_disconnected({
        websocket_close: {
          closed_at_client_time: closed_at_instant,
          was_clean: event.wasClean,
          code: event.code,
        }
      }));
    };
    this.WEBSOCKET.addEventListener("close", this.WS_EVENT_HANDLER_CLOSE);

    this.WS_EVENT_HANDLER_MESSAGE = (event: unknown) => {
      console.debug("TODO: Deserialize message and set to Redux state", event);
    };
    this.WEBSOCKET.addEventListener("message", this.WS_EVENT_HANDLER_MESSAGE);
  }

  /**
   * Unregister all event handlers associated with a tracked WebSocket, and
   * close the socket, and NULL the tracking reference. No-op if there's no
   * tracked socket.
   *
   * TODO: Add AbortContoller, track all Fetch API requests too, and abort them...
   */
  static #abort_close_free_all() {
    if (this.WEBSOCKET) {
      if (this.WS_EVENT_HANDLER_CLOSE) {
        this.WEBSOCKET.removeEventListener("close", this.WS_EVENT_HANDLER_CLOSE);
        this.WS_EVENT_HANDLER_CLOSE = null;
      }
      if (this.WS_EVENT_HANDLER_MESSAGE) {
        this.WEBSOCKET.removeEventListener("message", this.WS_EVENT_HANDLER_MESSAGE);
        this.WS_EVENT_HANDLER_MESSAGE = null;
      }
      if (this.WS_EVENT_HANDLER_ERROR) {
        this.WEBSOCKET.removeEventListener("error", this.WS_EVENT_HANDLER_ERROR);
        this.WS_EVENT_HANDLER_ERROR = null;
      }
      if (this.WS_EVENT_HANDLER_OPEN) {
        this.WEBSOCKET.removeEventListener("open", this.WS_EVENT_HANDLER_OPEN);
        this.WS_EVENT_HANDLER_OPEN = null;
      }
      this.WEBSOCKET.close();
      this.WEBSOCKET = null;
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

export function collect_causes_enumerable<Error>(root: Error): Array<{ name: string, message: string, stack: string }> {
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
export class FetchAPIError extends Error {
  constructor(cause: DOMException) {
    super("Fetch API failed: maybe offline, or maybe TLS, CORS or DNS related issue, or something else", { cause });
  }
}

/**
 * The exception(s) that may occur when instantiating a `WebSocket`.
 */
class WebSocketInstantiationError extends Error {
  constructor(cause: unknown) {
    super("WebSocket instantiation failed: maybe offline, or maybe TLS, CORS or DNS related issue, or something else", { cause });
  }
}

/**
 * The event(s) that a `WebSocket` instance may emit as "error".
 */
class WebSocketEmittedError extends Error {
  constructor(cause: unknown) {
    super("WebSocket emitted error: maybe WebSocket protocol handshake was rejected, or something else", { cause });
  }
}
