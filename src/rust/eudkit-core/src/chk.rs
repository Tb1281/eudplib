use crate::error::{EudkitError, Result};

pub const USED_SECTIONS: &[[u8; 4]] = &[
    *b"VER ", *b"VCOD", *b"OWNR", *b"ERA ", *b"DIM ", *b"SIDE", *b"MTXM",
    *b"UNIT", *b"THG2", *b"MASK", *b"STRx", *b"UPRP", *b"MRGN", *b"TRIG",
    *b"MBRF", *b"SPRP", *b"FORC", *b"COLR", *b"PUNI", *b"PUPx", *b"PTEx",
    *b"UNIx", *b"UPGx", *b"TECx", *b"CRGB",
];

pub const DETERMINISTIC_ISOM_LEN: u32 = 0x8000_0000 | 0x1919_43C8;

pub fn format_section_name(name: &[u8]) -> Result<[u8; 4]> {
    if name.len() > 4 {
        return Err(EudkitError::InvalidChkSection { offset: 0 });
    }
    let mut out = *b"    ";
    out[..name.len()].copy_from_slice(name);
    Ok(out)
}

#[derive(Debug, Clone, Default)]
pub struct Chk {
    sections: Vec<([u8; 4], Vec<u8>)>,
    has_isom_footer: bool,
}

impl Chk {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(raw: &[u8]) -> Result<Self> {
        let mut sections: Vec<([u8; 4], Vec<u8>)> = Vec::new();
        let mut has_isom_footer = false;
        let mut index = 0usize;

        while index < raw.len() {
            if index + 8 > raw.len() {
                return Err(EudkitError::InvalidChkSection { offset: index });
            }
            let mut name = [0u8; 4];
            name.copy_from_slice(&raw[index..index + 4]);
            let len_i32 = i32::from_le_bytes(raw[index + 4..index + 8].try_into().unwrap());

            if len_i32 < 0 {
                if name == *b"ISOM" && index + 8 == raw.len() {
                    has_isom_footer = true;
                    break;
                }
                return Err(EudkitError::InvalidChkSection { offset: index });
            }

            let len = len_i32 as usize;
            let end = (index + 8).saturating_add(len).min(raw.len());
            let data = raw[index + 8..end].to_vec();
            index = index.saturating_add(8).saturating_add(len);

            if let Some(existing) = sections.iter_mut().find(|(n, _)| *n == name) {
                existing.1 = data;
            } else {
                sections.push((name, data));
            }
        }

        Ok(Self {
            sections,
            has_isom_footer,
        })
    }

    pub fn has_isom_footer(&self) -> bool {
        self.has_isom_footer
    }

    pub fn sections(&self) -> &[([u8; 4], Vec<u8>)] {
        &self.sections
    }

    pub fn section(&self, name: impl AsRef<[u8]>) -> Option<&[u8]> {
        let key = format_section_name(name.as_ref()).ok()?;
        self.sections
            .iter()
            .find(|(n, _)| *n == key)
            .map(|(_, data)| data.as_slice())
    }

    pub fn set_section(&mut self, name: impl AsRef<[u8]>, data: Vec<u8>) -> Result<()> {
        let key = format_section_name(name.as_ref())?;
        if let Some(existing) = self.sections.iter_mut().find(|(n, _)| *n == key) {
            existing.1 = data;
        } else {
            self.sections.push((key, data));
        }
        Ok(())
    }

    pub fn remove_section(&mut self, name: impl AsRef<[u8]>) -> Option<Vec<u8>> {
        let key = format_section_name(name.as_ref()).ok()?;
        let pos = self.sections.iter().position(|(n, _)| *n == key)?;
        Some(self.sections.remove(pos).1)
    }

    pub fn optimize(&mut self) {
        self.sections
            .retain(|(name, _)| USED_SECTIONS.contains(name));
    }

    pub fn fix_map_data(&mut self) {
        // 1. Fix UNIT section default flags
        if let Some((_, unit)) = self.sections.iter_mut().find(|(n, _)| *n == *b"UNIT") {
            let record_count = unit.len() / 36;
            for idx in 0..record_count {
                let i = idx * 36;
                if unit[i + 17] == 100 {
                    unit[i + 14] &= !2;
                }
                if unit[i + 18] == 100 {
                    unit[i + 14] &= !4;
                }
                if unit[i + 24..i + 26] == [0, 0] {
                    unit[i + 14] &= !32;
                }
                for k in 0..5 {
                    let bit = 1u8 << k;
                    if unit[i + 26] & bit == 0 {
                        unit[i + 12] &= !bit;
                    }
                }

                if unit[i + 14] & 2 == 0 {
                    unit[i + 17] = 0;
                }
                if unit[i + 14] & 4 == 0 {
                    unit[i + 18] = 0;
                }
                if unit[i + 14] & 8 == 0 {
                    unit[i + 19] = 0;
                }
                if unit[i + 14] & 16 == 0 {
                    unit[i + 20..i + 24].fill(0);
                }
                if unit[i + 14] & 32 == 0 {
                    unit[i + 24..i + 26].fill(0);
                }
                for k in 0..5 {
                    let bit = 1u8 << k;
                    if unit[i + 12] & bit == 0 {
                        unit[i + 26] &= !bit;
                    }
                }
            }
        }

        // 2. Fix UNIx, UPGx, TECx default stats
        let stat_sections: &[([u8; 4], usize, &[usize])] = &[
            (*b"UNIx", 228, &[4, 2, 1, 2, 2, 2, 2]),
            (*b"UPGx", 61, &[2, 2, 2, 2, 2, 2]),
            (*b"TECx", 44, &[2, 2, 2, 2]),
        ];
        for &(sec_name, count, settings) in stat_sections {
            if let Some((_, data)) = self.sections.iter_mut().find(|(n, _)| *n == sec_name) {
                if data.len() < count {
                    continue;
                }
                for i in 0..count {
                    if data[i] == 0 {
                        continue;
                    }
                    let mut offset = count;
                    if sec_name == *b"UPGx" {
                        offset += 1;
                    }
                    for &setting_size in settings {
                        let start = offset + setting_size * i;
                        let end = start + setting_size;
                        if end <= data.len() {
                            data[start..end].fill(0);
                        }
                        offset += setting_size * count;
                    }
                }
            }
        }

        // 3. Apply Remastered VER = 206
        let _ = self.set_section(b"VER ", vec![0xCE, 0x00]);

        // 4. Replace 0000.00 null tiles in MTXM with 0000.01
        if let Some((_, mtxm)) = self.sections.iter_mut().find(|(n, _)| *n == *b"MTXM") {
            for chunk in mtxm.chunks_exact_mut(2) {
                if chunk == [0, 0] {
                    chunk[0] = 1;
                    chunk[1] = 0;
                }
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let total_len: usize = self
            .sections
            .iter()
            .map(|(_, data)| 8 + data.len())
            .sum::<usize>()
            + 8;
        let mut out = Vec::with_capacity(total_len);
        for (name, data) in &self.sections {
            out.extend_from_slice(name);
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(data);
        }
        out.extend_from_slice(b"ISOM");
        out.extend_from_slice(&DETERMINISTIC_ISOM_LEN.to_le_bytes());
        out
    }
}
