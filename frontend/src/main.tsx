import * as librdom from "react-dom/client";
import * as libredux from "react-redux";
import * as libstore from "./store/_mod";
import * as libutil from "./util";
import StatusCheck from "./logic/StatusCheck";

const element: HTMLElement = libutil.getRootElement();
const root: librdom.Root = librdom.createRoot(element);

const endpoints: libutil.Endpoints = libutil.getUrls();

const store: libstore.Store = libstore.init(libstore.reducers);

root.render(
  <libredux.Provider store={store}>
    <StatusCheck url={endpoints.status} />
  </libredux.Provider>
);
