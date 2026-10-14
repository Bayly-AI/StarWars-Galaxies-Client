# Playbook: Issue 08 — Animation: Skeletal Rigging (.skm, .sat), Bone Transforms, and .ans Animation Blender (`swg-client-anim`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#8  
> **Branch:** `enhancement/8-skeletal-rigging-animation`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement character skeletal animation and modular armor attachment in `crates/swg-client-anim`:

1. **Skeletal Mesh .skm & Skeleton Rigging (`skm.rs`):**
   - Parse and serialize SWG skeletal mesh format (`SKMG`/`SKTM`) with IFF chunking.
   - Vertex streams with up to 4 bone influences (`[u16; 4]` indices, `[f32; 4]` weights with unit normalization).
   - Joint hierarchy with bone parent-child indices, local rest pose, and inverse bind pose matrices.
   - Hardpoint attachment locators for weapon sockets and accessory positioning.

2. **Skeletal Appearance Template .sat (`sat.rs`):**
   - Parse modular attachment tables (`STAT`) defining customization slots (head, chest, legs, boots, gloves, back).
   - Link modular rigid and skinned armor meshes to shared base skeleton bones.

3. **Keyframe Animation .ans & Pose Evaluation (`ans.rs`):**
   - Parse `.ans` animation clips with rotation quaternions (`Quat`), translations (`Vec3`), and scales (`Vec3`).
   - Time-sampled pose interpolation using `Quat::slerp` and `Vec3::lerp`.
   - Forward kinematics traversal computing global composite bone transforms and GPU skinning matrices ($M = G_{\text{bone}} \times B_{\text{inv}}$).

4. **Animation Blending & State Machine (`blend.rs`, `state_machine.rs`):**
   - Cross-fade blending between locomotion clips (Idle $\to$ Walk $\to$ Run) with smooth normalized transitions.
   - Locomotion and posture state machine (Standing, Kneeling, Prone, Combat).
   - Deform vertices on CPU/GPU skinning palette for testing and verification.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Accurately parses `.skm` skeletons, joints, and 4-bone weighted vertex skins.
- [x] Resolves `.sat` modular armor and clothing attachments onto underlying skeleton rigs.
- [x] Smoothly interpolates `.ans` keyframes with SLERP rotation and forward kinematics.
- [x] Evaluates cross-fade blending across idle/walk/run locomotion states.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all quality gates.
