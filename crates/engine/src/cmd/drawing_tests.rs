use super::*;
use crate::cmd::{Part, command_specs, find_command};

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    s.execute(id, &p).unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn refused(s: &mut Session, id: &str, p: Value) -> String {
    s.execute(id, &p).err().map(|e| e.to_string()).unwrap_or_else(|| panic!("{id} accepted {p}"))
}

/// A folder of the test's own, removed when dropped.
struct Folder(std::path::PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let dir = solidcraft_io::vfs::temp_dir().join(format!("solidcraft-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Folder(dir)
    }
    fn path(&self, file: &str) -> String {
        self.0.join(file).to_str().unwrap().to_string()
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A design with a box.
fn design() -> Session {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 40, "width": 30, "height": 10, "body_name": "Plate"}));
    s
}

/// The drawing open, as the design holds it.
fn open_one(s: &Session) -> Arc<Drawing> {
    s.drawing("test").unwrap().clone()
}

/// The params naming drawing `id`.
fn the(id: u64) -> Value {
    json!({"drawing": ids::encode(id)})
}

fn names(s: &Session) -> Vec<&str> {
    s.doc.drawings.iter().map(|d| d.name.as_str()).collect()
}

/// A drawing of the design: one A3 sheet with the ISO border, the design its one source, kept in
/// the design and opened. Making it is one step of the model, and evaluates nothing.
#[test]
fn a_drawing_of_the_design_is_kept_in_it_and_opened() {
    let mut s = design();
    let (undo, evaluations, features) = (s.undo.len(), s.evaluations, s.doc.features.clone());
    let made = run(&mut s, "drawing.create", json!({"name": "Plate drawing", "units": "mm"}));
    assert_eq!((made["name"].as_str(), made["language"].as_str()), (Some("Plate drawing"), Some("en")));
    assert_eq!(made["units"], json!({"fixed": {"unit": "mm"}}));
    let sheet = &made["sheets"][0];
    assert_eq!(
        (sheet["size_mm"].clone(), sheet["orientation"].clone(), sheet["dpi"].clone()),
        (json!([420.0, 297.0]), json!("landscape"), json!(300.0))
    );
    assert_eq!((made["sheets"].as_array().map(Vec::len), made["sources"].as_array().map(Vec::len)), (Some(1), Some(1)));
    let d = open_one(&s);
    assert_eq!(made["id"], json!(ids::encode(d.id)));
    assert!(d.sheets[0].border.is_some(), "the ISO border");
    assert_eq!((s.doc.drawings.len(), s.open_drawing), (1, Some(d.id)));
    assert_eq!((s.undo.len(), s.undo.last().map(|u| u.part)), (undo + 1, Some(None)), "a step of the model");
    assert_eq!((s.evaluations, &s.doc.features), (evaluations, &features), "nothing evaluated, the model as it was");
    assert!(s.is_dirty());
    assert_eq!(run(&mut s, "drawing.inspect", json!({})), made);
    let listed = run(&mut s, "drawing.list", json!({}));
    assert_eq!(listed, json!({"drawings": [{"id": made["id"], "name": "Plate drawing", "sheets": 1, "open": true}]}));
    // Its name is the design's unless given.
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "drawing.create", json!({}));
    assert_eq!(open_one(&s).name, s.doc.name);
}

/// What making a drawing refuses, the design unchanged: an empty design, units, a language and a
/// name the record can't take, and a design that holds the most drawings already.
#[test]
fn a_drawing_is_made_only_of_a_design_with_something_in_it() {
    assert!(refused(&mut Session::default(), "drawing.create", json!({})).contains("nothing to draw"));
    let mut s = design();
    let before = s.doc.clone();
    for (p, says) in [
        (json!({"units": "parsec"}), "length unit"),
        (json!({"units": 3}), "follow_model"),
        (json!({"language": "e"}), "BCP 47"),
        (json!({"name": " "}), "name"),
        (json!({"name": 7}), "must be text"),
    ] {
        let e = refused(&mut s, "drawing.create", p.clone());
        assert!(e.contains(says), "{p}: {e}");
        assert!(Arc::ptr_eq(&s.doc, &before) && s.open_drawing.is_none() && s.undo.len() == 1, "{p}: nothing changed");
    }
    run(&mut s, "drawing.create", json!({}));
    run(&mut s, "drawing.close", json!({}));
    let one = s.doc.drawings[0].clone();
    s.doc_mut().drawings = (1..=drawing::MAX_DRAWINGS as u64).map(|id| Arc::new(Drawing { id, ..(*one).clone() })).collect();
    let full = s.doc.clone();
    assert!(refused(&mut s, "drawing.create", json!({})).contains("at most 1000 drawings"));
    assert!(Arc::ptr_eq(&s.doc, &full));
}

/// While a drawing is open the model's commands wait, and the drawing's run only while one is:
/// running a command, running it inside another and listing it all say so.
#[test]
fn model_and_drawing_commands_each_run_on_their_own_part() {
    let mut s = design();
    for id in ["drawing.inspect", "drawing.set", "drawing.properties", "drawing.properties.set", "drawing.properties.remove"] {
        assert!(refused(&mut s, id, json!({"name": "X", "key": "author", "value": "X"})).contains("open a drawing first"), "{id}");
    }
    run(&mut s, "drawing.create", json!({}));
    let doc = s.doc.clone();
    let boxed = json!({"length": 5, "width": 5, "height": 5});
    let e = refused(&mut s, "solid.box", boxed.clone());
    assert!(e.contains("a drawing is open: close it to change the model (drawing.close)"), "{e}");
    assert!(find_command("solid.box").unwrap().run_nested(&mut s, &boxed).is_err(), "nor inside another command");
    assert!(Arc::ptr_eq(&s.doc, &doc));
    let enabled = |s: &mut Session, id: &str| {
        let listed = run(s, "engine.commands", json!({}));
        listed.as_array().unwrap().iter().find(|c| c["id"] == id).map(|c| c["enabled"] == true).unwrap()
    };
    assert!(!enabled(&mut s, "solid.box") && enabled(&mut s, "drawing.set"));
    for id in ["edit.undo", "file.save", "file.new", "doc.open", "select.clear", "edit.checkpoint", "drawing.list", "drawing.close"] {
        assert!(find_command(id).unwrap().allowed(&s).is_ok(), "{id} runs with a drawing open");
    }
    run(&mut s, "drawing.close", json!({}));
    assert!(enabled(&mut s, "solid.box") && !enabled(&mut s, "drawing.set"));
    run(&mut s, "solid.box", boxed);
    // A new design or another opened has no drawing open.
    run(&mut s, "drawing.create", json!({}));
    run(&mut s, "file.new", json!({}));
    assert!(s.open_drawing.is_none() && s.doc.drawings.is_empty());
}

