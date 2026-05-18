mod engine;
mod state;

fn main() {
    let mut st = state::State::default();
    lte_strategy_bridge::run_loop(|input| engine::run(&input, &mut st));
}
