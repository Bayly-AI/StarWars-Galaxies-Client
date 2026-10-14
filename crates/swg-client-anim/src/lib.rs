// crates/swg-client-anim/src/lib.rs

pub mod ans;
pub mod blend;
pub mod sat;
pub mod skm;
pub mod state_machine;

pub use ans::{AnimationClip, Keyframe, TransformTrack};
pub use blend::{AnimationBlender, BoneTransform};
pub use sat::{AppearanceAttachment, AttachmentType, SkeletalAppearanceTemplate};
pub use skm::{Bone, Hardpoint, SkeletalMesh, SkeletalMeshWriter, SkinVertex};
pub use state_machine::{AnimationController, LocomotionState};
