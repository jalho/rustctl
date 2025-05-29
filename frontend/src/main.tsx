import type * as react from "react";
import * as react_dom from "react-dom/client";

async function main() {
  console.log("Hello world!");

  const root_elem: HTMLElement = document.getElementById("root")!;

  const root: react_dom.Root = react_dom.createRoot(root_elem);

  const app: react.JSX.Element = <>TODO</>;

  root.render(app);
}

main();
