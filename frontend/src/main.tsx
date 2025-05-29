import * as react_dom_client from "react-dom/client";
import * as react_redux from "react-redux";
import ConnectionManager from "./ConnectionManager";
import Session from "./components/Session";
import Store from "./state/store";
import type * as react from "react";

async function main() {
  console.log("Hello world!");

  const dom_root_elem: HTMLElement = document.getElementById("root")!;
  const react_root: react_dom_client.Root = react_dom_client.createRoot(dom_root_elem);

  const connection_manager = new ConnectionManager();
  connection_manager.start();

  const react_app: react.JSX.Element = (
    <react_redux.Provider store={Store}>
      <Session />
    </react_redux.Provider>
  );

  react_root.render(react_app);
}

main();
