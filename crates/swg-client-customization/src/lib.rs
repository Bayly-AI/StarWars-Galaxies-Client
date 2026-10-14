// crates/swg-client-customization/src/lib.rs

pub mod character;
pub mod morph;
pub mod palette;
pub mod species;

pub use character::{
    CharacterAppearance, VAR_CLOTH_PRIMARY, VAR_CLOTH_SECONDARY, VAR_EYE_COLOR, VAR_HAIR_COLOR,
    VAR_SKIN_COLOR,
};
pub use morph::{
    BaseMesh, DeformedMesh, MORPH_EYE_SIZE, MORPH_HEIGHT, MORPH_HORN_SIZE, MORPH_JAW_LINE,
    MORPH_LEKKU_LENGTH, MORPH_MUSCLE, MORPH_NOSE_WIDTH, MORPH_WEIGHT, MorphEvaluator,
    MorphSliderState, MorphTarget,
};
pub use palette::{ColorRamp, CustomizationVariable, PaletteLibrary};
pub use species::{Gender, Species, SpeciesProfile, SpeciesRegistry};
