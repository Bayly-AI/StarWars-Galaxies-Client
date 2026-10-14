// crates/swg-client-anim/tests/anim_pipeline_tests.rs

use glam::{Mat4, Quat, Vec2, Vec3};
use std::f32::consts::PI;
use swg_client_anim::{
    AnimationClip, AnimationController, AttachmentType, Bone, Hardpoint, Keyframe, LocomotionState,
    SkeletalAppearanceTemplate, SkeletalMesh, SkeletalMeshWriter, SkinVertex, TransformTrack,
};

fn create_test_humanoid_skeleton() -> Vec<Bone> {
    vec![
        // 0: Pelvis (Root)
        Bone {
            name: "root_pelvis".to_string(),
            parent_index: None,
            local_bind_transform: Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            inv_bind_pose: Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        },
        // 1: Spine
        Bone {
            name: "spine".to_string(),
            parent_index: Some(0),
            local_bind_transform: Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            inv_bind_pose: Mat4::from_translation(Vec3::new(0.0, -1.5, 0.0)),
        },
        // 2: Head
        Bone {
            name: "head".to_string(),
            parent_index: Some(1),
            local_bind_transform: Mat4::from_translation(Vec3::new(0.0, 0.3, 0.0)),
            inv_bind_pose: Mat4::from_translation(Vec3::new(0.0, -1.8, 0.0)),
        },
        // 3: Arm_R
        Bone {
            name: "arm_r".to_string(),
            parent_index: Some(1),
            local_bind_transform: Mat4::from_translation(Vec3::new(0.4, 0.0, 0.0)),
            inv_bind_pose: Mat4::from_translation(Vec3::new(-0.4, -1.5, 0.0)),
        },
        // 4: Hand_R
        Bone {
            name: "hand_r".to_string(),
            parent_index: Some(3),
            local_bind_transform: Mat4::from_translation(Vec3::new(0.0, -0.4, 0.0)),
            inv_bind_pose: Mat4::from_translation(Vec3::new(-0.4, -1.1, 0.0)),
        },
    ]
}

