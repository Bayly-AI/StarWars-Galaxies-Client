// crates/swg-client-iff/src/iff.rs

use thiserror::Error;

pub const TAG_FORM: [u8; 4] = *b"FORM";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum IffError {
    #[error("Unexpected end of file while reading IFF tag")]
    UnexpectedEof,
    #[error("Expected FORM tag, found {0:?}")]
    ExpectedForm(String),
    #[error("Expected form type '{0}', found '{1}'")]
    MismatchedFormType(String, String),
    #[error("Expected chunk tag '{0}', found '{1}'")]
    MismatchedChunkTag(String, String),
    #[error("Chunk size {0} exceeds enclosing container bounds {1}")]
    OutOfBounds(usize, usize),
    #[error("Attempted to exit form while at root level")]
    CannotExitRoot,
    #[error("Malformed null-terminated string")]
    InvalidString,
}

/// Helper converting a 4-byte chunk tag to a printable string.
pub fn tag_to_string(tag: &[u8; 4]) -> String {
    String::from_utf8_lossy(tag).to_string()
}

/// Helper converting a 4-char string literal to a 4-byte array tag.
#[must_use]
pub const fn make_tag(s: &[u8; 4]) -> [u8; 4] {
    *s
}

/// Represents an IFF node in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IffNode<'a> {
    Form {
        form_type: [u8; 4],
        children: Vec<IffNode<'a>>,
    },
    Chunk {
        tag: [u8; 4],
        data: &'a [u8],
    },
}

impl<'a> IffNode<'a> {
    /// Recursively parses an IFF hierarchy from a byte slice.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, IffError> {
        let mut cursor = IffCursor::new(bytes);
        cursor.parse_node()
    }

    /// Returns the form type if this is a Form node.
    #[must_use]
    pub fn form_type(&self) -> Option<[u8; 4]> {
        match self {
            IffNode::Form { form_type, .. } => Some(*form_type),
            IffNode::Chunk { .. } => None,
        }
    }

    /// Finds first child chunk with matching tag.
    #[must_use]
    pub fn find_chunk(&self, tag: &[u8; 4]) -> Option<&'a [u8]> {
        match self {
            IffNode::Form { children, .. } => children.iter().find_map(|child| match child {
                IffNode::Chunk { tag: t, data } if t == tag => Some(*data),
                _ => None,
            }),
            IffNode::Chunk { .. } => None,
        }
    }

    /// Finds first child form with matching form type.
    #[must_use]
    pub fn find_form(&self, target_type: &[u8; 4]) -> Option<&IffNode<'a>> {
        match self {
            IffNode::Form { children, .. } => children.iter().find(|child| match child {
                IffNode::Form { form_type, .. } => form_type == target_type,
                _ => false,
            }),
            IffNode::Chunk { .. } => None,
        }
    }
}

/// Zero-copy cursor providing forward and stack-based hierarchical navigation through IFF files.
pub struct IffCursor<'a> {
    data: &'a [u8],
    offset: usize,
    /// Stack of (form_type, end_offset) for nested container scopes
    stack: Vec<([u8; 4], usize)>,
}

impl<'a> IffCursor<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            stack: Vec::new(),
        }
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        let limit = self.current_limit();
        limit.saturating_sub(self.offset)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    #[must_use]
    pub fn current_offset(&self) -> usize {
        self.offset
    }

    fn current_limit(&self) -> usize {
        self.stack.last().map_or(self.data.len(), |&(_, end)| end)
    }

    /// Enters a `FORM` container, verifying its 4-byte sub-type if specified.
    pub fn enter_form(&mut self, expected_type: Option<&[u8; 4]>) -> Result<[u8; 4], IffError> {
        let limit = self.current_limit();
        if self.offset + 12 > limit {
            return Err(IffError::UnexpectedEof);
        }

        let tag = [
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ];

        if tag != TAG_FORM {
            return Err(IffError::ExpectedForm(tag_to_string(&tag)));
        }

        let size = u32::from_be_bytes([
            self.data[self.offset + 4],
            self.data[self.offset + 5],
            self.data[self.offset + 6],
            self.data[self.offset + 7],
        ]) as usize;

        let form_end = self.offset + 8 + size;
        if form_end > limit {
            return Err(IffError::OutOfBounds(form_end, limit));
        }

        let form_type = [
            self.data[self.offset + 8],
            self.data[self.offset + 9],
            self.data[self.offset + 10],
            self.data[self.offset + 11],
        ];

        if let Some(expected) = expected_type
            && &form_type != expected
        {
            return Err(IffError::MismatchedFormType(
                tag_to_string(expected),
                tag_to_string(&form_type),
            ));
        }

        self.offset += 12;
        self.stack.push((form_type, form_end));

        Ok(form_type)
    }

    /// Exits the current `FORM` container, advancing the cursor to the end of the form.
    pub fn exit_form(&mut self) -> Result<(), IffError> {
        let (_, form_end) = self.stack.pop().ok_or(IffError::CannotExitRoot)?;
        self.offset = form_end;
        Ok(())
    }

    /// Reads next chunk header and payload slice without allocating heap memory.
    pub fn read_chunk(&mut self) -> Result<([u8; 4], &'a [u8]), IffError> {
        let limit = self.current_limit();
        if self.offset + 8 > limit {
            return Err(IffError::UnexpectedEof);
        }

        let tag = [
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ];

        let size = u32::from_be_bytes([
            self.data[self.offset + 4],
            self.data[self.offset + 5],
            self.data[self.offset + 6],
            self.data[self.offset + 7],
        ]) as usize;

        let chunk_end = self.offset + 8 + size;
        if chunk_end > limit {
            return Err(IffError::OutOfBounds(chunk_end, limit));
        }

        let chunk_data = &self.data[self.offset + 8..chunk_end];
        self.offset = chunk_end;

        Ok((tag, chunk_data))
    }

    /// Verifies next chunk has expected tag and returns its payload slice.
    pub fn enter_chunk(&mut self, expected_tag: &[u8; 4]) -> Result<&'a [u8], IffError> {
        let (tag, data) = self.read_chunk()?;
        if &tag != expected_tag {
            return Err(IffError::MismatchedChunkTag(
                tag_to_string(expected_tag),
                tag_to_string(&tag),
            ));
        }
        Ok(data)
    }

    /// Checks if next item is a `FORM`.
    #[must_use]
    pub fn is_next_form(&self) -> bool {
        let limit = self.current_limit();
        if self.offset + 4 > limit {
            false
        } else {
            self.data[self.offset..self.offset + 4] == TAG_FORM
        }
    }

    fn parse_node(&mut self) -> Result<IffNode<'a>, IffError> {
        if self.is_next_form() {
            let form_type = self.enter_form(None)?;
            let mut children = Vec::new();
            while !self.is_empty() {
                children.push(self.parse_node()?);
            }
            self.exit_form()?;
            Ok(IffNode::Form {
                form_type,
                children,
            })
        } else {
            let (tag, data) = self.read_chunk()?;
            Ok(IffNode::Chunk { tag, data })
        }
    }
}

