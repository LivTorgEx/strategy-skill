#!/usr/bin/env python3
"""Generate NATR+NTPS+MRC+SAR Multi Long strategy JSON."""
import json

def math_op(left, op, right):
    return {"type": "Math", "value": {"type": "Operation", "left": left, "operation": op, "right": right}}

def position(field):
    return {"type": "Position", "value": field}

def number(v):
    return {"type": "Number", "value": v}

def direction(v):
    return {"type": "Direction", "value": v}

def variable(name):
    return {"type": "Variable", "name": name}

def global_val(name):
    return {"type": "Global", "value": name}

def indicator(tf, ind_obj, idx=0):
    return {"type": "Indicator", "token": "Chart", "timeframe": tf, "idx": idx, "indicator": ind_obj}

def op_filter(left, operation, right):
    return {"type": "Operation", "operation": operation, "left": left, "right": right}

def order_exist(mark_str, pside_direction):
    return {
        "type": "Order",
        "mark": {"type": "String", "value": mark_str},
        "pside": direction(pside_direction),
        "value": "Exist"
    }

# Price: EntryPrice ± (EntryPrice * pct_decimal * Factor)
def dca_price(pct_decimal):
    return math_op(
        position("EntryPrice"),
        "-",
        math_op(
            math_op(position("EntryPrice"), "*", number(pct_decimal)),
            "*",
            position("Factor")
        )
    )

def tp_price(pct_decimal):
    return math_op(
        position("EntryPrice"),
        "+",
        math_op(
            math_op(position("EntryPrice"), "*", number(pct_decimal)),
            "*",
            position("Factor")
        )
    )

# Amount = MaxAmount * 0.1667
def sixth_amount():
    return math_op(global_val("MaxAmount"), "*", number(0.1667))

# Amount threshold for DCA-N: MaxAmount * (N/6)
def dca_threshold(n):
    return math_op(global_val("MaxAmount"), "*", number(round(n / 6, 4)))

def create_dca_order(n, pct_decimal):
    mark = f"DCA{n}"
    return {
        "type": "Action",
        "filters": [
            op_filter(position("Amount"), ">", number(0.0)),
            op_filter(position("Amount"), "<", dca_threshold(n)),
            op_filter(order_exist(mark, "LONG"), "==", number(0.0))
        ],
        "action": {
            "type": "CreateOrder",
            "order_type": "LIMIT",
            "side": direction("LONG"),
            "pside": direction("LONG"),
            "mark": {"type": "String", "value": mark},
            "amount": sixth_amount(),
            "price": dca_price(pct_decimal),
            "msg": f"DCA{n} at -{round(pct_decimal*100, 1)}%"
        }
    }

def create_tp_order(n, pct_decimal):
    mark = f"TP{n}"
    return {
        "type": "Action",
        "filters": [
            op_filter(position("Amount"), ">", number(0.0)),
            op_filter(order_exist(mark, "LONG"), "==", number(0.0))
        ],
        "action": {
            "type": "CreateOrder",
            "order_type": "LIMIT",
            "side": direction("SHORT"),   # closing LONG
            "pside": direction("LONG"),
            "mark": {"type": "String", "value": mark},
            "amount": sixth_amount(),
            "price": tp_price(pct_decimal),
            "msg": f"TP{n} at +{round(pct_decimal*100, 1)}%"
        }
    }

def remove_order_action(mark_str):
    return {
        "filters": [],
        "action": {
            "type": "RemoveOrder",
            "mark": {"type": "String", "value": mark_str},
            "pside": direction("LONG")
        }
    }

# DCA levels: -1.5%, -3%, -5%, -7%, -10%, -13%
dca_levels = [
    (2, 0.015),
    (3, 0.030),
    (4, 0.050),
    (5, 0.070),
    (6, 0.100),
]

# TP levels: +2%, +3.5%, +5.5%, +7.5%, +10%, +13%
tp_levels = [
    (1, 0.020),
    (2, 0.035),
    (3, 0.055),
    (4, 0.075),
    (5, 0.100),
    (6, 0.130),
]

all_marks = [f"DCA{n}" for n, _ in dca_levels] + [f"TP{n}" for n, _ in tp_levels]

