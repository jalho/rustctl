import SessionSlice from "./state/slices/session";
import Store from "./state/store";

let counter: number = 0;

class ConnectionManager {
  public start(): void {
    console.debug("TODO: Keep WebSocket connected", Store.getState());

    setInterval(() => {
      counter++;
      if (counter % 2 === 0) {
        Store.dispatch(SessionSlice.actions.set_offline());
      } else {
        Store.dispatch(SessionSlice.actions.set_unauthorized());
      }
    }, 3000);
  }
}

export default ConnectionManager;
