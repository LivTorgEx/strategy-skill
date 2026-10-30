use lte_strategy_bridge::abi::{
    Direction, ModuleDataRequirements, ModuleEvent, ModuleInput, ModuleOpenPosition, ModuleOutput,
    ModulePlaceOrder,
};
use lte_strategy_bridge::{InternalIndicator, InternalIndicatorKind, MODULE_ABI_VERSION};
use std::collections::BTreeMap;

use crate::state::State;

// ── Position sizing ──────────────────────────────────────────────────────────

/// DCA price offsets from entry.
const DCA1_OFFSET: f64 = 0.005; // 0.5 %
const DCA2_OFFSET: f64 = 0.010; // 1.0 %

// ── Risk / reward ────────────────────────────────────────────────────────────

/// Take-profit distance from entry (positive = profit side).
#[allow(dead_code)]
const TAKE_PROFIT_PCT: f64 = 2.0; // 2 %

/// Stop-loss distance from entry (negative = loss side).
#[allow(dead_code)]
const STOP_LOSS_PCT: f64 = -1.0; // 1 %

// ── Order marks (stable identifiers) ────────────────────────────────────────

const DCA1_LONG: &str = "dca1-long";
const DCA2_LONG: &str = "dca2-long";
const DCA1_SHORT: &str = "dca1-short";
const DCA2_SHORT: &str = "dca2-short";

// ─────────────────────────────────────────────────────────────────────────────

/// Core strategy logic.
pub fn run(input: &ModuleInput, state: &mut State) -> ModuleOutput {
    if !matches!(&input.event, ModuleEvent::Requirements) {
        state.ticks += 1;
    }

    match &input.event {
        // Declare only the history this module needs. The host validates the
        // request, loads it once, then sends Init followed by incremental updates.
        ModuleEvent::Requirements => ModuleOutput {
            data_requirements: ModuleDataRequirements {
                abi_version: MODULE_ABI_VERSION,
                klines: BTreeMap::from([(60, 200), (300, 200)]),
                internal_indicators: BTreeMap::from([(
                    InternalIndicatorKind::Ntps,
                    BTreeMap::from([(60, 10)]),
                )]),
            },
            ..Default::default()
        },

        // ── Initialisation ────────────────────────────────────────────────────
        // Seed the local candle cache from the requested history. No trading
        // action is taken here.
        ModuleEvent::Init => {
            state.klines.init(&input.klines);
            ModuleOutput {
                debug: format!(
                    "init klines_1m={} klines_5m={} ticks={}",
                    state.klines.get(60).map_or(0, |rows| rows.len()),
                    state.klines.get(300).map_or(0, |rows| rows.len()),
                    state.ticks
                ),
                ..Default::default()
            }
        }

        ModuleEvent::KlineUpdate { timeframes } => {
            state.klines.update(&input.klines);
            let latest_1m = state.klines.get_prev(60, 0).map(|candle| candle.close);
            ModuleOutput {
                debug: format!(
                    "kline update tf={timeframes:?} latest_1m={latest_1m:?} ticks={}",
                    state.ticks
                ),
                ..Default::default()
            }
        }

        ModuleEvent::InternalIndicators { timeframes } => {
            let records = input
                .internal_indicators
                .iter()
                .filter(|(timeframe, _)| timeframes.contains(timeframe))
                .flat_map(|(_, records)| records.iter());
            let (record_count, latest_ntps) = records.fold((0, None), |(count, latest), record| {
                let ntps = match &record.indicator {
                    InternalIndicator::Ntps { value, .. } => Some(*value),
                };
                (count + 1, ntps.or(latest))
            });
            ModuleOutput {
                debug: format!(
                    "internal indicators tf={timeframes:?} records={record_count} latest_ntps={latest_ntps:?}"
                ),
                ..Default::default()
            }
        }

        // Dynamic module-requested indicators are a future feature.
        ModuleEvent::Indicators { .. } => ModuleOutput {
            debug: "dynamic indicators are not available yet".to_owned(),
            ..Default::default()
        },

        // ── Position closed ───────────────────────────────────────────────────
        // Cancel any unfilled DCA orders for the closed direction.
        ModuleEvent::FinishPosition { direction, pnl } => {
            let marks = dca_marks(*direction);
            ModuleOutput {
                cancel_orders: marks.iter().map(|m| m.to_string()).collect(),
                debug: format!(
                    "finish {:?} pnl={:.4} ticks={}",
                    direction, pnl, state.ticks
                ),
                ..Default::default()
            }
        }

        // ── Position just opened ──────────────────────────────────────────────
        // Place two DCA limit orders at 0.5 % and 1.0 % from actual entry.
        // Each order uses 25 % of max_open_amount to average into the trade.
        ModuleEvent::NewPosition {
            direction,
            entry_price,
            ..
        } => {
            let orders = build_dca_orders(*direction, *entry_price);
            ModuleOutput {
                place_orders: orders,
                debug: format!(
                    "new_pos {:?} entry={:.4} ticks={}",
                    direction, entry_price, state.ticks
                ),
                ..Default::default()
            }
        }

        // ── Tick / signal ─────────────────────────────────────────────────────
        // Use state.indicators (kept current by Init + Indicators events) to
        // evaluate entry conditions.  No indicator data is sent on Signal events.
        // Replace the stubs below with your real indicator conditions.
        // Entry uses a LIMIT order so slippage is controlled.
        ModuleEvent::Signal { .. } => {
            let opens: Vec<ModuleOpenPosition> = Vec::new();

            // ── Long entry ────────────────────────────────────────────────────
            if input.positions.long.is_none() {
                // TODO: add your indicator conditions here.
                // Example: enter long at current price as a limit order.
                //
                // opens.push(ModuleOpenPosition {
                //     direction: Direction::Long,
                //     amount: Some(input.max_amount * 0.5),
                //     qty: None,
                //     enter_price: Some(input.price),
                //     order_type: "Limit".to_string(),
                //     take_profit: Some(TAKE_PROFIT_PCT),
                //     stop_loss: Some(STOP_LOSS_PCT),
                //     note: "entry-long".to_string(),
                // });
            }

            // ── Short entry ───────────────────────────────────────────────────
            if input.positions.short.is_none() {
                // TODO: add your indicator conditions here.
                // Example: enter short at current price as a limit order.
                //
                // opens.push(ModuleOpenPosition {
                //     direction: Direction::Short,
                //     amount: Some(input.max_amount * 0.5),
                //     qty: None,
                //     enter_price: Some(input.price),
                //     order_type: "Limit".to_string(),
                //     take_profit: Some(TAKE_PROFIT_PCT),
                //     stop_loss: Some(STOP_LOSS_PCT),
                //     note: "entry-short".to_string(),
                // });
            }

            ModuleOutput {
                open_positions: opens,
                debug: format!("signal tick={} price={:.4}", state.ticks, input.price),
                ..Default::default()
            }
        }

        _ => ModuleOutput {
            debug: format!("tick={} price={:.4}", state.ticks, input.price),
            ..Default::default()
        },
    }
}