# Entry block
entry_block = {
    "type": "Action",
    "filters": [
        {"type": "IsEmpty", "value": position("Amount")},
        op_filter(variable("mrc5_long"), "==", number(1.0)),
        op_filter(variable("mrc30_long"), "==", number(1.0)),
        op_filter(
            indicator(300, {"type": "Psar", "property": "Direction"}),
            "==", direction("LONG")
        ),
        op_filter(
            indicator(300, {"type": "Ntps", "property": "Value"}),
            ">", number(50.0)
        ),
        op_filter(
            indicator(300, {"type": "Natr", "period": "14", "property": "Value"}),
            ">", number(0.3)
        ),
    ],
    "action": {
        "type": "ForceStartPosition",
        "side": direction("LONG"),
        "msg": "Entry: MRC5+MRC30 LONG + SAR LONG + NTPS>50 + NATR>0.3"
    }
}

# SAR exit block — also clears all pending DCA/TP orders
sar_exit_block = {
    "type": "Action",
    "filters": [
        op_filter(position("Amount"), ">", number(0.0)),
        op_filter(
            indicator(1800, {"type": "Psar", "property": "Direction"}),
            "==", direction("SHORT")
        ),
        op_filter(variable("mrc30_long"), "==", number(0.0)),
    ],
    "action": {
        "type": "Actions",
        "actions": [remove_order_action(m) for m in all_marks] + [
            {
                "filters": [],
                "action": {
                    "type": "ForceStopPosition",
                    "side": direction("LONG"),
                    "msg": "Exit: SAR 30m + MRC30 bearish"
                }
            }
        ]
    }
}

# on_indicators for MRC 5m and 30m direction tracking
def mrc_indicator_block(timeframe):
    return {
        "timeframe": timeframe,
        "filters": [],
        "actions": [
            {
                "filters": [
                    op_filter(
                        indicator(timeframe, {"type": "Mrc", "period": "20", "property": "Direction"}),
                        "==", direction("LONG")
                    )
                ],
                "action": {
                    "type": "SetVariable",
                    "name": "mrc5_long" if timeframe == 300 else "mrc30_long",
                    "value": number(1.0)
                }
            },
            {
                "filters": [
                    op_filter(
                        indicator(timeframe, {"type": "Mrc", "period": "20", "property": "Direction"}),
                        "==", direction("SHORT")
                    )
                ],
                "action": {
                    "type": "ClearVariable",
                    "name": "mrc5_long" if timeframe == 300 else "mrc30_long"
                }
            }
        ]
    }

strategy = {
    "name": "[NEW] NATR NTPS MRC SAR Multi Long",
    "api_key_id": 2,
    "margin_mode": "Isolated",
    "margin_leverage": 10,
    "margin": 100.0,
    "max_active_bots": 5,
    "symbol_ids": [],
    "settings": {
        "margin_mode": "Isolated",
        "margin_leverage": 10,
        "signal": {"name": "Indicator", "min_tf": 5},
        "strategy": {
            "name": "Trading",
            "max_open_amount": 500.0,
            "direction": "Long",
            "professional": {
                "variables": [
                    {
                        "name": "MRC 5m Long",
                        "key": "mrc5_long",
                        "type": "Variable",
                        "default": number(0.0)
                    },
                    {
                        "name": "MRC 30m Long",
                        "key": "mrc30_long",
                        "type": "Variable",
                        "default": number(0.0)
                    }
                ],
                "enter_price": {"type": "Wait"},
                "enter_amount": {"type": "Percentage", "source": "MaxAmount", "value": 16.67},
                "enter_direction": {"type": "Default"},
                "filters": [],
                "take_profit": None,
                "stop_loss": {
                    "activates": [],
                    "filters": [],
                    "order": {
                        "amount": {"source": "MaxAmount", "type": "Percentage", "value": 100.0},
                        "price": {"price": {"type": "Percentage", "value": -13.0}, "type": "Position"},
                        "type": "Market"
                    },
                    "type": "NextOrder"
                },
                "on_indicators": [
                    mrc_indicator_block(300),
                    mrc_indicator_block(1800),
                ],
                "on_analysis": (
                    [entry_block]
                    + [create_dca_order(n, pct) for n, pct in dca_levels]
                    + [create_tp_order(n, pct) for n, pct in tp_levels]
                    + [sar_exit_block]
                ),
                "on_actions": []
            }
        }
    }
}

print(json.dumps(strategy, indent=2))
