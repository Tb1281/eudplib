use crate::error::{EudkitError, Result};
use crate::hir::epd;
use crate::lir::{
    LirAction, LirCondition, LirModule, LirObject, LirTrigger, LirVProcBuffer, ValueExpr,
};

#[derive(Debug, Clone, Default)]
struct OccupationBuilder {
    suboccupmap: bool,
    suboccupidx: u32,
    occupmap: Vec<i32>,
}

impl OccupationBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, occupied: bool) {
        let val = if !occupied {
            -1
        } else if let Some(&last) = self.occupmap.last() {
            if last != -1 {
                last
            } else {
                self.occupmap.len() as i32
            }
        } else {
            0
        };
        self.occupmap.push(val);
    }

    fn occup1(&mut self) {
        self.suboccupmap = true;
        self.suboccupidx += 1;
        if self.suboccupidx == 4 {
            self.push(self.suboccupmap);
            self.suboccupidx = 0;
            self.suboccupmap = false;
        }
    }

    fn write_space(&mut self, ssize: u32) {
        self.suboccupidx += ssize;
        if self.suboccupidx >= 4 {
            self.push(self.suboccupmap);
            self.suboccupidx -= 4;
            for _ in 0..(self.suboccupidx / 4) {
                self.push(false);
            }
            self.suboccupidx %= 4;
            self.suboccupmap = false;
        }
    }

    fn write_bytes_len(&mut self, ssize: usize) {
        for _ in 0..(ssize >> 2) {
            self.push(true);
        }
        for _ in 0..(ssize & 3) {
            self.occup1();
        }
    }

    fn write_trigger(&mut self, condition_count: usize, action_count: usize) {
        self.push(true); // prev_ptr
        self.push(true); // next_ptr

        // 16 Condition slots (5 dwords = 20 bytes each)
        for _ in 0..(condition_count * 5) {
            self.push(true);
        }
        if condition_count < 16 {
            for _ in 0..5 {
                self.push(true); // zeroed sentinel condition
            }
            self.write_space((20 * (15 - condition_count)) as u32);
        }

        // 64 Action slots (8 dwords = 32 bytes each)
        for _ in 0..(action_count * 8) {
            self.push(true);
        }
        if action_count < 64 {
            for _ in 0..8 {
                self.push(true); // zeroed sentinel action
            }
            self.write_space((32 * (63 - action_count)) as u32);
        }

        // Preserved / execution flags dword (+2376)
        self.push(true);

        // 27 unused trailing bytes + 1 byte currentAction = 0 at +2407
        self.write_space(27);
        self.occup1();
    }

    fn write_vproc_buffer(&mut self, slot_count: usize) {
        for _ in 0..4 {
            self.push(true); // nextptr
            self.write_space(15);
            self.occup1(); // nocond
            self.write_space(52);
        }
        self.push(true); // nextptr (slot 4)
        self.write_space(15);
        self.occup1(); // nocond (slot 4)
        self.write_space(16);

        // 72 * slot_count bytes of repeating action/nextptr blocks
        self.write_bytes_len(72 * slot_count);

        self.write_space(32);
        self.push(true); // flags
        self.write_space(27);
        self.occup1(); // currentAction

        for _ in 0..27 {
            self.write_space(40);
            self.push(true); // flags
            self.write_space(27);
            self.occup1(); // currentAction
        }
    }

    fn finish(mut self) -> Vec<i32> {
        if self.suboccupidx > 0 {
            self.push(self.suboccupmap);
            self.suboccupidx = 0;
        }
        self.occupmap
    }
}

pub fn trigger_occupation_map(condition_count: usize, action_count: usize) -> Vec<i32> {
    let mut builder = OccupationBuilder::new();
    builder.write_trigger(condition_count, action_count);
    builder.finish()
}

pub fn vproc_buffer_occupation_map(slot_count: usize) -> Vec<i32> {
    if slot_count == 0 {
        return Vec::new();
    }
    let mut builder = OccupationBuilder::new();
    builder.write_vproc_buffer(slot_count);
    builder.finish()
}

pub fn byte_buf_occupation_map(byte_len: usize) -> Vec<i32> {
    let mut builder = OccupationBuilder::new();
    builder.write_bytes_len(byte_len);
    builder.finish()
}

