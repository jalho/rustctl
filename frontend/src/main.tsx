import * as react_dom_client from "react-dom/client";
import * as react_redux from "react-redux";
import * as reduxjs_toolkit from '@reduxjs/toolkit'
import type * as react from "react";

namespace Counter {
  const slice_name = "counter_slice" as const;

  type State = {
    value: number
  };

  export const initial_state: State = {
    value: 0,
  };

  export const slice = reduxjs_toolkit.createSlice({
    name: slice_name,
    initialState: Counter.initial_state,
    reducers: {
      increment,
      decrement,
      increment_by_amount,
    },
  });

  function increment(state: State) {
    state.value += 1;
  }

  function decrement(state: State) {
    state.value -= 1;
  }

  function increment_by_amount(state: State, action: reduxjs_toolkit.PayloadAction<number>) {
    state.value += action.payload
  }
}

async function main() {
  console.log("Hello world!");

  const dom_root_elem: HTMLElement = document.getElementById("root")!;
  const react_root: react_dom_client.Root = react_dom_client.createRoot(dom_root_elem);

  const redux_store = reduxjs_toolkit.configureStore({
    reducer: {
      counter: Counter.slice.reducer,
    },
  });

  const react_app: react.JSX.Element = (
    <react_redux.Provider store={redux_store}>
      <>TODO</>
    </react_redux.Provider>
  );

  react_root.render(react_app);
}

main();