/// Build two DCA averaging limit orders for the given direction and entry price.
///
/// - **Long**: orders below entry (average down on dip).
/// - **Short**: orders above entry (average up on rally).
fn build_dca_orders(direction: Direction, entry_price: f64) -> Vec<ModulePlaceOrder> {
    let (dca1_price, dca1_mark, dca2_price, dca2_mark) = match direction {
        Direction::Long | Direction::Both => (
            entry_price * (1.0 - DCA1_OFFSET),
            DCA1_LONG,
            entry_price * (1.0 - DCA2_OFFSET),
            DCA2_LONG,
        ),
        Direction::Short => (
            entry_price * (1.0 + DCA1_OFFSET),
            DCA1_SHORT,
            entry_price * (1.0 + DCA2_OFFSET),
            DCA2_SHORT,
        ),
    };

    vec![
        ModulePlaceOrder {
            direction,
            amount: Some(125.0),
            qty: None,
            enter_price: dca1_price,
            take_profit: None,
            stop_loss: None,
            mark: dca1_mark.to_string(),
            order_side: Default::default(),
            reduce_only: false,
        },
        ModulePlaceOrder {
            direction,
            amount: Some(125.0),
            qty: None,
            enter_price: dca2_price,
            take_profit: None,
            stop_loss: None,
            mark: dca2_mark.to_string(),
            order_side: Default::default(),
            reduce_only: false,
        },
    ]
}

/// Returns the DCA marks for the given direction (used to cancel on close).
fn dca_marks(direction: Direction) -> &'static [&'static str] {
    match direction {
        Direction::Long | Direction::Both => &[DCA1_LONG, DCA2_LONG],
        Direction::Short => &[DCA1_SHORT, DCA2_SHORT],
    }
}