#[test]
fn test_humanoid_locomotion_blending_and_gpu_skinning() {
    let skeleton = create_test_humanoid_skeleton();

    let mut mesh_writer = SkeletalMeshWriter::new("shader/character/stormtrooper.sht");
    for b in &skeleton {
        mesh_writer.add_bone(b.clone());
    }

    // Vertex 0: Pelvis
    mesh_writer.add_vertex(SkinVertex {
        position: Vec3::new(0.0, 1.0, 0.0),
        normal: Vec3::Y,
        uv: Vec2::ZERO,
        bone_indices: [0, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    });

    // Vertex 1: Hand Right (bone 4)
    mesh_writer.add_vertex(SkinVertex {
        position: Vec3::new(0.4, 1.1, 0.0),
        normal: Vec3::X,
        uv: Vec2::ONE,
        bone_indices: [4, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    });

    mesh_writer.add_triangle(0, 1, 0);

    // Hardpoint: Blaster socket on right hand
    let weapon_hp = Hardpoint {
        name: "hold_r".to_string(),
        bone_index: 4,
        offset: Mat4::from_translation(Vec3::new(0.05, 0.0, 0.1)),
    };
    mesh_writer.add_hardpoint(weapon_hp.clone());

    let skm_bytes = mesh_writer.build();
    let mesh = SkeletalMesh::from_bytes(&skm_bytes).expect("Failed to parse humanoid SKM");

    // Create Idle Clip (neutral pose)
    let idle_clip = AnimationClip::new("idle", 2.0, true);

    // Create Running Clip (swinging right arm forward)
    let mut run_clip = AnimationClip::new("run", 1.0, true);
    let mut arm_track = TransformTrack::new("arm_r");
    // Swing arm around Z/X
    arm_track.rotations.push(Keyframe {
        time: 0.0,
        value: Quat::IDENTITY,
    });
    arm_track.rotations.push(Keyframe {
        time: 0.5,
        value: Quat::from_rotation_x(PI / 3.0),
    });
    arm_track.rotations.push(Keyframe {
        time: 1.0,
        value: Quat::IDENTITY,
    });
    run_clip.add_track(arm_track);

    // Setup Animation Controller
    let mut controller = AnimationController::new(LocomotionState::StandingIdle);
    controller.register_clip(LocomotionState::StandingIdle, idle_clip);
    controller.register_clip(LocomotionState::Running, run_clip);

    // 1. Initial rest pose at Idle
    let (_globals_idle, skin_idle) = controller.evaluate_skinning(0.0, &mesh);
    let def_idle = mesh.deform_vertices(&skin_idle);
    // At rest pose, deformed vertex matches bind pose
    assert!((def_idle[1].x - 0.4).abs() < 1e-3);
    assert!((def_idle[1].y - 1.1).abs() < 1e-3);

    // 2. Transition from Idle -> Run (halfway through transition blend)
    controller.transition_to(LocomotionState::Running);
    let (_globals_blend, skin_blend) = controller.evaluate_skinning(0.125, &mesh);
    let def_blend = mesh.deform_vertices(&skin_blend);
    // Arm has partially rotated
    assert!(def_blend[1] != def_idle[1]);

    // 3. Fully transitioned into Running state at 0.5s (peak swing)
    let (globals_run, skin_run) = controller.evaluate_skinning(0.375, &mesh);
    let def_run = mesh.deform_vertices(&skin_run);

    // Vertex 1 (hand) rotated on X axis, so Y and Z coordinates change
    assert!(def_run[1].z.abs() > 0.05);

    // 4. Verify blaster hardpoint followed hand bone accurately
    let hp_world =
        AnimationController::evaluate_hardpoint(&weapon_hp, &globals_run, Mat4::IDENTITY);
    let hand_world_pos = globals_run[4].w_axis.truncate();
    let hp_world_pos = hp_world.w_axis.truncate();
    let dist = hand_world_pos.distance(hp_world_pos);
    // Hardpoint offset length is hypot(0.05, 0.1) ~ 0.1118
    assert!((dist - 0.1118).abs() < 1e-3);
}

#[test]
fn test_sat_modular_armor_equipment_and_slots() {
    let mut sat = SkeletalAppearanceTemplate::new("appearance/mesh/human_m.skm");

    // Equip Stormtrooper Composite set
    sat.attach(
        "helmet",
        "appearance/mesh/armor_st_helmet.msh",
        AttachmentType::SocketRigid,
        Some("head".to_string()),
    );
    sat.attach(
        "chest",
        "appearance/mesh/armor_st_chest.skm",
        AttachmentType::SkeletalMesh,
        None,
    );
    sat.attach(
        "legs",
        "appearance/mesh/armor_st_legs.skm",
        AttachmentType::SkeletalMesh,
        None,
    );
    sat.attach(
        "weapon_r",
        "appearance/mesh/wp_rifle_dlt19.msh",
        AttachmentType::SocketRigid,
        Some("hold_r".to_string()),
    );

    let bytes = sat.to_bytes();
    let reloaded =
        SkeletalAppearanceTemplate::from_bytes(&bytes).expect("Failed to deserialize SAT");

    assert_eq!(reloaded.slots.len(), 4);
    assert!(reloaded.get_slot("helmet").is_some());
    assert!(reloaded.get_slot("chest").is_some());
    assert!(reloaded.get_slot("legs").is_some());
    assert!(reloaded.get_slot("weapon_r").is_some());

    // Unequip helmet
    let mut dynamic_sat = reloaded;
    let detached = dynamic_sat.detach("helmet");
    assert!(detached.is_some());
    assert!(dynamic_sat.get_slot("helmet").is_none());
    assert_eq!(dynamic_sat.slots.len(), 3);
}
