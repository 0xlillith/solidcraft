use serde_json::json;

use crate::{Session, product_scene};

#[test]
fn named_opaque_finishes_reach_the_scene_and_survive_undo_and_reopen_without_changing_mass() {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"body_name":"Specimen","length":10,"width":10,"height":10})).unwrap();
    s.execute("material.assign", &json!({"bodies":["Specimen"],"material":"Steel"})).unwrap();
    let mass = s.execute("component.properties", &json!({})).unwrap();
    let library = s.execute("appearance.library", &json!({})).unwrap();
    let entries = library["appearances"].as_array().unwrap();
    for (name, ..) in solidcraft_doc::appearance::OPAQUE_FINISHES {
        let expected = solidcraft_doc::appearance::library(name).unwrap();
        assert!(entries.iter().any(|e| e["name"] == name && e["pbr"] == json!(expected.pbr.unwrap())));
        let before = s.doc.appearances.clone();
        s.execute("appearance.assign", &json!({"bodies":["Specimen"],"appearance":name.to_lowercase()})).unwrap();
        assert_eq!(s.doc.appearances.bodies["Specimen"], expected);
        assert_eq!(product_scene::extract(&s).unwrap().meshes[0].surface, expected.opaque_surface().unwrap());
        assert_eq!(s.execute("component.properties", &json!({})).unwrap(), mass);
        let saved = s.doc.to_json();
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.doc.appearances, before);
        s.execute("edit.redo", &json!({})).unwrap();
        let mut reopened = Session::new(solidcraft_doc::Document::from_json(&saved).unwrap());
        assert_eq!(reopened.doc.appearances.bodies["Specimen"], expected);
        assert_eq!(reopened.execute("component.properties", &json!({})).unwrap(), mass);
        assert!(solidcraft_io::catalog::Catalog::from_document(&reopened.doc, "user.fixture", "Original", Default::default()).is_err());
    }
}

#[test]
fn named_opaque_finish_refuses_transparency_atomically_and_allows_explicit_scalar_override() {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"body_name":"Specimen","length":10,"width":10,"height":10})).unwrap();
    let doc = s.doc.clone();
    let history = (s.undo.len(), s.redo.len(), s.revision);
    for opacity in [json!(0.5), json!(2), json!("invalid"), json!(null)] {
        assert!(s.execute("appearance.assign", &json!({"bodies":["Specimen"],"appearance":"Steel - Satin PBR","opacity":opacity})).is_err());
        assert_eq!(*s.doc, *doc);
        assert_eq!((s.undo.len(), s.redo.len(), s.revision), history);
    }
    s.execute("appearance.assign", &json!({"bodies":["Specimen"],"appearance":"Steel - Satin PBR","pbr":null,"opacity":0.5})).unwrap();
    assert!(s.doc.appearances.bodies["Specimen"].pbr.is_none());
    assert_eq!(s.doc.appearances.bodies["Specimen"].opacity, 0.5);
    for (name, ..) in solidcraft_doc::appearance::LIBRARY {
        assert!(solidcraft_doc::appearance::library(name).unwrap().pbr.is_none());
    }
}
