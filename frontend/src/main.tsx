import * as react_dom_client from "react-dom/client";
import * as react_redux from "react-redux";
import redux_store from "./state/store";
import type * as react from "react";

async function main() {
  console.log("Hello world!");

  const dom_root_elem: HTMLElement = document.getElementById("root")!;
  const react_root: react_dom_client.Root = react_dom_client.createRoot(dom_root_elem);

  const react_app: react.JSX.Element = (
    <react_redux.Provider store={redux_store}>
      <>TODO</>
    </react_redux.Provider>
  );

  react_root.render(react_app);
}

main();
