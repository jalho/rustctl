import * as reduxjs_toolkit from '@reduxjs/toolkit'
import session from "./slices/session";

const redux_store = reduxjs_toolkit.configureStore({
  reducer: {
    session: session.reducer,
  },
});

export type RootState = ReturnType<typeof redux_store.getState>;
export type AppDispatch = typeof redux_store.dispatch;
export default redux_store;