/// Helper for constructing valid IFF binary streams.
pub struct IffWriter {
    buffer: Vec<u8>,
    form_stack: Vec<(usize, [u8; 4])>, // (size_field_offset, form_type)
}

impl Default for IffWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl IffWriter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            form_stack: Vec::new(),
        }
    }

    /// Begins a `FORM` block with given 4-byte form sub-type.
    pub fn begin_form(&mut self, form_type: &[u8; 4]) {
        self.buffer.extend_from_slice(&TAG_FORM);
        let size_offset = self.buffer.len();
        self.buffer.extend_from_slice(&[0, 0, 0, 0]); // Placeholder for big-endian size
        self.buffer.extend_from_slice(form_type);
        self.form_stack.push((size_offset, *form_type));
    }

    /// Ends the current `FORM` block, patching the size field.
    pub fn end_form(&mut self) {
        if let Some((size_offset, _)) = self.form_stack.pop() {
            let form_content_start = size_offset + 4;
            let size = (self.buffer.len() - form_content_start) as u32;
            let bytes = size.to_be_bytes();
            self.buffer[size_offset..size_offset + 4].copy_from_slice(&bytes);
        }
    }

    /// Writes a leaf chunk with tag and byte payload.
    pub fn write_chunk(&mut self, tag: &[u8; 4], data: &[u8]) {
        self.buffer.extend_from_slice(tag);
        let size = data.len() as u32;
        self.buffer.extend_from_slice(&size.to_be_bytes());
        self.buffer.extend_from_slice(data);
    }

    /// Completes the stream and returns raw bytes.
    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        while !self.form_stack.is_empty() {
            self.end_form();
        }
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iff_roundtrip_nested() {
        let mut writer = IffWriter::new();
        writer.begin_form(b"WSNP"); // World Snapshot Form
        {
            writer.write_chunk(b"0000", &[1, 2, 3, 4]); // Version chunk

            writer.begin_form(b"NODS"); // Node List
            {
                writer.write_chunk(b"DATA", b"OBJECT_NODE_DATA");
                writer.write_chunk(b"INFO", &[0xAA, 0xBB, 0xCC]);
            }
            writer.end_form();
        }
        writer.end_form();

        let bytes = writer.finish();

        // Parse with IffNode
        let tree = IffNode::parse(&bytes).unwrap();
        assert_eq!(tree.form_type(), Some(*b"WSNP"));

        let version_chunk = tree.find_chunk(b"0000").unwrap();
        assert_eq!(version_chunk, &[1, 2, 3, 4]);

        let nods_form = tree.find_form(b"NODS").unwrap();
        assert_eq!(nods_form.form_type(), Some(*b"NODS"));
        assert_eq!(nods_form.find_chunk(b"DATA").unwrap(), b"OBJECT_NODE_DATA");

        // Cursor-based traversal
        let mut cursor = IffCursor::new(&bytes);
        let root_type = cursor.enter_form(Some(b"WSNP")).unwrap();
        assert_eq!(&root_type, b"WSNP");

        let (c_tag, c_data) = cursor.read_chunk().unwrap();
        assert_eq!(&c_tag, b"0000");
        assert_eq!(c_data, &[1, 2, 3, 4]);

        let sub_type = cursor.enter_form(Some(b"NODS")).unwrap();
        assert_eq!(&sub_type, b"NODS");

        let (d_tag, d_data) = cursor.read_chunk().unwrap();
        assert_eq!(&d_tag, b"DATA");
        assert_eq!(d_data, b"OBJECT_NODE_DATA");

        cursor.exit_form().unwrap();
        cursor.exit_form().unwrap();
        assert!(cursor.is_empty());
    }
}
