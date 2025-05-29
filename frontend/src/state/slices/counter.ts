import * as reduxjs_toolkit from '@reduxjs/toolkit'

type State = {
  value: number
};

const initial_state: State = {
  value: 0,
};

function increment(state: State): void {
  state.value += 1;
}

function decrement(state: State): void {
  state.value -= 1;
}

function increment_by_amount(state: State, action: reduxjs_toolkit.PayloadAction<number>): void {
  state.value += action.payload;
}

const slice = reduxjs_toolkit.createSlice({
  name: "counter_slice" as const,
  initialState: initial_state,
  reducers: {
    increment,
    decrement,
    increment_by_amount,
  },
});

export default slice;
