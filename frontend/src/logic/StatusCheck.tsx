import * as libreact from "react";
import * as libredux from "react-redux";
import type * as libstatus from "../store/status";
import type * as libstore from "../store/_mod";

const StatusCheck = (props: { url: string }): libreact.ReactElement => {
  const state = libredux.useSelector<libstore.RootState, libstatus.State>((s) => {
    return s.status;
  });
  return <>TODO.</>;
};

export default StatusCheck;
