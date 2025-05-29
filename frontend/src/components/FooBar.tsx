import * as react_redux from "react-redux";
import slice from "../state/slices/counter";
import type * as react from "react";
import type * as store from "../state/store";

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

export default FooBar;