/// Each change of a drawing is one undo step of that drawing: undo and redo bring back the same
/// record, and a change to the same values is no step. No model is evaluated, and the design
/// changes, so whatever follows it sees each change.
#[test]
fn drawing_edits_are_undo_steps_of_the_drawing_that_evaluate_nothing() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({}));
    let (first, evaluations, features, steps) = (open_one(&s), s.evaluations, s.doc.features.clone(), s.undo.len());
    let (doc, revision) = (s.doc.clone(), s.revision);
    run(&mut s, "drawing.set", json!({"name": "Renamed", "language": "de", "units": "in"}));
    let renamed = open_one(&s);
    assert_eq!((renamed.name.as_str(), renamed.language.as_str()), ("Renamed", "de"));
    assert_eq!(renamed.units, Units::Fixed { unit: "in".into() });
    assert!(!Arc::ptr_eq(&s.doc, &doc) && s.revision > revision, "the design changed");
    assert_eq!((s.undo.len(), s.undo.last().map(|u| u.part)), (steps + 1, Some(Some(first.id))), "a step of the drawing");
    run(&mut s, "drawing.set", json!({"name": "Renamed"}));
    assert_eq!(s.undo.len(), steps + 1, "the same value again is no step");
    for _ in 0..100 {
        assert_eq!(run(&mut s, "edit.undo", json!({}))["undone"], "Drawing Settings");
        assert!(Arc::ptr_eq(&open_one(&s), &first));
        run(&mut s, "edit.redo", json!({}));
        assert!(Arc::ptr_eq(&open_one(&s), &renamed));
    }
    assert_eq!((s.evaluations, &s.doc.features), (evaluations, &features), "no model evaluated or changed");
}

/// Undo and redo go through the open part's steps only. With a drawing open, its own steps; the
/// model's stay where they are. With none open, the model's steps, which put the design back as
/// it was but keep each drawing it had as it is now.
#[test]
fn undo_and_redo_go_through_one_part_at_a_time() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({"name": "A"}));
    let id = open_one(&s).id;
    run(&mut s, "drawing.set", json!({"name": "B"}));
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "solid.box", json!({"length": 5, "width": 5, "height": 5}));
    let two = s.doc.features.len();
    run(&mut s, "drawing.open", the(id));
    run(&mut s, "drawing.set", json!({"name": "C"}));
    // In the drawing: C, B, A; the model's box stays.
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "edit.undo", json!({}));
    assert_eq!((open_one(&s).name.as_str(), s.doc.features.len()), ("A", two));
    assert!(refused(&mut s, "edit.undo", json!({})).contains("nothing to undo in this drawing (close it to undo in the model)"));
    // In the model: the box goes and the drawing stays as it is now (A, where the step had B);
    // then the drawing goes.
    run(&mut s, "drawing.close", json!({}));
    let evaluations = s.evaluations;
    run(&mut s, "edit.undo", json!({}));
    assert_eq!((s.doc.features.len(), names(&s)), (two - 1, vec!["A"]));
    assert!(s.evaluations > evaluations, "the model's step evaluates the model");
    run(&mut s, "edit.undo", json!({}));
    assert!(s.doc.drawings.is_empty());
    // Redo: the drawing as it was when it went, then the box, the drawing kept.
    run(&mut s, "edit.redo", json!({}));
    assert_eq!((names(&s), s.open_drawing), (vec!["A"], None));
    run(&mut s, "edit.redo", json!({}));
    assert_eq!((s.doc.features.len(), names(&s)), (two, vec!["A"]));
    assert_eq!(refused(&mut s, "edit.redo", json!({})), "nothing to redo");
    // The drawing's own redo steps were kept all along.
    run(&mut s, "drawing.open", the(id));
    run(&mut s, "edit.redo", json!({}));
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(open_one(&s).name, "C");
    assert!(refused(&mut s, "edit.redo", json!({})).contains("nothing to redo in this drawing"));
}

/// A new step ends what could be redone in its own part only: the others' redo steps still hold.
#[test]
fn a_new_step_ends_what_could_be_redone_in_its_part_only() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({"name": "A"}));
    let id = open_one(&s).id;
    run(&mut s, "drawing.set", json!({"name": "B"}));
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "solid.box", json!({"length": 5, "width": 5, "height": 5}));
    run(&mut s, "drawing.open", the(id));
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(open_one(&s).name, "B", "a model step leaves the drawing's redo");
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "drawing.set", json!({"name": "Z"}));
    assert!(refused(&mut s, "edit.redo", json!({})).contains("nothing to redo in this drawing"), "a drawing step ends its own");
    run(&mut s, "drawing.close", json!({}));
    let features = s.doc.features.len();
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "drawing.open", the(id));
    run(&mut s, "drawing.set", json!({"name": "Q"}));
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "edit.redo", json!({}));
    assert_eq!((s.doc.features.len(), names(&s)), (features, vec!["Q"]), "a drawing step leaves the model's redo");
}

