#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct State {
    pub ticks: u64,
}

pub fn load(v: Option<&serde_json::Value>) -> State {
    v.and_then(|x| serde_json::from_value(x.clone()).ok())
        .unwrap_or_default()
}

pub fn save(s: &State) -> Option<serde_json::Value> {
    serde_json::to_value(s).ok()
}
