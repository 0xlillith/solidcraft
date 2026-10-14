use super::*;
use crate::{dialogs::Kind, scenario::Harness};
use solidcraft_engine::{Sel, Session};

#[test]
fn catalog_clicked_load_apply_export_and_ok_keep_one_design_undo_step() {
    let mut h = Harness::new();
    h.call("ui.resize", json!({"width":1600,"height":1400}));
    let path = h.dir.join("source-library.json").to_string_lossy().into_owned();
    let mut source = Session::default();
    source.execute("solid.box", &json!({"length":10,"width":10,"height":10})).unwrap();
    source.execute("appearance.assign", &json!({"bodies":["Body1"],"appearance":"Warm oak colour","color":[180,120,70],"opacity":0.8})).unwrap();
    source.execute("library.export", &json!({"path":path})).unwrap();
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10})).unwrap();
    h.app
        .run(
            "appearance.assign",
            json!({"bodies":["Body1"],"color":[20,20,20],
        "pbr":{"metallic":1.0,"roughness":0.3,"dielectric_ior":1.5}}),
        )
        .unwrap();
    h.app.session.mark_saved();
    h.app.session.selection = vec![Sel::Body { name: "Body1".into() }];
    h.app.start("appearance.assign");
    if let Some(dialog) = &mut h.app.dialog
        && let Kind::Appearance(state) = &mut dialog.kind
    {
        state.local.path = path;
    }
    h.frames(3);
    let undo = h.app.session.undo.len();
    h.step(&json!({"widget":"Load library"})).unwrap();
    h.step(&json!({"click":{"handle":"library:entry:0"}})).unwrap();
    h.step(&json!({"widget":"Apply library appearance"})).unwrap();
    assert_eq!(h.app.session.doc.appearances.bodies["Body1"].name, "Warm oak colour");
    assert!(h.app.session.doc.appearances.bodies["Body1"].pbr.is_none());
    assert_eq!(h.app.session.undo.len(), undo + 1);
    let revision = h.app.session.revision;
    h.step(&json!({"widget":"Export design library..."})).unwrap();
    let exported = h.dir.join("my-library.solidcraft-library.json");
    let catalog = Catalog::read(&exported.to_string_lossy()).unwrap();
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].definition.name(), "Warm oak colour");
    assert_eq!(h.app.session.revision, revision);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(h.app.dialog.is_none());
    assert_eq!(h.app.session.undo.len(), undo + 1);
}

fn appearance_fixture() -> Result<(Harness, Arc<()>), String> {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10}))?;
    h.app.run("appearance.assign", json!({"bodies":["Body1"],"appearance":"Warm oak colour","color":[180,120,70]}))?;
    h.app.session.mark_saved();
    let token = start_appearance(&mut h)?;
    Ok((h, token))
}

fn start_appearance(h: &mut Harness) -> Result<Arc<()>, String> {
    h.app.start("appearance.assign");
    match h.app.dialog.as_ref().map(|dialog| &dialog.kind) {
        Some(Kind::Appearance(k)) => Ok(k.local.token.clone()),
        _ => Err("appearance dialog missing".into()),
    }
}

#[test]
fn async_library_export_is_bound_to_its_document_after_a_tab_switch() {
    use crate::file_dialog::test_support::{Slot, manual};
    let (mut h, token) = appearance_fixture().unwrap();
    let original = h.app.session.doc.clone();
    let path = h.dir.join("delayed-library.json");
    let ctx = egui::Context::default();
    let slot: Slot = Default::default();
    save_library(&mut h.app, token, false);
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    assert!(!path.exists());
    crate::documents::new_design(&mut h.app);
    let other = h.app.session.doc.clone();
    *slot.borrow_mut() = Some(Some(path.to_string_lossy().into_owned()));
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    assert_eq!(h.app.docs.active, 1);
    assert!(Arc::ptr_eq(&other, &h.app.session.doc));
    let catalog = Catalog::read(&path.to_string_lossy()).unwrap();
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].definition.name(), "Warm oak colour");
    crate::documents::switch(&mut h.app, 0);
    assert!(Arc::ptr_eq(&original, &h.app.session.doc));
    assert!(!h.app.session.is_dirty());
}

#[test]
fn async_library_export_captures_overwrite_consent_and_cancel_writes_nothing() {
    use crate::file_dialog::test_support::{Slot, manual};
    let (mut h, token) = appearance_fixture().unwrap();
    let path = h.dir.join("existing-library.json");
    std::fs::write(&path, b"keep this file").unwrap();
    let original = h.app.session.doc.clone();
    let ctx = egui::Context::default();
    let slot: Slot = Default::default();
    save_library(&mut h.app, token.clone(), false);
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    editor(&mut h.app, &token).unwrap().local.overwrite = true;
    *slot.borrow_mut() = Some(Some(path.to_string_lossy().into_owned()));
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    assert_eq!(std::fs::read(&path).unwrap(), b"keep this file");
    assert!(matches!(&h.app.status, Some((_, _, true))));
    save_library(&mut h.app, token, false);
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    *slot.borrow_mut() = Some(None);
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    assert!(!h.app.file_dialog_open());
    assert_eq!(std::fs::read(&path).unwrap(), b"keep this file");
    assert!(Arc::ptr_eq(&original, &h.app.session.doc));
}

#[test]
fn async_library_open_does_not_replace_a_new_appearance_editor() {
    use crate::file_dialog::test_support::{Slot, manual};
    let (mut h, token) = appearance_fixture().unwrap();
    let path = h.dir.join("source-library.json");
    h.app.run("library.export", json!({"path":path})).unwrap();
    let original = h.app.session.doc.clone();
    let ctx = egui::Context::default();
    let slot: Slot = Default::default();
    open_library(&mut h.app, token);
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    h.app.dialog = None;
    let newer = start_appearance(&mut h).unwrap();
    editor(&mut h.app, &newer).unwrap().local.path = "new editor draft".into();
    *slot.borrow_mut() = Some(Some(path.to_string_lossy().into_owned()));
    h.app.poll_file_dialog(&ctx, |_| manual(&slot));
    let k = editor(&mut h.app, &newer).unwrap();
    assert_eq!(k.local.path, "new editor draft");
    assert!(k.local.catalog.is_none());
    assert!(Arc::ptr_eq(&original, &h.app.session.doc));
}