/// A design lists its drawings by name, then id, whatever order they were made, renamed or
/// undone in.
#[test]
fn a_designs_drawings_are_listed_by_name_then_id() {
    let mut s = design();
    for name in ["Zeta", "Alpha", "Mid", "Alpha"] {
        run(&mut s, "drawing.create", json!({"name": name}));
    }
    assert_eq!(names(&s), ["Alpha", "Alpha", "Mid", "Zeta"]);
    assert!(s.doc.drawings[0].id < s.doc.drawings[1].id);
    let first = s.doc.drawings[0].id;
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "solid.box", json!({"length": 5, "width": 5, "height": 5}));
    run(&mut s, "drawing.open", the(first));
    run(&mut s, "drawing.set", json!({"name": "Zz"}));
    assert_eq!(names(&s), ["Alpha", "Mid", "Zeta", "Zz"]);
    let listed = run(&mut s, "drawing.list", json!({}));
    let open: Vec<bool> = listed["drawings"].as_array().unwrap().iter().map(|d| d["open"] == true).collect();
    assert_eq!(open, [false, false, false, true]);
    // The model's step, made before the rename, keeps the drawing renamed and in its place.
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), ["Alpha", "Mid", "Zeta", "Zz"]);
    run(&mut s, "drawing.open", the(first));
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), ["Alpha", "Alpha", "Mid", "Zeta"]);
}

/// Deleting a drawing is a step of the model: it closes it when it was open, and undo brings it
/// back as it was, closed. What isn't a drawing of the design is refused.
#[test]
fn deleting_a_drawing_is_a_step_of_the_model() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({"name": "A"}));
    let a = open_one(&s);
    run(&mut s, "drawing.create", json!({"name": "B"}));
    let b = open_one(&s);
    let doc = s.doc.clone();
    for (p, says) in [(json!({}), "must be a drawing's id"), (json!({"drawing": 7}), "must be a drawing's id"), (the(1), "has no drawing")] {
        let e = refused(&mut s, "drawing.delete", p.clone());
        assert!(e.contains(says), "{p}: {e}");
        assert!(Arc::ptr_eq(&s.doc, &doc) && s.open_drawing == Some(b.id));
    }
    assert_eq!(run(&mut s, "drawing.delete", the(b.id)), json!({"deleted": ids::encode(b.id)}));
    assert_eq!((names(&s), s.open_drawing), (vec!["A"], None));
    run(&mut s, "drawing.delete", the(a.id));
    assert!(s.doc.drawings.is_empty());
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "edit.undo", json!({}));
    assert!(Arc::ptr_eq(&s.doc.drawings[0], &a) && Arc::ptr_eq(&s.doc.drawings[1], &b), "back as they were");
    assert_eq!(s.open_drawing, None);
    assert!(refused(&mut s, "drawing.open", the(1)).contains("has no drawing"));
}

/// A refused change leaves the drawing as it was, with no step, and a refused opening leaves the
/// drawing open as it was.
#[test]
fn a_refused_change_leaves_the_drawing_as_it_was() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({}));
    let (before, steps) = (s.doc.clone(), s.undo.len());
    for p in [json!({"language": "e"}), json!({"name": ""}), json!({"units": "kg"}), json!({"name": "x".repeat(300)})] {
        refused(&mut s, "drawing.set", p.clone());
        assert!(Arc::ptr_eq(&s.doc, &before) && s.undo.len() == steps, "{p}");
    }
    let open = s.open_drawing;
    refused(&mut s, "drawing.open", json!({"drawing": "AAAAAAAAAAAAA"}));
    assert_eq!(s.open_drawing, open);
}

/// A checkpoint brings back the design's drawings, the one open and the undo history. Going back
/// over a change of the model and of a drawing evaluates the model: it is no drawing-only change.
#[test]
fn a_checkpoint_brings_back_the_drawings_and_the_one_open() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({"name": "A"}));
    let a = open_one(&s);
    let (checkpoint, steps, features) = (run(&mut s, "edit.checkpoint", json!({}))["checkpoint"].clone(), s.undo.len(), s.doc.features.clone());
    run(&mut s, "drawing.set", json!({"name": "B"}));
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10}));
    run(&mut s, "drawing.delete", the(a.id));
    run(&mut s, "drawing.create", json!({"name": "C"}));
    let evaluations = s.evaluations;
    run(&mut s, "edit.restore_checkpoint", json!({"checkpoint": checkpoint}));
    assert!(s.doc.drawings.len() == 1 && Arc::ptr_eq(&s.doc.drawings[0], &a));
    assert_eq!((s.open_drawing, s.undo.len(), &s.doc.features), (Some(a.id), steps, &features));
    assert!(s.evaluations > evaluations, "the model changed too, so it is evaluated");
}

/// A design's drawings are saved in its file and come back with it, as a container and as plain
/// JSON, with a drawing open when it is saved; a design opened has none open.
#[test]
fn a_designs_drawings_are_saved_with_it_and_come_back() {
    let folder = Folder::new("drawing-saved");
    let mut s = design();
    run(&mut s, "drawing.create", json!({"name": "A", "language": "en-GB"}));
    run(&mut s, "drawing.properties.set", json!({"values": {"author": "Jane Doe", "com.acme.shop": {"shop_id": 88}}}));
    run(&mut s, "drawing.create", json!({"name": "B", "units": "mm"}));
    let drawings = s.doc.drawings.clone();
    for file in ["plate.solidcraft", "plate.json"] {
        run(&mut s, "file.save", json!({"path": folder.path(file)}));
        assert!(!s.is_dirty(), "{file}");
        let mut opened = Session::default();
        run(&mut opened, "doc.open", json!({"path": folder.path(file)}));
        assert_eq!(opened.doc.drawings, drawings, "{file}");
        assert!(opened.open_drawing.is_none(), "{file}");
    }
}

