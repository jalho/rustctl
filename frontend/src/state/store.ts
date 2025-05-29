import * as reduxjs_toolkit from '@reduxjs/toolkit'
import counter from "./slices/counter";

const redux_store = reduxjs_toolkit.configureStore({
  reducer: {
    counter: counter.reducer,
  },
});

export type RootState = ReturnType<typeof redux_store.getState>;
export type AppDispatch = typeof redux_store.dispatch;
export default redux_store;
