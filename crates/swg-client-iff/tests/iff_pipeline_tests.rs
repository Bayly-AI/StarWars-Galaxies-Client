// crates/swg-client-iff/tests/iff_pipeline_tests.rs

use swg_client_iff::{IffCursor, IffNode, IffWriter, StfWriter, StringTable};

#[test]
fn test_deep_nested_iff_template() {
    let mut writer = IffWriter::new();
    writer.begin_form(b"SHOT"); // Object template root
    {
        writer.write_chunk(b"0000", &[1, 0, 0, 0]);

        writer.begin_form(b"DERV"); // Base template derive
        {
            writer.write_chunk(b"XXXX", b"object/creature/base.iff\0");
        }
        writer.end_form();

        writer.begin_form(b"APPR"); // Appearance form
        {
            writer.begin_form(b"MESH"); // Mesh
            {
                writer.write_chunk(b"NAME", b"appearance/rancor.msh\0");
                writer.write_chunk(b"DATA", &[10, 20, 30, 40, 50]);
            }
            writer.end_form();
        }
        writer.end_form();
    }
    writer.end_form();

    let bytes = writer.finish();

    // 1. Recursive IffNode validation
    let root = IffNode::parse(&bytes).unwrap();
    assert_eq!(root.form_type(), Some(*b"SHOT"));

    let appr = root.find_form(b"APPR").unwrap();
    assert_eq!(appr.form_type(), Some(*b"APPR"));

    let mesh = appr.find_form(b"MESH").unwrap();
    assert_eq!(mesh.form_type(), Some(*b"MESH"));
    assert_eq!(
        mesh.find_chunk(b"NAME").unwrap(),
        b"appearance/rancor.msh\0"
    );
    assert_eq!(mesh.find_chunk(b"DATA").unwrap(), &[10, 20, 30, 40, 50]);

    // 2. Cursor navigation
    let mut cursor = IffCursor::new(&bytes);
    cursor.enter_form(Some(b"SHOT")).unwrap();
    let version = cursor.enter_chunk(b"0000").unwrap();
    assert_eq!(version, &[1, 0, 0, 0]);

    cursor.enter_form(Some(b"DERV")).unwrap();
    let parent = cursor.enter_chunk(b"XXXX").unwrap();
    assert_eq!(parent, b"object/creature/base.iff\0");
    cursor.exit_form().unwrap();

    cursor.enter_form(Some(b"APPR")).unwrap();
    cursor.enter_form(Some(b"MESH")).unwrap();
    let name = cursor.enter_chunk(b"NAME").unwrap();
    assert_eq!(name, b"appearance/rancor.msh\0");
    cursor.exit_form().unwrap();
    cursor.exit_form().unwrap();
    cursor.exit_form().unwrap();

    assert!(cursor.is_empty());
}

#[test]
fn test_stf_multilingual_unicode_support() {
    let mut writer = StfWriter::new();
    writer.add_string(
        10,
        "prompt_welcome",
        "Welcome to Mos Eisley! ¡Bienvenido! Willkommen!",
    );
    writer.add_string(20, "species_twi_lek", "Twi'lek 🪐");
    writer.add_string(30, "kanji_jedi", "ジェダイ・マスター");

    let bytes = writer.finish();
    let table = StringTable::parse(&bytes).unwrap();

    assert_eq!(table.len(), 3);
    assert_eq!(
        table.get("prompt_welcome"),
        Some("Welcome to Mos Eisley! ¡Bienvenido! Willkommen!")
    );
    assert_eq!(table.get("species_twi_lek"), Some("Twi'lek 🪐"));
    assert_eq!(table.get("kanji_jedi"), Some("ジェダイ・マスター"));
    assert_eq!(table.get_by_id(30), Some("ジェダイ・マスター"));
}