/// A drawing's properties follow the design properties' rules: set, read and removed, each change
/// one undo step, other tools' keys kept, and what the design's would refuse refused.
#[test]
fn a_drawings_properties_follow_the_design_properties_rules() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({}));
    let steps = s.undo.len();
    let set = run(&mut s, "drawing.properties.set", json!({"values": {"author": "Jane Doe", "com.acme.shop": {"shop_id": 88}}}));
    assert_eq!(set["changed"], json!(["author", "com.acme.shop"]));
    assert_eq!(s.undo.len(), steps + 1);
    let read = run(&mut s, "drawing.properties", json!({}));
    assert_eq!((read["properties"]["author"].clone(), read["metadata"]["com.acme.shop"]["shop_id"].clone()), (json!("Jane Doe"), json!(88)));
    assert_eq!(read["properties"]["revision"], Value::Null);
    run(&mut s, "drawing.properties.set", json!({"key": "author", "value": "Jane Doe"}));
    assert_eq!(s.undo.len(), steps + 1, "the same value again is no change");
    let removed = run(&mut s, "drawing.properties.remove", json!({"keys": ["author", "missing"]}));
    assert_eq!(removed["removed"], json!(["author"]));
    assert_eq!((open_one(&s).properties.text("author"), s.undo.len()), (None, steps + 2));
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(open_one(&s).properties.text("author"), Some("Jane Doe"));
    assert!(s.doc.metadata.text("author").is_none(), "the design's own properties untouched");
    let before = open_one(&s);
    for (id, p, says) in [
        ("drawing.properties.set", json!({"key": "bom_type", "value": "purchased"}), "component.metadata.set"),
        ("drawing.properties.set", json!({"key": "author"}), "`value` is required"),
        ("drawing.properties.set", json!({"values": {"author": 7}}), "author"),
        ("drawing.properties.remove", json!({"keys": [1]}), "list of keys"),
    ] {
        let e = refused(&mut s, id, p.clone());
        assert!(e.contains(says), "{p}: {e}");
        assert!(Arc::ptr_eq(&open_one(&s), &before));
    }
}

/// Hostile values for each key of every command that runs with a drawing open, from the registry,
/// and as the whole params, each on a copy of one session with a drawing open (the shared pass
/// runs with none, where the part gate refuses them): an error or a result, never a panic, and the
/// design's drawings and the one open still keeping their rules. As in the shared pass, what
/// writes files is left to its own tests; the recovery commands never read a drawing.
#[test]
fn hostile_drawing_params_never_panic() {
    let mut s = design();
    run(&mut s, "drawing.create", json!({}));
    run(&mut s, "drawing.create", json!({"name": "Second"}));
    assert_eq!(s.scratch().open_drawing, s.open_drawing, "each copy has the drawing open");
    let keys = ["name", "language", "units", "drawing", "key", "value", "values", "keys", "checkpoint", "id"];
    let skipped = |id: &str| matches!(id, "doc.open" | "file.save" | "file.save_as") || id.starts_with("doc.recover");
    let mut internal = Vec::new();
    for spec in command_specs().iter().filter(|spec| spec.part != Part::Model && !skipped(spec.id)) {
        for value in crate::tests::hostile_values() {
            for p in keys.iter().map(|key| json!({ *key: value.clone() })).chain([value.clone()]) {
                let mut copy = s.scratch();
                if let Err(crate::EngineError::Internal(_, msg)) = copy.execute(spec.id, &p) {
                    internal.push(format!("{} {p}: {msg}", spec.id));
                }
                drawing::check_in_design(&copy.doc.drawings).unwrap();
                copy.doc.drawings.iter().for_each(|d| d.validate().unwrap());
                assert!(copy.open_drawing.is_none_or(|id| copy.doc.drawing(id).is_some()), "{} {p}", spec.id);
            }
        }
    }
    assert!(internal.is_empty(), "commands panicked with a drawing open:\n{}", internal.join("\n"));
}

/// Making, opening, closing, changing and deleting drawings, model edits, undo, redo and
/// checkpoints in any order: never a panic, and the design's drawings and the one open always
/// keep their rules.
#[test]
fn drawing_steps_in_any_order_keep_the_rules() {
    let mut state = 3890_u64;
    let mut next = move |n: u64| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) % n
    };
    for _ in 0..12 {
        let mut s = design();
        let mut checkpoints = Vec::new();
        for _ in 0..50 {
            let pick = next(8) as usize;
            let named = s.doc.drawings.get(pick % s.doc.drawings.len().max(1)).map_or(json!({"drawing": "AAAAAAAAAAAAA"}), |d| the(d.id));
            let (id, p) = match next(11) {
                0 => ("drawing.create", json!({"name": format!("D{}", next(4))})),
                1 => ("drawing.open", named),
                2 => ("drawing.close", json!({})),
                3 => ("drawing.set", json!({"name": format!("D{}", next(4))})),
                4 => ("drawing.delete", named),
                5 | 6 => ("edit.undo", json!({})),
                7 => ("edit.redo", json!({})),
                8 => ("solid.box", json!({"length": 1 + next(9), "width": 3, "height": 3})),
                9 => ("drawing.properties.set", json!({"key": "author", "value": format!("A{}", next(3))})),
                _ => match checkpoints.pop() {
                    Some(checkpoint) if next(2) == 0 => ("edit.restore_checkpoint", json!({"checkpoint": checkpoint})),
                    _ => ("edit.checkpoint", json!({})),
                },
            };
            let r = s.execute(id, &p);
            if id == "edit.checkpoint"
                && let Ok(v) = &r
            {
                checkpoints.push(v["checkpoint"].clone());
            }
            assert!(!matches!(r, Err(crate::EngineError::Internal(..))), "{id} {p}: {r:?}");
            drawing::check_in_design(&s.doc.drawings).unwrap();
            assert!(s.doc.drawings.windows(2).all(|w| (&w[0].name, w[0].id) < (&w[1].name, w[1].id)), "{id}: in their order");
            assert!(s.open_drawing.is_none_or(|id| s.doc.drawing(id).is_some()), "{id}: the drawing open is the design's");
        }
    }
}

