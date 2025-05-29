import * as react_dom_client from "react-dom/client";
import * as react_redux from "react-redux";
import redux_store from "./state/store";
import slice from "./state/slices/counter";
import type * as react from "react";
import type * as store from "./state/store";

const FooBar = (): react.JSX.Element => {
  const dispatch = react_redux.useDispatch<store.AppDispatch>();
  const state: number = react_redux.useSelector<store.RootState, number>((state) => state.counter.value);

  return (
    <div>
      <p>FooBar: {state}</p>
      <button onClick={() => dispatch(slice.actions.increment_by_amount(3))}>Click me</button>
    </div>
  );
}

async function main() {
  console.log("Hello world!");

  const dom_root_elem: HTMLElement = document.getElementById("root")!;
  const react_root: react_dom_client.Root = react_dom_client.createRoot(dom_root_elem);

  const react_app: react.JSX.Element = (
    <react_redux.Provider store={redux_store}>
      <FooBar />
    </react_redux.Provider>
  );

  react_root.render(react_app);
}

main();
