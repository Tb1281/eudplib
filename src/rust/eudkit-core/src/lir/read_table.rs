use crate::hir::{epd, Comparison, Modifier, CURRENT_PLAYER_ADDR};
use crate::lir::types::{
    ActField, FieldRef, LirAction, LirCondition, LirTrigger, ObjectId, TriggerRole,
    TriggerSlotField, ValueExpr, EUDX_SIGNATURE,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ReadTable {
    pub read_end_id: ObjectId,
    pub triggers: Vec<LirTrigger>,
    pub entries: HashMap<(u32, i32), ObjectId>,
    /// `(shift, most_significant_bit) -> first trigger of that bit chain`.
    ///
    /// Built while constructing the chains so `entry()` never scans trigger
    /// labels, which are diagnostic only.
    pub bit_entries: HashMap<(i32, u32), ObjectId>,
}

fn record_chain(
    entries: &mut HashMap<(i32, u32), ObjectId>,
    shift: i32,
    chain: &HashMap<u32, ObjectId>,
) {
    entries.extend(chain.iter().map(|(&bit, &id)| ((shift, bit), id)));
}

impl ReadTable {
    pub fn build(start_obj_id: u32) -> Self {
        let mut cur_id = start_obj_id;
        let read_end_id = ObjectId(cur_id);
        cur_id += 1;

        let mut triggers = Vec::new();
        let mut entries = HashMap::new();
        let mut bit_entries = HashMap::new();

        let copy_ret_action = LirAction {
            locid1: ValueExpr::Const(0),
            strid: ValueExpr::Const(0),
            wavid: ValueExpr::Const(0),
            time: ValueExpr::Const(0),
            player1: ValueExpr::Const(0),
            player2: ValueExpr::Const(0),
            unitid: 0,
            acttype: 45,
            amount: Modifier::SetTo.encode(),
            flags: 20,
            eudx: 0,
        };
        let reset_copy_ret = LirAction {
            locid1: ValueExpr::Const(0),
            strid: ValueExpr::Const(0),
            wavid: ValueExpr::Const(0),
            time: ValueExpr::Const(0),
            player1: ValueExpr::FieldEpd(FieldRef::new(
                read_end_id,
                TriggerSlotField::Action(0, ActField::Player2),
            )),
            player2: ValueExpr::Const(0),
            unitid: 0,
            acttype: 45,
            amount: Modifier::SetTo.encode(),
            flags: 20,
            eudx: 0,
        };
        let reset_cp = LirAction {
            locid1: ValueExpr::Const(0),
            strid: ValueExpr::Const(0),
            wavid: ValueExpr::Const(0),
            time: ValueExpr::Const(0),
            player1: ValueExpr::Const(epd(CURRENT_PLAYER_ADDR)),
            player2: ValueExpr::Const(0),
            unitid: 0,
            acttype: 45,
            amount: Modifier::SetTo.encode(),
            flags: 20,
            eudx: 0,
        };
        let read_end_trig = LirTrigger {
            id: read_end_id,
            label: Some("ReadTable_end".to_string()),
            role: TriggerRole::ReadTableEnd,
            prev_ptr: ValueExpr::Const(0),
            next_ptr: ValueExpr::Const(0),
            conditions: Vec::new(),
            actions: vec![copy_ret_action, reset_copy_ret, reset_cp],
            flags: 4,
        };
        triggers.push(read_end_trig);

        let build_chain = |msb: u32,
                           lsb: u32,
                           shift: i32,
                           triggers: &mut Vec<LirTrigger>,
                           cur_id: &mut u32|
         -> HashMap<u32, ObjectId> {
            let mut bit_to_obj = HashMap::new();
            let mut chain_ids = Vec::new();
            for bit in (lsb..=msb).rev() {
                let id = ObjectId(*cur_id);
                *cur_id += 1;
                bit_to_obj.insert(bit, id);
                chain_ids.push((bit, id));
            }
            for i in 0..chain_ids.len() {
                let (bit, id) = chain_ids[i];
                let next_obj = if i + 1 < chain_ids.len() {
                    chain_ids[i + 1].1
                } else {
                    read_end_id
                };
                let add_val = if shift >= 0 {
                    (1u32 << (bit + shift as u32)) as i32
                } else {
                    (1u32 << (bit - (-shift) as u32)) as i32
                };
                let cond = LirCondition {
                    locid: ValueExpr::Const((1u32 << bit) as i32),
                    player: ValueExpr::Const(13),
                    amount: ValueExpr::Const(1),
                    unitid: 0,
                    comparison: Comparison::AtLeast.encode(),
                    condtype: 15,
                    restype: 0,
                    flags: 0,
                    eudx: EUDX_SIGNATURE,
                };
                let act = LirAction {
                    locid1: ValueExpr::Const(0),
                    strid: ValueExpr::Const(0),
                    wavid: ValueExpr::Const(0),
                    time: ValueExpr::Const(0),
                    player1: ValueExpr::FieldEpd(FieldRef::new(
                        read_end_id,
                        TriggerSlotField::Action(0, ActField::Player2),
                    )),
                    player2: ValueExpr::Const(add_val),
                    unitid: 0,
                    acttype: 45,
                    amount: Modifier::Add.encode(),
                    flags: 20,
                    eudx: 0,
                };
                triggers.push(LirTrigger {
                    id,
                    label: Some(format!("ReadTable_s{shift}_b{bit}")),
                    role: TriggerRole::ReadTable,
                    prev_ptr: ValueExpr::Const(0),
                    next_ptr: ValueExpr::ObjectAddress {
                        object: next_obj,
                        byte_offset: 0,
                    },
                    conditions: vec![cond],
                    actions: vec![act],
                    flags: 4,
                });
            }
            bit_to_obj
        };

        // Chain 1: shift 0, bits 31..0
        let s0 = build_chain(31, 0, 0, &mut triggers, &mut cur_id);
        record_chain(&mut bit_entries, 0, &s0);
        entries.insert((0xFFFF_FFFF, 0), s0[&31]);
        entries.insert((0x0000_FFFF, 0), s0[&15]);
        entries.insert((0x0000_00FF, 0), s0[&7]);

        // Chain 2: shift -2, bits 31..2
        let s_2 = build_chain(31, 2, -2, &mut triggers, &mut cur_id);
        record_chain(&mut bit_entries, -2, &s_2);
        entries.insert((0xFFFF_FFFC, -2), s_2[&31]);

        // Chain 3: shift -8, bits 15..8
        let s_8 = build_chain(15, 8, -8, &mut triggers, &mut cur_id);
        record_chain(&mut bit_entries, -8, &s_8);
        entries.insert((0x0000_FF00, -8), s_8[&15]);

        // Chain 4: shift -16, bits 31..16
        let s_16 = build_chain(31, 16, -16, &mut triggers, &mut cur_id);
        record_chain(&mut bit_entries, -16, &s_16);
        entries.insert((0xFFFF_0000, -16), s_16[&31]);
        entries.insert((0x00FF_0000, -16), s_16[&23]);

        // Chain 5: shift -24, bits 31..24
        let s_24 = build_chain(31, 24, -24, &mut triggers, &mut cur_id);
        record_chain(&mut bit_entries, -24, &s_24);
        entries.insert((0xFF00_0000, -24), s_24[&31]);

        ReadTable {
            read_end_id,
            triggers,
            entries,
            bit_entries,
        }
    }

    /// Resolves an arbitrary `(mask, shift)` to the first read-table trigger.
    ///
    /// Exact-mask entries come from `entries`; other masks resolve through the
    /// precomputed `(shift, msb)` map. Trigger labels are never inspected.
    pub fn entry(&self, mask: u32, shift: i32) -> ObjectId {
        if let Some(&id) = self.entries.get(&(mask, shift)) {
            return id;
        }
        let msb = if mask == 0 {
            u32::MAX
        } else {
            31 - mask.leading_zeros()
        };
        match shift {
            0 | -2 | -8 | -16 | -24 => self
                .bit_entries
                .get(&(shift, msb))
                .copied()
                .unwrap_or(self.read_end_id),
            _ => self
                .entries
                .get(&(0xFFFF_FFFF, 0))
                .copied()
                .unwrap_or(self.read_end_id),
        }
    }
}