/// The shared model-store port must not make a saved sheet sketch a model feature,
/// even when their numeric ids coincide. Drawing tools are not enabled by this port.
#[test]
fn sheet_records_do_not_redirect_the_shared_model_sketch_store() {
    use crate::cmd::sketch_store;
    use solidcraft_doc::drawing::SheetSketch;
    use solidcraft_geom::Vec2;
    use solidcraft_sketch::Sketch;

    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY", "name": "Model sketch"}));
    let model_id = s.active_sketch.unwrap();
    run(&mut s, "sketch.line", json!({"points": [[0, 0], [40, 0]]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "drawing.create", json!({"name": "Drawing", "units": "in"}));
    let mut ids = IdSource::seeded(6336);
    let separate = s.drawing("test").unwrap().allocate_id(&mut ids).unwrap();
    assert!(s.doc.feature(separate).is_none());
    s.edit_drawing("test", |drawing| {
        let mut sheet_sketch = Sketch::new();
        sheet_sketch.add_point(Vec2::new(15.0, 10.0), Some("sheet_point"))?;
        for id in [model_id, separate] {
            drawing.sketches.push(Arc::new(SheetSketch {
                id,
                sheet: drawing.sheets[0].id,
                name: "Sheet sketch".into(),
                sketch: sheet_sketch.clone(),
            }));
        }
        Ok(())
    })
    .unwrap();
    let drawing = open_one(&s);
    let before = s.doc.clone();
    let history = (s.undo.len(), s.redo.len(), s.evaluations);
    let model = sketch_store::read(&s.doc, model_id).unwrap();
    assert_eq!(model.curves.len(), 1);
    assert!(model.resolve_point("sheet_point").is_none());
    assert!(sketch_store::read(&s.doc, separate).is_err());
    assert!(sketch_store::target(&s, &json!({"sketch": separate}), "test").is_err());
    for command in ["sketch.edit", "sketch.line", "sketch.export_dxf", "sketch.solve"] {
        let error = refused(&mut s, command, json!({"sketch": model_id, "points": [[0, 0], [10, 0]]}));
        assert!(error.contains("a drawing is open"), "{command}: {error}");
        assert!(Arc::ptr_eq(&s.doc, &before));
        assert_eq!((s.undo.len(), s.redo.len(), s.evaluations), history);
    }
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "sketch.edit", json!({"sketch": model_id.to_string()}));
    run(&mut s, "sketch.point", json!({"point": [60, 5], "id": "model_point"}));
    assert!(sketch_store::read(&s.doc, model_id).unwrap().resolve_point("model_point").is_some());
    assert!(Arc::ptr_eq(s.doc.drawing(drawing.id).unwrap(), &drawing));
    assert!(drawing.sheet_sketch(model_id).unwrap().sketch.resolve_point("model_point").is_none());
}

/// Callback refusal inside the existing store never publishes its private clone.
#[test]
fn receiver_store_callback_failure_preserves_model_drawings_and_history() {
    use crate::cmd::sketch_store;
    use solidcraft_geom::Vec2;

    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY"}));
    let id = s.active_sketch.unwrap();
    run(&mut s, "sketch.line", json!({"points": [[0, 0], [40, 0]]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "drawing.create", json!({}));
    run(&mut s, "drawing.close", json!({}));
    let before = s.doc.clone();
    let state = s.model.state();
    let history = (s.undo.len(), s.redo.len(), s.evaluations);
    let result: Result<()> = sketch_store::update(&mut s, id, |_, _, sketch| {
        sketch.add_point(Vec2::new(15.0, 10.0), Some("not_committed"))?;
        Err(bad("test", "refuse after changing the private sketch"))
    });
    assert!(result.unwrap_err().to_string().contains("private sketch"));
    assert!(Arc::ptr_eq(&s.doc, &before));
    assert!(Arc::ptr_eq(&s.model.state(), &state));
    assert_eq!((s.undo.len(), s.redo.len(), s.evaluations), history);
    assert!(sketch_store::read(&s.doc, id).unwrap().resolve_point("not_committed").is_none());
}

// W38a R4: exercise real commands and Session history, not an alternative sketcher.
fn sheet_editor() -> (Session, u64, u64, Value) {
    let mut s = design();
    s.doc_mut().units = "in".into();
    let drawing = run(&mut s, "drawing.create", json!({"name": "Paper", "units": "in"}));
    let owner = drawing["sheets"][0]["id"].clone();
    let made = run(&mut s, "drawing.sketch.create", json!({"owner": owner, "name": "Detail"}));
    let d = ids::decode(made["drawing"].as_str().unwrap()).unwrap();
    let sk = ids::decode(made["sketch"].as_str().unwrap()).unwrap();
    (s, d, sk, made)
}

#[test]
fn sheet_editor_existing_tools_make_paper_geometry_without_evaluating_the_model() {
    use solidcraft_geom::Vec2;
    let (mut s, d, sk, made) = sheet_editor();
    let source = s.doc.clone();
    let model = s.model.state();
    let count = s.evaluations;
    let steps = s.undo.len();
    assert!(s.active_sketch.is_none(), "a sheet id is never a model active_sketch");
    assert_eq!(s.active_sheet_sketch.as_ref().unwrap().sketch, sk);
    let rectangle = run(&mut s, "sketch.rectangle.two_point", json!({"p0": [15,10], "p1": [55,30]}));
    assert_eq!(rectangle["created"]["curves"].as_array().unwrap().len(), 4);
    let record = s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap();
    assert_eq!(record.sketch.curves.len(), 4);
    for at in [[15.0, 10.0], [55.0, 10.0], [55.0, 30.0], [15.0, 30.0]] {
        assert!(record.sketch.points.iter().any(|p| p.pos.dist(Vec2::new(at[0], at[1])) < 1e-8));
    }
    run(&mut s, "sketch.line", json!({"points": [[70,10],[100,50]]}));
    let record = s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap();
    assert_eq!(record.sketch.curves.len(), 5);
    assert_eq!(s.undo.len(), steps + 2);
    assert_eq!(s.undo.last().unwrap().part, Some(d));
    assert_eq!((s.evaluations, &s.doc.features, &s.doc.params), (count, &source.features, &source.params));
    assert!(Arc::ptr_eq(&model, &s.model.state()));
    let inspected = run(&mut s, "drawing.sketch.inspect", json!({}));
    assert_eq!(inspected["sketch"], made["sketch"]);
    assert_eq!(inspected["curves"].as_array().unwrap().len(), 5);
    assert_eq!(s.undo.len(), steps + 2, "inspection adds no step");
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch.curves.len(), 4);
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch.curves.len(), 5);
    assert_eq!(s.evaluations, count);
}

#[test]
fn sheet_editor_point_and_line_expressions_keep_paper_defaults_and_read_only_source() {
    let (mut s, d, sk, _) = sheet_editor();
    s.doc_mut().set_param("Inset", "1", Some("in"), None).unwrap();
    s.doc_mut().set_param("Angle", "90", Some("deg"), None).unwrap();
    let original = s.doc.clone();
    let count = s.evaluations;
    run(&mut s, "sketch.point", json!({"point": ["Inset + 5", "2"], "id": "driven"}));
    run(&mut s, "sketch.line", json!({"points": [["Inset", "5"], ["Inset + 10", "5"]]}));
    let sketch = &s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch;
    let point = &sketch.points[sketch.resolve_point("driven").unwrap()];
    assert!((point.pos.x - 30.4).abs() < 1e-9);
    assert_eq!(point.pos.y, 2.0);
    assert_eq!(point.exprs.as_ref().unwrap().x.as_deref(), Some("Inset + 5"));
    assert!(sketch.points.iter().any(|p| (p.pos.x - 25.4).abs() < 1e-9 && p.pos.y == 5.0));
    assert!(sketch.points.iter().any(|p| (p.pos.x - 35.4).abs() < 1e-9 && p.pos.y == 5.0));
    assert_eq!((&s.doc.params, &s.doc.features), (&original.params, &original.features));
    for point in [json!(["Missing", 0]), json!(["Angle", 0]), json!(["1 / 0", 0])] {
        let before = s.doc.clone();
        let entry = s.active_sheet_sketch.clone();
        let history = (s.undo.len(), s.redo.len());
        refused(&mut s, "sketch.point", json!({"point": point}));
        assert!(Arc::ptr_eq(&before, &s.doc));
        assert_eq!(s.active_sheet_sketch, entry);
        assert_eq!((s.undo.len(), s.redo.len()), history);
    }
    assert_eq!(s.evaluations, count);
}

#[test]
fn sheet_editor_finish_cancel_and_their_undo_preserve_the_correct_entry() {
    let (mut s, d, sk, made) = sheet_editor();
    let count = s.evaluations;
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [15,10], "p1": [55,30]}));
    let finished = s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().clone();
    assert_eq!(run(&mut s, "sketch.finish", json!({}))["editing"], false);
    assert!(s.active_sheet_sketch.is_none());
    run(&mut s, "drawing.sketch.edit", json!({"sketch": made["sketch"]}));
    run(&mut s, "sketch.point", json!({"point": [90,60], "id": "temporary"}));
    let edited = s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().clone();
    // Re-entering the same sketch must not move the cancellation boundary.
    run(&mut s, "drawing.sketch.edit", json!({"sketch": made["sketch"]}));
    assert_eq!(run(&mut s, "sketch.cancel", json!({}))["removed"], false);
    assert!(Arc::ptr_eq(&finished, s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap()));
    assert!(s.active_sheet_sketch.is_none());
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch, edited.sketch);
    assert_eq!(s.active_sheet_sketch.as_ref().unwrap().sketch, sk);
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch, finished.sketch);
    assert!(s.active_sheet_sketch.is_none());
    let sheet = made["sheet"].clone();
    let new = run(&mut s, "drawing.sketch.create", json!({"owner": sheet}));
    let new_id = ids::decode(new["sketch"].as_str().unwrap()).unwrap();
    run(&mut s, "sketch.point", json!({"point": [5,7]}));
    assert_eq!(run(&mut s, "sketch.cancel", json!({}))["removed"], true);
    assert!(s.doc.drawing(d).unwrap().sheet_sketch(new_id).is_none());
    assert_eq!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch, finished.sketch);
    run(&mut s, "edit.undo", json!({}));
    assert!(s.doc.drawing(d).unwrap().sheet_sketch(new_id).is_some());
    run(&mut s, "edit.redo", json!({}));
    assert!(s.doc.drawing(d).unwrap().sheet_sketch(new_id).is_none());
    assert!(s.active_sheet_sketch.is_none());
    assert_eq!(s.evaluations, count);
}

