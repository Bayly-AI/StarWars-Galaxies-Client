// crates/swg-client-iff/src/stf.rs

use std::collections::HashMap;
use thiserror::Error;

pub const STF_MAGIC: [u8; 4] = *b"STF ";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum StfError {
    #[error("Unexpected EOF while parsing STF table")]
    UnexpectedEof,
    #[error("Invalid STF header magic: expected 'STF ', got {0:?}")]
    InvalidMagic([u8; 4]),
    #[error("Malformed UTF-16 text in string table: {0}")]
    InvalidUtf16(String),
    #[error("Malformed UTF-8 key in string table")]
    InvalidKey,
}

/// Localized String Table (.stf) holding localized strings indexed by key and integer ID.
#[derive(Debug, Clone, Default)]
pub struct StringTable {
    by_key: HashMap<String, String>,
    by_id: HashMap<u32, String>,
}

impl StringTable {
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_key: HashMap::new(),
            by_id: HashMap::new(),
        }
    }

    /// Number of localized strings in table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// Looks up string by name / key (case-insensitive).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.by_key.get(&key.to_lowercase()).map(|s| s.as_str())
    }

    /// Looks up string by integer ID.
    #[must_use]
    pub fn get_by_id(&self, id: u32) -> Option<&str> {
        self.by_id.get(&id).map(|s| s.as_str())
    }

    /// Resolves an SWG localization token, such as `@item_n:blaster_e11` or `blaster_e11`.
    #[must_use]
    pub fn resolve_token<'a>(&'a self, token: &'a str) -> Option<&'a str> {
        let key = if let Some(stripped) = token.strip_prefix('@') {
            if let Some((_file, id_key)) = stripped.split_once(':') {
                id_key
            } else {
                stripped
            }
        } else {
            token
        };

        self.get(key)
    }

    /// Parses a binary .stf file.
    pub fn parse(bytes: &[u8]) -> Result<Self, StfError> {
        if bytes.len() < 12 {
            return Err(StfError::UnexpectedEof);
        }

        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != STF_MAGIC {
            return Err(StfError::InvalidMagic(magic));
        }

        let mut offset = 4;
        let entry_count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        let mut by_key = HashMap::with_capacity(entry_count);
        let mut by_id = HashMap::with_capacity(entry_count);

        for _ in 0..entry_count {
            if offset + 8 > bytes.len() {
                return Err(StfError::UnexpectedEof);
            }

            let id = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            offset += 4;

            let key_len = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + key_len > bytes.len() {
                return Err(StfError::UnexpectedEof);
            }
            let key_str = std::str::from_utf8(&bytes[offset..offset + key_len])
                .map_err(|_| StfError::InvalidKey)?
                .to_string();
            offset += key_len;

            if offset + 4 > bytes.len() {
                return Err(StfError::UnexpectedEof);
            }
            let val_char_count = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            offset += 4;

            let val_byte_len = val_char_count * 2;
            if offset + val_byte_len > bytes.len() {
                return Err(StfError::UnexpectedEof);
            }

            let mut u16_chars = Vec::with_capacity(val_char_count);
            for i in 0..val_char_count {
                let char_offset = offset + i * 2;
                let ch = u16::from_le_bytes([bytes[char_offset], bytes[char_offset + 1]]);
                u16_chars.push(ch);
            }
            offset += val_byte_len;

            let val_str = String::from_utf16(&u16_chars)
                .map_err(|e| StfError::InvalidUtf16(e.to_string()))?;

            by_id.insert(id, val_str.clone());
            by_key.insert(key_str.to_lowercase(), val_str);
        }

        Ok(Self { by_key, by_id })
    }
}

/// Helper for constructing binary .stf string tables.
pub struct StfWriter {
    entries: Vec<(u32, String, String)>, // (id, key, value)
}

impl Default for StfWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl StfWriter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_string(&mut self, id: u32, key: &str, value: &str) {
        self.entries.push((id, key.to_string(), value.to_string()));
    }

    #[must_use]
    pub fn finish(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&STF_MAGIC);
        let count = self.entries.len() as u32;
        buffer.extend_from_slice(&count.to_le_bytes());

        for (id, key, val) in &self.entries {
            buffer.extend_from_slice(&id.to_le_bytes());
            let key_bytes = key.as_bytes();
            buffer.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
            buffer.extend_from_slice(key_bytes);

            let utf16: Vec<u16> = val.encode_utf16().collect();
            buffer.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
            for ch in utf16 {
                buffer.extend_from_slice(&ch.to_le_bytes());
            }
        }

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stf_string_table_roundtrip() {
        let mut writer = StfWriter::new();
        writer.add_string(1001, "weapon_e11", "E-11 Carbine");
        writer.add_string(1002, "weapon_dl44", "DL-44 Heavy Blaster Pistol");
        writer.add_string(2001, "species_wookiee", "Wookiee");

        let bytes = writer.finish();
        let table = StringTable::parse(&bytes).unwrap();

        assert_eq!(table.len(), 3);
        assert_eq!(table.get("weapon_e11"), Some("E-11 Carbine"));
        assert_eq!(table.get("WEAPON_E11"), Some("E-11 Carbine")); // Case-insensitive
        assert_eq!(table.get_by_id(1002), Some("DL-44 Heavy Blaster Pistol"));

        // Token resolution
        assert_eq!(
            table.resolve_token("@item_n:weapon_dl44"),
            Some("DL-44 Heavy Blaster Pistol")
        );
        assert_eq!(table.resolve_token("species_wookiee"), Some("Wookiee"));
    }
}
