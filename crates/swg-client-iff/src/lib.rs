// crates/swg-client-iff/src/lib.rs

pub mod iff;
pub mod stf;

pub use iff::{IffCursor, IffError, IffNode, IffWriter, TAG_FORM, make_tag, tag_to_string};
pub use stf::{STF_MAGIC, StfError, StfWriter, StringTable};
