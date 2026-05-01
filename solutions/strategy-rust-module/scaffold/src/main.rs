mod engine;
mod state;

use std::io::{self, Read};

use lte_strategy_bridge::abi::ModuleOutput;

fn main() {
    let mut raw = String::new();
    if io::stdin().read_to_string(&mut raw).is_err() {
        emit(&ModuleOutput::default());
        return;
    }

    let input: lte_strategy_bridge::abi::ModuleInput = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            emit(&ModuleOutput::default());
            return;
        }
    };

    let mut st = state::load(input.state.as_ref());
    let mut out = engine::run(&input, &mut st);
    out.state = state::save(&st);
    emit(&out);
}

fn emit(out: &ModuleOutput) {
    println!("{}", serde_json::to_string(out).unwrap());
}
