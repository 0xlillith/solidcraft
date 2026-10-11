use serde_json::{Value, json};

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

fn measure(s: &mut Session) -> Value {
    run(s, "inspect.measure", json!({}))
}

fn volume(s: &mut Session) -> f64 {
    measure(s)["total"]["volume_mm3"].as_f64().unwrap_or(f64::NAN)
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-12)
}

/// An L bracket: a 40 x 20 x 5 base and a 5 x 20 x 40 wall at x = 0..5.
fn bracket() -> Session {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 40, "width": 20, "height": 5, "corner": [0, 0, 0], "body_name": "L"}));
    run(&mut s, "solid.box", json!({"length": 5, "width": 20, "height": 40, "corner": [0, 0, 0], "operation": "join"}));
    assert!(rel(volume(&mut s), 4000.0 + 3500.0) < 1e-9);
    // Mid-plane of the bracket (y = 10): sketch x = X, sketch y = Z.
    run(&mut s, "sketch.create", json!({"plane": {"origin": [0, 10, 0], "x_dir": [1, 0, 0], "y_dir": [0, 0, 1]}, "name": "RibSketch"}));
    run(&mut s, "sketch.line", json!({"points": [[5, 25], [25, 5]], "ids": ["l1"]}));
    run(&mut s, "sketch.finish", json!({}));
    s
}

#[test]
fn rib_to_the_next_face_fills_the_corner() {
    let mut s = bracket();
    run(&mut s, "solid.rib", json!({"sketch": "RibSketch", "curves": ["l1"], "thickness": 4}));
    // The triangle (5,5)-(5,25)-(25,5) of area 200, 4 thick.
    assert!(rel(volume(&mut s), 7500.0 + 800.0) < 1e-6, "{}", volume(&mut s));
    assert_eq!(measure(&mut s)["body_count"], 1);
    // The thickness is a parameter.
    let t = s.doc.features.last().and_then(|f| f.param_names.first().cloned()).unwrap_or_default();
    run(&mut s, "parameters.change", json!({"name": t, "expression": "2"}));
    assert!(rel(volume(&mut s), 7500.0 + 400.0) < 1e-6);
}

#[test]
fn rib_with_a_depth_and_web() {
    let mut s = bracket();
    run(&mut s, "solid.rib", json!({"sketch": "RibSketch", "curves": ["l1"], "thickness": 4, "depth": 2}));
    let v = volume(&mut s);
    assert!(v > 7500.0 + 100.0 && v < 7500.0 + 800.0, "{v}");
    let mut s = bracket();
    run(&mut s, "solid.web", json!({"sketch": "RibSketch", "curves": ["l1"], "thickness": 4}));
    assert!(rel(volume(&mut s), 8300.0) < 1e-6);
    // Curves that face no body.
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10}));
    run(&mut s, "sketch.create", json!({"plane": "XY", "name": "Far"}));
    run(&mut s, "sketch.line", json!({"points": [[50, 50], [60, 60]], "ids": ["l1"]}));
    run(&mut s, "sketch.finish", json!({}));
    let e = s.execute("solid.rib", &json!({"sketch": "Far", "curves": ["l1"], "thickness": 1}));
    assert!(e.is_err_and(|e| e.to_string().contains("do not face a body")));
}

#[test]
fn emboss_and_deboss_on_a_face() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 40, "width": 30, "height": 10}));
    run(&mut s, "sketch.create", json!({"plane": {"face": [20, 15, 10]}, "name": "Logo"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [5, 5], "p1": [15, 15]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.emboss", json!({"sketch": "Logo", "depth": 2}));
    assert!(rel(volume(&mut s), 12000.0 + 200.0) < 1e-6, "{}", volume(&mut s));
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "solid.emboss", json!({"sketch": "Logo", "depth": 3, "mode": "deboss"}));
    assert!(rel(volume(&mut s), 12000.0 - 300.0) < 1e-6);
    assert!(s.execute("solid.emboss", &json!({"sketch": "Logo", "depth": 1, "mode": "sideways"})).is_err());
}

#[test]
fn replace_face_align_remove() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "A"}));
    run(&mut s, "solid.replace_face", json!({"faces": [[5, 5, 10]], "target": {"origin": [0, 0, 15], "normal": [0, 0, 1]}}));
    assert!(rel(volume(&mut s), 1500.0) < 1e-6);
    let e = s.execute("solid.replace_face", &json!({"faces": [[5, 5, 15]], "target": {"origin": [0, 0, 15], "normal": [1, 0, 1]}}));
    assert!(e.is_err() || s.model.results.last().is_some_and(|r| r.error.as_deref().unwrap_or("").contains("parallel")));
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "A"}));
    run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10, "corner": [50, 0, 0], "body_name": "B"}));
    // B's top face onto A's top face: B turns over and sits on A.
    run(&mut s, "solid.align", json!({"bodies": ["B"], "from_face": [55, 5, 10], "to_face": [5, 5, 10]}));
    let m = measure(&mut s);
    let b = m["bodies"].as_array().unwrap().iter().find(|b| b["name"] == "B").unwrap().clone();
    let (lo, hi) = (b["bbox"]["min"].clone(), b["bbox"]["max"].clone());
    for (got, want) in [(&lo[0], 0.0), (&lo[2], 10.0), (&hi[0], 10.0), (&hi[2], 20.0)] {
        assert!((got.as_f64().unwrap_or(f64::NAN) - want).abs() < 1e-6, "{b}");
    }
    // Points only: a plain move.
    run(&mut s, "solid.align", json!({"bodies": ["B"], "from": [0, 0, 10], "to": [0, 0, 30]}));
    let m = measure(&mut s);
    let b = m["bodies"].as_array().unwrap().iter().find(|b| b["name"] == "B").unwrap().clone();
    assert!((b["bbox"]["min"][2].as_f64().unwrap_or(0.0) - 30.0).abs() < 1e-6, "{b}");
    run(&mut s, "solid.remove", json!({"bodies": ["B"]}));
    assert_eq!(measure(&mut s)["body_count"], 1);
}

