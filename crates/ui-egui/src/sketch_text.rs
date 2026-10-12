//! Edit the source of fixed text outlines through the existing engine command.
use serde_json::json;
use solvecraft_engine::{
    Sel,
    sketch::{LinkKind, LinkSource},
};

use crate::SolveApp;

pub(crate) struct Editor {
    sketch: u64,
    tab: usize,
    revision: u64,
    link: String,
    text: String,
    values: [String; 4],
    focus: bool,
    error: Option<String>,
}

/// Only a complete selection belonging to one text source can seed the editor.
pub(crate) fn selected_entity(app: &SolveApp) -> Option<String> {
    let sk = app.session.doc.sketch(app.session.active_sketch?).ok()?;
    let mut selected = None;
    let mut link = None;
    for selection in &app.session.selection {
        let Sel::SketchCurve { id } = selection else { return None };
        let curve = sk.curves.iter().find(|curve| &curve.id == id)?;
        let source = sk.link(curve.link.as_deref()?)?;
        if source.kind != LinkKind::Text || link.is_some_and(|old| old != source.id.as_str()) {
            return None;
        }
        link = Some(source.id.as_str());
        selected = Some(id.clone());
    }
    selected
}

pub(crate) fn open(app: &mut SolveApp, entity: &str) {
    let Some(sketch) = app.session.active_sketch else { return };
    let Ok(sk) = app.session.doc.sketch(sketch) else { return };
    let source = sk.curves.iter().find(|curve| curve.id == entity).and_then(|curve| curve.link.as_deref()).and_then(|id| sk.link(id));
    let Some(source) = source.filter(|source| source.kind == LinkKind::Text) else {
        app.set_status("Choose an outline belonging to sketch text", true);
        return;
    };
    let LinkSource::Text { text, at, height, angle } = &source.source else { return };
    app.text_editor = Some(Editor {
        sketch,
        tab: app.docs.active,
        revision: app.session.revision,
        link: source.id.clone(),
        text: text.clone(),
        values: [at.x.to_string(), at.y.to_string(), height.to_string(), angle.to_degrees().to_string()],
        focus: true,
        error: None,
    });
    app.tool = None;
}