/// Packs object dword occupation maps into a single contiguous payload buffer.
/// Returns `(alloc_table_in_bytes, payload_size_in_bytes)`.
pub fn stack_objects(dwoccupmap_list: &[Vec<i32>]) -> (Vec<u32>, usize) {
    let total_dwords: usize = dwoccupmap_list.iter().map(Vec::len).sum();
    if total_dwords == 0 {
        return (vec![0; dwoccupmap_list.len()], 0);
    }

    let mut dwoccupmap_sum = vec![-1i32; total_dwords + 1];
    let mut alloctable = Vec::with_capacity(dwoccupmap_list.len());
    let mut lallocaddr = 0usize;
    let mut payload_size = 0usize;

    for dwoccupmap in dwoccupmap_list {
        let mut i = 0usize;
        while i < dwoccupmap.len() {
            if dwoccupmap[i] != -1 && dwoccupmap_sum[lallocaddr + i] != -1 {
                lallocaddr = (dwoccupmap_sum[lallocaddr + i] - dwoccupmap[i]) as usize;
                i = 0;
            } else {
                i += 1;
            }
        }

        for (i, &occup) in dwoccupmap.iter().enumerate().rev() {
            let curoff = lallocaddr + i;
            if occup != -1 || dwoccupmap_sum[curoff] != -1 {
                dwoccupmap_sum[curoff] = if dwoccupmap_sum[curoff + 1] == -1 {
                    curoff as i32 + 1
                } else {
                    dwoccupmap_sum[curoff + 1]
                };
            }
        }
        alloctable.push((lallocaddr * 4) as u32);

        let obj_size = lallocaddr + dwoccupmap.len();
        let obj_payload_size = obj_size * 4;
        if obj_payload_size > payload_size {
            payload_size = obj_payload_size;
        }
    }

    (alloctable, payload_size)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocatedPayload {
    pub data: Vec<u8>,
    pub prt_table: Vec<usize>,
    pub ort_table: Vec<usize>,
    pub object_offsets: Vec<u32>,
    pub entry_offset: u32,
}

struct PayloadWriter {
    datacur: usize,
    data: Vec<u8>,
    prt_table: Vec<usize>,
    ort_table: Vec<usize>,
}

impl PayloadWriter {
    fn new(size: usize) -> Self {
        Self {
            datacur: 0,
            data: vec![0u8; size],
            prt_table: Vec::new(),
            ort_table: Vec::new(),
        }
    }

    fn seek(&mut self, offset: usize) {
        self.datacur = offset;
    }

    fn write_u8(&mut self, v: u8) {
        self.data[self.datacur] = v;
        self.datacur += 1;
    }

    fn write_u16(&mut self, v: u16) {
        self.data[self.datacur..self.datacur + 2].copy_from_slice(&v.to_le_bytes());
        self.datacur += 2;
    }

    fn write_u32(&mut self, v: u32) {
        self.data[self.datacur..self.datacur + 4].copy_from_slice(&v.to_le_bytes());
        self.datacur += 4;
    }

    fn write_bytes(&mut self, b: &[u8]) {
        self.data[self.datacur..self.datacur + b.len()].copy_from_slice(b);
        self.datacur += b.len();
    }

    fn write_space(&mut self, len: usize) {
        self.datacur += len;
    }

    fn write_expr(&mut self, expr: ValueExpr, object_offsets: &[u32]) -> Result<()> {
        if self.datacur % 4 != 0 {
            return Err(EudkitError::Allocation(format!(
                "unaligned dword write at offset {}",
                self.datacur
            )));
        }
        let (val, rlocmode) = evaluate_value_expr(expr, object_offsets)?;
        match rlocmode {
            0 => {}
            1 => self.prt_table.push(self.datacur),
            4 => self.ort_table.push(self.datacur),
            other => {
                return Err(EudkitError::Allocation(format!(
                    "unsupported rlocmode {other}"
                )));
            }
        }
        self.data[self.datacur..self.datacur + 4].copy_from_slice(&val.to_le_bytes());
        self.datacur += 4;
        Ok(())
    }

    fn write_condition(&mut self, cond: &LirCondition, object_offsets: &[u32]) -> Result<()> {
        self.write_expr(cond.locid, object_offsets)?;
        self.write_expr(cond.player, object_offsets)?;
        self.write_expr(cond.amount, object_offsets)?;
        self.write_u16(cond.unitid);
        self.write_u8(cond.comparison);
        self.write_u8(cond.condtype);
        self.write_u8(cond.restype);
        self.write_u8(cond.flags);
        self.write_u16(cond.eudx);
        Ok(())
    }

    fn write_action(&mut self, act: &LirAction, object_offsets: &[u32]) -> Result<()> {
        self.write_expr(act.locid1, object_offsets)?;
        self.write_expr(act.strid, object_offsets)?;
        self.write_expr(act.wavid, object_offsets)?;
        self.write_expr(act.time, object_offsets)?;
        self.write_expr(act.player1, object_offsets)?;
        self.write_expr(act.player2, object_offsets)?;
        self.write_u16(act.unitid);
        self.write_u8(act.acttype);
        self.write_u8(act.amount);
        self.write_u8(act.flags);
        self.write_u8(0); // padding byte (+0x1D)
        self.write_u16(act.eudx);
        Ok(())
    }

    fn write_trigger(&mut self, trig: &LirTrigger, object_offsets: &[u32]) -> Result<()> {
        let start = self.datacur;
        self.write_expr(trig.prev_ptr, object_offsets)?;
        self.write_expr(trig.next_ptr, object_offsets)?;

        for c in &trig.conditions {
            self.write_condition(c, object_offsets)?;
        }
        if trig.conditions.len() < 16 {
            self.write_bytes(&[0u8; 20]);
            self.write_space(20 * (15 - trig.conditions.len()));
        }

        for a in &trig.actions {
            self.write_action(a, object_offsets)?;
        }
        if trig.actions.len() < 64 {
            self.write_bytes(&[0u8; 32]);
            self.write_space(32 * (63 - trig.actions.len()));
        }

        self.write_u32(trig.flags);
        self.write_space(27);
        self.write_u8(0); // currentAction = 0

        debug_assert_eq!(self.datacur - start, 2408);
        Ok(())
    }

    fn write_vproc_buffer(&mut self, vbuf: &LirVProcBuffer, object_offsets: &[u32]) -> Result<()> {
        if vbuf.slots.is_empty() {
            return Ok(());
        }
        let start = self.datacur;
        let slot_count = vbuf.slots.len();

        // First 5 nextptr + nocond header blocks (4 * 72 + 36 = 324 bytes)
        for i in 0..5 {
            let nptr = vbuf
                .slots
                .get(i)
                .map(|s| s.next_ptr)
                .unwrap_or(ValueExpr::Const(0));
            self.write_expr(nptr, object_offsets)?;
            self.write_space(15);
            self.write_u8(0); // nocond = 0
            if i < 4 {
                self.write_space(52);
            } else {
                self.write_space(16);
            }
        }
        debug_assert_eq!(self.datacur - start, 324);

        // 72 * slot_count repeating action/nextptr blocks
        for (i, slot) in vbuf.slots.iter().enumerate() {
            let blk_start = self.datacur;
            self.write_expr(slot.mask, object_offsets)?; // +0..+4: mask
            self.write_bytes(&[0u8; 12]); // +4..+16: strid, wavid, time (and nocond=0 for slot i-4)
            self.write_expr(slot.dest, object_offsets)?; // +16..+20: player1 (dest EPD)
            self.write_expr(slot.value, object_offsets)?; // +20..+24: player2 (value)
            let mod_dword = ((slot.modifier.encode() as u32) << 24) | 0x002D_0000;
            self.write_u32(mod_dword); // +24..+28: unitid=0, acttype=45 (SetDeaths), amount=modifier
            self.write_bytes(b"\x00\x00SC"); // +28..+32: flags=0, pad=0, eudx="SC"
            self.write_u32(4); // +32..+36: flags=4 (PRESERVE_TRIGGER) for slot i-28
            let next_slot_nptr = vbuf
                .slots
                .get(i + 5)
                .map(|s| s.next_ptr)
                .unwrap_or(ValueExpr::Const(0));
            self.write_expr(next_slot_nptr, object_offsets)?; // +36..+40: nextptr for slot i+5
            self.write_bytes(&[0u8; 32]); // +40..+72: zeroed (includes nocond=0 for slot i+5, act1 acttype=0 for slot i, currentAction=0 for slot i-28)
            debug_assert_eq!(self.datacur - blk_start, 72);
        }

        // Trailing footer (2008 bytes)
        self.write_space(32);
        self.write_u32(4); // flags = 4
        self.write_space(27);
        self.write_u8(0); // currentAction = 0

        for _ in 0..27 {
            self.write_space(40);
            self.write_u32(4); // flags = 4
            self.write_space(27);
            self.write_u8(0); // currentAction = 0
        }

        debug_assert_eq!(self.datacur - start, 2404 + 72 * (slot_count - 1));
        Ok(())
    }
}

pub fn evaluate_value_expr(expr: ValueExpr, object_offsets: &[u32]) -> Result<(i32, i32)> {
    match expr {
        ValueExpr::Const(v) => Ok((v, 0)),
        ValueExpr::ObjectAddress {
            object,
            byte_offset,
        } => {
            let base = *object_offsets
                .get(object.0 as usize)
                .ok_or_else(|| EudkitError::Allocation(format!("invalid ObjectId {}", object.0)))?;
            Ok(((base as i32).wrapping_add(byte_offset), 4))
        }
        ValueExpr::ObjectEpd {
            object,
            dword_offset,
        } => {
            let base = *object_offsets
                .get(object.0 as usize)
                .ok_or_else(|| EudkitError::Allocation(format!("invalid ObjectId {}", object.0)))?;
            Ok((epd(base).wrapping_add(dword_offset), 1))
        }
        ValueExpr::FieldAddress(fr) => {
            let base = *object_offsets.get(fr.object.0 as usize).ok_or_else(|| {
                EudkitError::Allocation(format!("invalid ObjectId {}", fr.object.0))
            })?;
            let addr = (base as i32).wrapping_add(fr.signed_byte_offset());
            Ok((addr, 4))
        }
        ValueExpr::FieldEpd(fr) => {
            let base = *object_offsets.get(fr.object.0 as usize).ok_or_else(|| {
                EudkitError::Allocation(format!("invalid ObjectId {}", fr.object.0))
            })?;
            let addr = (base as i32).wrapping_add(fr.signed_byte_offset());
            if addr.rem_euclid(4) != 0 {
                return Err(EudkitError::Allocation(format!(
                    "FieldEpd requires 4-byte aligned field, got offset {addr}"
                )));
            }
            Ok((epd(addr as u32), 1))
        }
    }
}

pub fn allocate_lir_module(lir: &LirModule) -> Result<AllocatedPayload> {
    allocate_lir_module_with_seed(lir, None)
}

pub fn allocate_lir_module_with_seed(
    lir: &LirModule,
    seed: Option<u64>,
) -> Result<AllocatedPayload> {
    let obj_count = lir.objects().len();
    let mut occup_maps = Vec::with_capacity(obj_count);
    for obj in lir.objects() {
        match obj {
            LirObject::Trigger(t) => {
                occup_maps.push(trigger_occupation_map(t.conditions.len(), t.actions.len()));
            }
            LirObject::VProcBuffer(vb) => {
                occup_maps.push(vproc_buffer_occupation_map(vb.slots.len()));
            }
            LirObject::ByteBuf(b) => {
                occup_maps.push(byte_buf_occupation_map(b.data.len()));
            }
        }
    }

    let (object_offsets, payload_size) = if let Some(s) = seed {
        let mut rng = crate::obscurate::ObscureRng::new(s);
        let mut order: Vec<usize> = (0..obj_count).collect();
        rng.shuffle(&mut order);

        let permuted_occup_maps: Vec<Vec<i32>> =
            order.iter().map(|&idx| occup_maps[idx].clone()).collect();
        let (permuted_offsets, p_size) = stack_objects(&permuted_occup_maps);

        let mut orig_offsets = vec![0u32; obj_count];
        for (pos, &orig_idx) in order.iter().enumerate() {
            orig_offsets[orig_idx] = permuted_offsets[pos];
        }
        (orig_offsets, p_size)
    } else {
        stack_objects(&occup_maps)
    };

    let mut writer = PayloadWriter::new(payload_size);

    for (idx, obj) in lir.objects().iter().enumerate() {
        writer.seek(object_offsets[idx] as usize);
        match obj {
            LirObject::Trigger(t) => writer.write_trigger(t, &object_offsets)?,
            LirObject::VProcBuffer(vb) => writer.write_vproc_buffer(vb, &object_offsets)?,
            LirObject::ByteBuf(b) => writer.write_bytes(&b.data),
        }
    }

    let entry_offset = *object_offsets
        .get(lir.entry_trigger.0 as usize)
        .ok_or_else(|| {
            EudkitError::Allocation(format!("invalid entry trigger {}", lir.entry_trigger.0))
        })?;

    Ok(AllocatedPayload {
        data: writer.data,
        prt_table: writer.prt_table,
        ort_table: writer.ort_table,
        object_offsets,
        entry_offset,
    })
}