#[test]
fn coil_and_loft_to_a_point() {
    use std::f64::consts::{PI, TAU};
    let mut s = Session::default();
    run(&mut s, "solid.coil", json!({"diameter": 40, "revolutions": 2, "pitch": 8, "section_size": 4}));
    assert!(rel(volume(&mut s), PI * 4.0 * TAU * 20.0 * 2.0) < 1e-3, "{}", volume(&mut s));
    // Height and revolutions give the pitch; a square section outside the diameter.
    let mut s = Session::default();
    run(
        &mut s,
        "solid.coil",
        json!({"diameter": 40, "revolutions": 1.5, "height": 15, "section": "square", "section_size": 2, "section_position": "outside", "axis": "X"}),
    );
    assert!(rel(volume(&mut s), 4.0 * TAU * 21.0 * 1.5) < 1e-3, "{}", volume(&mut s));
    assert!(s.execute("solid.coil", &json!({"diameter": 40, "pitch": 1, "revolutions": 3, "section_size": 4})).is_err(), "runs into itself");
    assert!(s.execute("solid.coil", &json!({"diameter": 40, "section_size": 4, "pitch": 8})).is_err(), "two of three");
    // Pyramid: a square lofted to a point.
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY", "name": "Base"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [-10, -10], "p1": [10, 10]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "sketch.create", json!({"plane": {"origin": [0, 0, 30], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}, "name": "Top"}));
    run(&mut s, "sketch.point", json!({"point": [0, 0], "id": "apex"}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.loft", json!({"sections": [{"sketch": "Base"}, {"sketch": "Top", "point": "apex"}]}));
    assert!(rel(volume(&mut s), 400.0 * 30.0 / 3.0) < 1e-6);
    assert!(s.execute("solid.loft", &json!({"sections": [{"sketch": "Base"}, {"sketch": "Top", "point": "nope"}]})).is_err());
}

/// A 3 mm shelled basin (100 x 60 x 30, open top) with a fall lofted from its whole outline
/// 10 up down to a drain pad (`pad`: corners, 1 up), in either section order. The loft's top
/// edges lie on the outer walls; the cut used to fail for one order of the sections (an
/// `Option::unwrap()` panic inside the kernel, or "shell not oriented and closed").
fn basin_fall(pad: [f64; 4], reverse: bool, op: &str) -> (Session, std::result::Result<Value, String>) {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 100, "width": 60, "height": 30, "corner": [0, 0, 0]}));
    run(&mut s, "solid.shell", json!({"faces": [[50, 30, 30]], "thickness": 3}));
    run(&mut s, "sketch.create", json!({"plane": {"origin": [0, 0, 10], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}, "name": "Rim"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [0, 0], "p1": [100, 60]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "sketch.create", json!({"plane": {"origin": [0, 0, 1], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}, "name": "Pad"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [pad[0], pad[1]], "p1": [pad[2], pad[3]]}));
    run(&mut s, "sketch.finish", json!({}));
    let mut secs = vec![json!({"sketch": "Rim"}), json!({"sketch": "Pad"})];
    if reverse {
        secs.reverse();
    }
    let r = s.execute("solid.loft", &json!({"sections": secs, "operation": op})).map_err(|e| e.to_string());
    (s, r)
}

#[test]
fn loft_cut_through_a_shelled_basin_in_either_order() {
    // What is left after the cut: the basin less the loft's part of the floor and walls
    // (integrated over the height), the same for a pad in any corner.
    let want = 41582.028;
    for pad in [[80.0, 40.0, 90.0, 50.0], [10.0, 40.0, 20.0, 50.0], [10.0, 10.0, 20.0, 20.0], [80.0, 10.0, 90.0, 20.0]] {
        for reverse in [false, true] {
            let (mut s, r) = basin_fall(pad, reverse, "cut");
            assert!(r.is_ok(), "{pad:?} reverse {reverse}: {r:?}");
            let v = volume(&mut s);
            assert!((v - want).abs() < 1.0, "{pad:?} reverse {reverse}: {v} vs {want}");
            assert_eq!(measure(&mut s)["body_count"], 1);
        }
    }
    // As a new body and joined: a valid solid of positive volume either way.
    for op in ["new", "join"] {
        let mut vols = Vec::new();
        for reverse in [false, true] {
            let (mut s, r) = basin_fall([80.0, 40.0, 90.0, 50.0], reverse, op);
            assert!(r.is_ok(), "{op} reverse {reverse}: {r:?}");
            vols.push(volume(&mut s));
        }
        assert!(vols[0] > 42948.0 && rel(vols[0], vols[1]) < 1e-6, "{op}: {vols:?}");
    }
}