#[test]
fn sheet_editor_checkpoint_scratch_and_reopen_keep_owners_separate() {
    let (mut s, d, sk, made) = sheet_editor();
    let count = s.evaluations;
    let checkpoint = run(&mut s, "edit.checkpoint", json!({}))["checkpoint"].clone();
    let before = s.doc.clone();
    let mut scratch = s.scratch();
    run(&mut scratch, "sketch.rectangle.two_point", json!({"p0": [15,10], "p1": [55,30]}));
    assert!(Arc::ptr_eq(&before, &s.doc));
    assert!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch.curves.is_empty());
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [15,10], "p1": [55,30]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "edit.restore_checkpoint", json!({"checkpoint": checkpoint}));
    assert!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch.curves.is_empty());
    assert_eq!(s.active_sheet_sketch.as_ref().unwrap().sketch, sk);
    run(&mut s, "sketch.line", json!({"points": [[15,10],[55,30]]}));
    run(&mut s, "sketch.finish", json!({}));
    let saved = s.doc.to_json();
    let document = solidcraft_doc::Document::from_json(&saved).unwrap();
    let mut reopened = Session::new(document);
    assert!(reopened.active_sheet_sketch.is_none());
    run(&mut reopened, "drawing.open", json!({"drawing": made["drawing"]}));
    run(&mut reopened, "drawing.sketch.edit", json!({"sketch": made["sketch"]}));
    let result = run(&mut reopened, "drawing.sketch.inspect", json!({}));
    assert_eq!(result["curves"].as_array().unwrap().len(), 1);
    assert_eq!(reopened.doc.to_json(), saved);
    assert_eq!(s.evaluations, count);
}

