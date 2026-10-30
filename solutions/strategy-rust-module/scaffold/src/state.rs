use std::collections::BTreeMap;

use lte_strategy_bridge::KlineHistory;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct State {
    pub ticks: u64,
    /// Requested candle history — rebuilt from Init/KlineUpdate events.
    #[serde(skip)]
    pub klines: KlineHistory,
}

impl State {
    pub fn new() -> Self {
        Self {
            klines: KlineHistory::new(&BTreeMap::from([(60, 200), (300, 200)])),
            ..Default::default()
        }
    }
}

pub fn load(v: Option<&serde_json::Value>) -> State {
    let mut state = v
        .and_then(|x| serde_json::from_value(x.clone()).ok())
        .unwrap_or_default();
    state.klines = KlineHistory::new(&BTreeMap::from([(60, 200), (300, 200)]));
    state
}

pub fn save(s: &State) -> Option<serde_json::Value> {
    serde_json::to_value(s).ok()
}
