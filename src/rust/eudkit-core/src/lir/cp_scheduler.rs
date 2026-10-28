use crate::hir::{epd, HirValue, Modifier, PlayerId, StockAction, CURRENT_PLAYER_ADDR};
use crate::lir::types::{ExpandedBlock, ExpandedOp};

pub fn schedule_cp_cursor(blocks: &mut [ExpandedBlock]) {
    for block in blocks {
        let epd_access_count = block
            .ops
            .iter()
            .filter(|op| match op {
                ExpandedOp::LeafAction(StockAction::SetDeaths {
                    player: PlayerId::Epd(p),
                    ..
                })
                | ExpandedOp::LeafAction(StockAction::SetDeathsX {
                    player: PlayerId::Epd(p),
                    ..
                }) => *p != epd(CURRENT_PLAYER_ADDR),
                _ => false,
            })
            .count();

        if epd_access_count < 2 {
            continue;
        }

        let old_ops = std::mem::take(&mut block.ops);
        let mut new_ops = Vec::with_capacity(old_ops.len() + 4);
        let mut current_cp: Option<i32> = None;

        for op in old_ops {
            match op {
                ExpandedOp::LeafAction(StockAction::SetDeaths {
                    player: PlayerId::Epd(target_epd),
                    modifier,
                    value,
                    unit,
                }) if unit == 0 && target_epd != epd(CURRENT_PLAYER_ADDR) => {
                    let u = match current_cp {
                        None => {
                            new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeaths {
                                player: PlayerId::Epd(epd(CURRENT_PLAYER_ADDR)),
                                modifier: Modifier::SetTo,
                                value: HirValue::Const(target_epd),
                                unit: 0,
                            }));
                            current_cp = Some(target_epd);
                            0
                        }
                        Some(cur) => {
                            let diff = target_epd - cur;
                            if diff >= 0 && diff % 12 == 0 && (diff / 12) <= 227 {
                                (diff / 12) as u16
                            } else {
                                let adj = target_epd - cur;
                                new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeaths {
                                    player: PlayerId::Epd(epd(CURRENT_PLAYER_ADDR)),
                                    modifier: Modifier::Add,
                                    value: HirValue::Const(adj),
                                    unit: 0,
                                }));
                                current_cp = Some(target_epd);
                                0
                            }
                        }
                    };
                    new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeaths {
                        player: PlayerId::CurrentPlayer,
                        modifier,
                        value,
                        unit: u,
                    }));
                }
                ExpandedOp::LeafAction(StockAction::SetDeathsX {
                    player: PlayerId::Epd(target_epd),
                    modifier,
                    value,
                    unit,
                    mask,
                }) if unit == 0 && target_epd != epd(CURRENT_PLAYER_ADDR) => {
                    let u = match current_cp {
                        None => {
                            new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeaths {
                                player: PlayerId::Epd(epd(CURRENT_PLAYER_ADDR)),
                                modifier: Modifier::SetTo,
                                value: HirValue::Const(target_epd),
                                unit: 0,
                            }));
                            current_cp = Some(target_epd);
                            0
                        }
                        Some(cur) => {
                            let diff = target_epd - cur;
                            if diff >= 0 && diff % 12 == 0 && (diff / 12) <= 227 {
                                (diff / 12) as u16
                            } else {
                                let adj = target_epd - cur;
                                new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeaths {
                                    player: PlayerId::Epd(epd(CURRENT_PLAYER_ADDR)),
                                    modifier: Modifier::Add,
                                    value: HirValue::Const(adj),
                                    unit: 0,
                                }));
                                current_cp = Some(target_epd);
                                0
                            }
                        }
                    };
                    new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeathsX {
                        player: PlayerId::CurrentPlayer,
                        modifier,
                        value,
                        unit: u,
                        mask,
                    }));
                }
                other => {
                    if matches!(
                        other,
                        ExpandedOp::CallSetup { .. }
                            | ExpandedOp::CallIndirectSetup { .. }
                            | ExpandedOp::MemReadSetup { .. }
                    ) {
                        if current_cp.is_some() {
                            new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeaths {
                                player: PlayerId::Epd(epd(CURRENT_PLAYER_ADDR)),
                                modifier: Modifier::SetTo,
                                value: HirValue::Const(0),
                                unit: 0,
                            }));
                            current_cp = None;
                        }
                    }
                    new_ops.push(other);
                }
            }
        }

        if current_cp.is_some() {
            new_ops.push(ExpandedOp::LeafAction(StockAction::SetDeaths {
                player: PlayerId::Epd(epd(CURRENT_PLAYER_ADDR)),
                modifier: Modifier::SetTo,
                value: HirValue::Const(0),
                unit: 0,
            }));
        }

        block.ops = new_ops;
    }
}