#[test]
fn sheet_editor_rejects_cross_targets_model_tools_and_source_mutation() {
    let (mut s, d, sk, made) = sheet_editor();
    let before = s.doc.clone();
    let entry = s.active_sheet_sketch.clone();
    let state = s.model.state();
    let history = (s.undo.len(), s.redo.len(), s.evaluations);
    let model = s.doc.features[0].id;
    assert_ne!(model, sk);
    for (command, params) in [
        ("sketch.point", json!({"sketch": model, "point": [2,3]})),
        ("sketch.point", json!({"sketch": -1, "point": [2,3]})),
        ("sketch.point", json!({"sketch": 1.5, "point": [2,3]})),
        ("sketch.line", json!({"points": ["vertex:1,2,3",[4,5]]})),
        ("sketch.dimension", json!({"entities": [], "value": "Forbidden = 3"})),
        ("sketch.project", json!({"refs": []})),
        ("sketch.create", json!({"plane": "XY"})),
        ("drawing.close", json!({})),
        ("drawing.open", json!({"drawing": made["drawing"]})),
        ("drawing.delete", json!({"drawing": made["drawing"]})),
        ("drawing.sketch.create", json!({"owner": made["sheet"]})),
    ] {
        refused(&mut s, command, params);
        assert!(Arc::ptr_eq(&s.doc, &before), "{command}");
        assert_eq!(s.active_sheet_sketch, entry, "{command}");
        assert_eq!((s.undo.len(), s.redo.len(), s.evaluations), history, "{command}");
    }
    let result = super::super::sketch_store::edit_sheet(&mut s, &json!({}), "test", |_sketch, doc| {
        doc.name = "Forbidden mutation".into();
        Ok(())
    });
    assert!(result.unwrap_err().to_string().contains("cannot change the source design"));
    assert!(Arc::ptr_eq(&s.doc, &before) && Arc::ptr_eq(&s.model.state(), &state));
    assert_eq!(s.open_drawing, Some(d));
}

#[test]
fn sheet_editor_new_commands_validate_hostile_inputs_and_declare_schemas() {
    let mut s = design();
    let drawing = run(&mut s, "drawing.create", json!({}));
    let before = s.doc.clone();
    let history = (s.undo.len(), s.redo.len(), s.evaluations);
    for params in [
        json!({}),
        json!({"owner": 1}),
        json!({"owner": "not-an-id"}),
        json!({"owner": drawing["id"]}),
        json!({"owner": drawing["sheets"][0]["id"], "name": " "}),
        json!({"owner": drawing["sheets"][0]["id"], "name": "X".repeat(257)}),
        json!({"owner": drawing["sheets"][0]["id"], "unexpected": true}),
        json!([]),
    ] {
        refused(&mut s, "drawing.sketch.create", params);
        assert!(Arc::ptr_eq(&s.doc, &before));
        assert!(s.active_sheet_sketch.is_none());
        assert_eq!((s.undo.len(), s.redo.len(), s.evaluations), history);
    }
    for command in ["drawing.sketch.create", "drawing.sketch.edit", "drawing.sketch.inspect", "sketch.cancel"] {
        let spec = find_command(command).unwrap();
        assert!(spec.contract.is_some() && spec.output.is_some(), "{command}");
    }
    s.param_check = crate::schema::ParamCheck::Strict;
    let made = run(&mut s, "drawing.sketch.create", json!({"owner": drawing["sheets"][0]["id"]}));
    assert!(made["editing"].as_bool().unwrap());
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "drawing.sketch.edit", json!({"sketch": made["sketch"]}));
    run(&mut s, "sketch.cancel", json!({}));
}