pub(crate) fn show(app: &mut SolveApp, ctx: &egui::Context) {
    let Some(mut editor) = app.text_editor.take() else { return };
    if app.session.active_sketch != Some(editor.sketch) || app.docs.active != editor.tab || app.session.revision != editor.revision {
        return;
    }
    let mut apply = false;
    let mut cancel = false;
    crate::frame::window(ctx, "Edit Text", crate::frame::Width::Normal).collapsible(false).resizable(false).show(ctx, |ui| {
        ui.label("Text");
        let response = ui.add(egui::TextEdit::multiline(&mut editor.text).id_salt("sketch_text_content").desired_rows(3));
        crate::scenario::publish_handle("text:content", response.rect.center());
        if editor.focus {
            response.request_focus();
            editor.focus = false;
        }
        for (index, label) in ["X (mm)", "Y (mm)", "Height (mm)", "Angle (deg)"].iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(*label);
                if let Some(value) = editor.values.get_mut(index) {
                    let response = ui.add(egui::TextEdit::singleline(value).id_salt(("sketch_text_value", index)).desired_width(120.0));
                    crate::scenario::publish_handle(&format!("text:{index}"), response.rect.center());
                }
            });
        }
        if let Some(error) = &editor.error {
            ui.colored_label(crate::theme::Tokens::get().error, error);
        }
        ui.horizontal(|ui| {
            apply = ui.button("Apply text").clicked();
            cancel = ui.button("Cancel text").clicked() || ui.input(|input| input.key_pressed(egui::Key::Escape));
        });
    });
    if cancel {
        return;
    }
    if apply {
        let values = editor.values.each_ref().map(|value| value.parse::<f64>().ok().filter(|number| number.is_finite()));
        if let [Some(x), Some(y), Some(height), Some(angle)] = values {
            match app.run("sketch.edit_text", json!({"link":editor.link,"text":editor.text,"at":[x,y],"height":height,"angle":angle})) {
                Ok(_) => {
                    let _ = app.session.execute("select.clear", &json!({}));
                    return;
                }
                Err(error) => editor.error = Some(error),
            }
        } else {
            editor.error = Some("Enter finite numbers; positions and height are in millimetres.".into());
        }
    }
    app.text_editor = Some(editor);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Harness;

    fn setup() -> (Harness, String, String) {
        let mut h = Harness::new();
        h.app.home.open = false;
        h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
        let result = h.app.run("sketch.text", json!({"text":"I","at":[0,0],"height":10})).unwrap();
        let link = result["link"].as_str().unwrap().to_owned();
        let curve = result["curves"][0].as_str().unwrap().to_owned();
        h.app.session.selection = vec![Sel::SketchCurve { id: curve.clone() }];
        h.frames(3);
        (h, link, curve)
    }

    fn replace(h: &mut Harness, handle: &str, value: &str) {
        h.step(&json!({"click":{"handle":handle}})).unwrap();
        h.step(&json!({"key":"A","cmd":true})).unwrap();
        h.step(&json!({"text":value})).unwrap();
    }

    #[test]
    fn selected_text_menu_edits_source_position_and_size_with_one_undo_and_reopen() {
        let (mut h, link, _) = setup();
        let before = h.app.session.doc.to_json();
        let undo = h.app.session.undo.len();
        let item = crate::context_menu::items(&h.app, &crate::context_menu::Target::Viewport)
            .into_iter()
            .find(|item| item.id == "sketch.edit_text")
            .unwrap();
        crate::context_menu::run_item(&mut h.app, &item, egui::Pos2::ZERO);
        h.frames(3);
        assert!(h.app.text_editor.is_some());
        for (field, value) in [("text:content", "O"), ("text:0", "25"), ("text:1", "12"), ("text:2", "20"), ("text:3", "90")] {
            replace(&mut h, field, value);
        }
        assert_eq!(h.app.session.doc.to_json(), before);
        h.step(&json!({"widget":"Apply text"})).unwrap();
        assert!(h.app.text_editor.is_none());
        assert_eq!(h.app.session.undo.len(), undo + 1);
        let sid = h.app.session.active_sketch.unwrap();
        let sk = h.app.session.doc.sketch(sid).unwrap();
        let expected = LinkSource::Text {
            text: "O".into(),
            at: solvecraft_engine::geom::Vec2::new(25.0, 12.0),
            height: 20.0,
            angle: std::f64::consts::FRAC_PI_2,
        };
        assert_eq!(sk.link(&link).unwrap().source, expected);
        assert!(!h.app.session.world_state().sketch(sid).unwrap().profiles.is_empty());
        let changed = h.app.session.doc.to_json();
        let reopened = solvecraft_engine::Session::new(solvecraft_engine::doc::Document::from_json(&changed).unwrap());
        assert_eq!(reopened.doc.sketch(sid).unwrap().link(&link).unwrap().source, expected);
        h.app.run("edit.undo", json!({})).unwrap();
        assert_eq!(h.app.session.doc.to_json(), before);
        h.app.run("edit.redo", json!({})).unwrap();
        assert_eq!(h.app.session.doc.to_json(), changed);
    }

    #[test]
    fn invalid_text_values_and_cancel_leave_document_and_history_unchanged() {
        let (mut h, _, _) = setup();
        let before = (h.app.session.doc.to_json(), h.app.session.undo.len());
        h.app.start("sketch.edit_text");
        h.frames(3);
        for value in ["NaN", "inf", "-5", "0"] {
            replace(&mut h, "text:2", value);
            h.step(&json!({"widget":"Apply text"})).unwrap();
            assert!(h.app.text_editor.as_ref().unwrap().error.is_some());
            assert_eq!((h.app.session.doc.to_json(), h.app.session.undo.len()), before);
        }
        h.step(&json!({"widget":"Cancel text"})).unwrap();
        assert!(h.app.text_editor.is_none());
        assert_eq!((h.app.session.doc.to_json(), h.app.session.undo.len()), before);
    }

    #[test]
    fn text_editor_rejects_mixed_sources_and_leaves_with_document_or_command() {
        let (mut h, _, curve) = setup();
        let other = h.app.run("sketch.text", json!({"text":"O","at":[30,0],"height":10})).unwrap();
        h.app.session.selection.push(Sel::SketchCurve { id: other["curves"][0].as_str().unwrap().into() });
        assert!(selected_entity(&h.app).is_none());
        assert!(!crate::context_menu::items(&h.app, &crate::context_menu::Target::Viewport).iter().any(|item| item.id == "sketch.edit_text"));
        h.app.session.selection = vec![Sel::SketchCurve { id: curve }];
        h.app.start("sketch.edit_text");
        assert!(h.app.text_editor.is_some());
        h.app.start("sketch.line");
        assert!(h.app.text_editor.is_none());
        h.app.start("sketch.edit_text");
        assert!(h.app.text_editor.is_some());
        crate::documents::open_tab(&mut h.app, solvecraft_engine::Session::default()).unwrap();
        assert!(h.app.text_editor.is_none());
    }

    #[test]
    fn edit_text_without_preselection_picks_a_real_outline() {
        let (mut h, link, _) = setup();
        h.app.session.selection.clear();
        h.call("ui.view", json!({"view":"top"}));
        h.app.cam.distance = 40.0;
        h.frames(3);
        let sid = h.app.session.active_sketch.unwrap();
        let polyline = h.app.session.doc.sketch(sid).unwrap().polyline(0);
        let midpoint = (polyline[0] + polyline[1]) * 0.5;
        h.step(&json!({"start":"sketch.edit_text"})).unwrap();
        assert!(h.app.tool.is_some());
        h.step(&json!({"click":{"sketch":[midpoint.x,midpoint.y]}})).unwrap();
        assert_eq!(h.app.text_editor.as_ref().unwrap().link, link);
        assert!(h.app.tool.is_none());
        h.step(&json!({"widget":"Cancel text"})).unwrap();
    }

    #[test]
    fn issue_report_preserves_text_edit_and_owns_keyboard_input() {
        let (mut h, _, _) = setup();
        h.app.start("sketch.edit_text");
        h.frames(3);
        replace(&mut h, "text:content", "Pending text");
        let before = (h.app.session.doc.to_json(), h.app.session.undo.len());
        h.app.help.menu = true;
        h.frames(2);
        h.step(&json!({"click":{"handle":"menu:report"}})).unwrap();
        h.frames(3);
        let token = h.app.shot_token;
        crate::issue_report::receive(&mut h.app, token, std::sync::Arc::new(egui::ColorImage::new([2, 2], vec![egui::Color32::GREEN; 4])));
        h.frames(3);
        let focus = egui::Id::new("issue_report_title");
        h.ctx.memory_mut(|memory| memory.request_focus(focus));
        h.frames(2);
        assert_eq!(h.ctx.memory(|memory| memory.focused()), Some(focus));
        h.step(&json!({"text":"report-only-marker"})).unwrap();
        h.step(&json!({"key":"Escape"})).unwrap();
        assert_eq!(h.app.text_editor.as_ref().unwrap().text, "Pending text");
        assert_eq!((h.app.session.doc.to_json(), h.app.session.undo.len()), before);
        h.step(&json!({"widget":"Close draft"})).unwrap();
        h.step(&json!({"widget":"Apply text"})).unwrap();
        assert!(h.app.text_editor.is_none());
        assert_eq!(h.app.session.undo.len(), before.1 + 1);
    }
}