#[test]
fn sheet_editor_same_numeric_id_in_two_drawings_and_model_never_retargets_geometry() {
    use solidcraft_doc::drawing::SheetSketch;
    use solidcraft_sketch::Sketch;
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY", "name": "Model"}));
    let id = s.active_sketch.unwrap();
    run(&mut s, "sketch.line", json!({"points": [[0,0],[40,0]]}));
    run(&mut s, "sketch.finish", json!({}));
    let model = s.doc.sketch(id).unwrap().clone();
    run(&mut s, "drawing.create", json!({"name": "First"}));
    s.edit_drawing("test", |drawing| {
        drawing.sketches.push(Arc::new(SheetSketch { id, sheet: drawing.sheets[0].id, name: "Same id".into(), sketch: Sketch::new() }));
        Ok(())
    })
    .unwrap();
    let first = open_one(&s);
    let mut second = (*first).clone();
    second.id = first.allocate_id(&mut IdSource::seeded(7082)).unwrap();
    second.name = "Second".into();
    let second_id = second.id;
    s.doc_mut().put_drawing(Arc::new(second));
    let second = s.doc.drawing(second_id).unwrap().clone();
    let count = s.evaluations;
    run(&mut s, "drawing.sketch.edit", json!({"sketch": ids::encode(id)}));
    run(&mut s, "sketch.point", json!({"sketch": id, "point": [15,10], "id": "first_point"}));
    assert_eq!(s.doc.sketch(id).unwrap(), &model);
    assert!(Arc::ptr_eq(s.doc.drawing(second_id).unwrap(), &second));
    run(&mut s, "sketch.finish", json!({}));
    let first_edited = s.doc.drawing(first.id).unwrap().clone();
    run(&mut s, "drawing.open", the(second_id));
    run(&mut s, "drawing.sketch.edit", json!({"sketch": ids::encode(id)}));
    run(&mut s, "sketch.point", json!({"point": [50,20], "id": "second_point"}));
    assert_eq!(s.doc.sketch(id).unwrap(), &model);
    assert!(Arc::ptr_eq(s.doc.drawing(first.id).unwrap(), &first_edited));
    assert!(s.doc.drawing(second_id).unwrap().sheet_sketch(id).unwrap().sketch.resolve_point("first_point").is_none());
    run(&mut s, "edit.undo", json!({}));
    assert!(s.doc.drawing(second_id).unwrap().sheet_sketch(id).unwrap().sketch.resolve_point("second_point").is_none());
    assert!(Arc::ptr_eq(s.doc.drawing(first.id).unwrap(), &first_edited));
    assert_eq!(s.evaluations, count);
}

#[test]
fn sheet_editor_commands_and_refusals_do_not_evaluate_a_stale_model() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY"}));
    let model_sketch = s.active_sketch.unwrap();
    let line = run(&mut s, "sketch.line", json!({"points": [[0,0],[40,0]]}))["curves"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.box", json!({"length": 5,"width": 5,"height": 5}));
    run(&mut s, "sketch.edit", json!({"sketch": model_sketch}));
    run(&mut s, "sketch.drag", json!({"entity": format!("{line}.end"), "from": [40,0], "to": [45,3], "live": true}));
    assert!(s.model.is_stale(), "the fixture includes an unfinished model gesture");
    let model = s.model.state();
    let count = s.evaluations;
    let drawing = run(&mut s, "drawing.create", json!({}));
    run(&mut s, "drawing.sketch.create", json!({"owner": drawing["sheets"][0]["id"]}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [15,10], "p1": [55,30]}));
    run(&mut s, "drawing.sketch.inspect", json!({}));
    refused(&mut s, "solid.box", json!({"length": 3,"width": 3,"height": 3}));
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "edit.redo", json!({}));
    run(&mut s, "sketch.finish", json!({}));
    assert!(s.model.is_stale());
    assert!(Arc::ptr_eq(&model, &s.model.state()));
    assert_eq!(s.evaluations, count);
    assert_eq!(s.active_sketch, Some(model_sketch), "model active state was not overwritten");
}

// W38a review: an unchanged Arc is not a validation certificate.
#[test]
fn sheet_review_publication_revalidates_unchanged_sketch_records() {
    let (mut s, d, sk, _) = sheet_editor();
    let owner = s.doc.drawing(d).unwrap().clone();
    let mut next = (*owner).clone();
    let mut other = (*next.sheet_sketch(sk).unwrap().as_ref()).clone();
    other.id = next.allocate_id(&mut IdSource::seeded(7608)).unwrap();
    other.name = "Unchanged invalid payload".into();
    other.sketch.add_point(solidcraft_geom::Vec2::new(4.0, 5.0), Some("invalid_point")).unwrap();
    other.sketch.points[1].pos.x = solidcraft_sketch::MAX_COORD;
    let invalid = Arc::new(other);
    next.sketches.push(invalid.clone());
    s.doc_mut().put_drawing(Arc::new(next));
    let before = s.doc.clone();
    let model = s.model.state();
    let history = (s.undo.len(), s.redo.len(), s.evaluations);
    let e = s.execute("sketch.point", &json!({"point":[12,13],"id":"must_not_publish"})).unwrap_err().to_string();
    assert!(e.contains("out of range"), "{e}");
    assert!(Arc::ptr_eq(&s.doc, &before));
    assert!(Arc::ptr_eq(s.doc.drawing(d).unwrap().sheet_sketch(invalid.id).unwrap(), &invalid));
    assert!(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap().sketch.resolve_point("must_not_publish").is_none());
    assert_eq!((s.undo.len(), s.redo.len(), s.evaluations), history);
    assert!(Arc::ptr_eq(&s.model.state(), &model));
}

#[test]
fn sheet_review_shared_store_noop_keeps_the_document_and_record_arcs() {
    let (mut s, d, sk, _) = sheet_editor();
    let before = s.doc.clone();
    let record = before.drawing(d).unwrap().sheet_sketch(sk).unwrap().clone();
    let history = (s.undo.len(), s.redo.len(), s.evaluations);
    let (answer, _) = crate::cmd::sketch_store::edit_sheet(&mut s, &json!({}), "test", |_, _| Ok(42)).unwrap();
    assert_eq!(answer, 42);
    assert!(Arc::ptr_eq(&s.doc, &before));
    assert!(Arc::ptr_eq(s.doc.drawing(d).unwrap().sheet_sketch(sk).unwrap(), &record));
    assert_eq!((s.undo.len(), s.redo.len(), s.evaluations), history);
}
