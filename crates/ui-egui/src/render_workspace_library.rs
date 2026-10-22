//! Existing Render-panel library browser. Engine commands own catalog resolution,
//! assignments and saved resources; this view owns only search and request drafts.
use super::{ImageMethod, materials, setups};
use crate::{SolveApp, theme::Tokens, ui_kit};
use serde_json::{Value, json};
use solidcraft_engine::{doc::Document, geom::Vec3};
use std::sync::{Arc, Weak};

#[derive(Clone, Debug)]
pub(super) struct View {
    kind: String,
    query: String,
    entries: Vec<Value>,
    selected: Option<String>,
    details: Option<Value>,
    loaded: bool,
    offset: usize,
    total: usize,
    next: Option<usize>,
    target_query: String,
    targets: Vec<materials::Target>,
    target: Option<materials::Target>,
    target_revision: Option<u64>,
    target_document: Option<Weak<Document>>,
    acknowledge_projection: bool,
    replace_environment: bool,
    replace_studio: bool,
    center: [f64; 3],
    extent: f64,
}
impl Default for View {
    fn default() -> Self {
        Self {
            kind: "surfaces".into(),
            query: String::new(),
            entries: Vec::new(),
            selected: None,
            details: None,
            loaded: false,
            offset: 0,
            total: 0,
            next: None,
            target_query: String::new(),
            targets: Vec::new(),
            target: None,
            target_revision: None,
            target_document: None,
            acknowledge_projection: false,
            replace_environment: false,
            replace_studio: false,
            center: [0.0; 3],
            extent: 1000.0,
        }
    }
}

fn refresh(app: &mut SolveApp, view: &mut View) -> Result<(), String> {
    let reply = app.run("render.library.list", json!({"kind":view.kind,"query":view.query,"offset":view.offset,"limit":48}))?;
    view.entries = reply["items"].as_array().cloned().ok_or_else(|| "invalid render-library response".to_string())?;
    view.total = reply["total"].as_u64().and_then(|n| usize::try_from(n).ok()).unwrap_or(0);
    view.next = reply["next_offset"].as_u64().and_then(|n| usize::try_from(n).ok());
    view.loaded = true;
    view.selected = None;
    view.details = None;
    view.acknowledge_projection = false;
    Ok(())
}
fn current_target(app: &SolveApp, view: &View) -> bool {
    view.target.is_some()
        && view.target_revision == Some(app.session.revision)
        && view.target_document.as_ref().is_some_and(|d| Weak::ptr_eq(d, &Arc::downgrade(&app.session.doc)))
        && !app.session.model.is_stale()
}
fn refresh_targets(app: &mut SolveApp, view: &mut View) -> Result<(), String> {
    let reply = app.run("render.library.targets", json!({"query":view.target_query,"limit":128}))?;
    view.targets = serde_json::from_value(reply["items"].clone()).map_err(|e| e.to_string())?;
    view.target = None;
    view.target_revision = Some(app.session.revision);
    view.target_document = Some(Arc::downgrade(&app.session.doc));
    if let (Ok(min), Ok(max)) =
        (serde_json::from_value::<Vec3>(reply["bounds"]["min"].clone()), serde_json::from_value::<Vec3>(reply["bounds"]["max"].clone()))
    {
        let c = (min + max) * 0.5;
        let size = max - min;
        let extent = size.x.max(size.y).max(size.z);
        if extent.is_finite() && (0.001..=1e6).contains(&extent) {
            view.center = [c.x, c.y, c.z];
            view.extent = extent;
        }
    }
    Ok(())
}
fn target_controls(app: &mut SolveApp, ui: &mut egui::Ui, view: &mut View) {
    ui.label(app.tr("render.library_target"));
    ui_kit::bounded_cell_editor(
        ui,
        ui.id().with("render_library_target_search"),
        &mut view.target_query,
        app.tr("render.material_filter"),
        false,
        128,
    );
    if ui_kit::button(ui, app.tr("render.material_refresh"), ui_kit::Button::Normal).clicked()
        && let Err(e) = refresh_targets(app, view)
    {
        app.set_status(e, true);
    }
    let label = view.target.as_ref().map_or_else(|| app.tr("render.material_choose_body").to_string(), |t| t.name.clone());
    ui_kit::dropdown_menu(ui, "render_library_target", &label, |ui| {
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for target in &view.targets {
                if ui_kit::list_picker(ui, &format!("library-target:{}", target.ordinal), &target.name, view.target.as_ref() == Some(target))
                    .clicked()
                {
                    view.target = Some(target.clone());
                    ui.close();
                }
            }
        });
    });
    if view.target.is_some() && !current_target(app, view) {
        ui.small(app.tr("render.material_revalidate_needed"));
    }
}
fn patch(app: &SolveApp) -> Result<Value, String> {
    serde_json::to_value(setups::snapshot(app)?).map_err(|e| e.to_string())
}

pub(super) fn controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    saved_surface_controls(app, ui);
    let title = app.tr("render.library_title").to_string();
    ui_kit::section(ui, "render_resource_library", &title, false, |ui| {
        let mut view = std::mem::take(&mut app.render_workspace.view.library);
        let kinds = [
            ("surfaces", "render.library_surfaces"),
            ("selene_appearances", "render.library_selene"),
            ("hdr", "render.library_hdr"),
            ("rigs", "render.library_rigs"),
            ("scenes", "render.library_scenes"),
            ("materials", "render.library_material_concepts"),
            ("lighting", "render.library_light_concepts"),
            ("environments", "render.library_environment_concepts"),
            ("quality", "render.library_quality_concepts"),
        ];
        let label = kinds.iter().find(|(kind, _)| *kind == view.kind).map_or("", |(_, key)| app.tr(key)).to_string();
        ui_kit::dropdown_menu(ui, "render_library_kind", &label, |ui| {
            for (kind, key) in kinds {
                if ui_kit::list_picker(ui, kind, app.tr(key), view.kind == kind).clicked() {
                    view.kind = kind.into();
                    view.offset = 0;
                    view.loaded = false;
                    ui.close();
                }
            }
        });
        ui_kit::bounded_cell_editor(ui, ui.id().with("render_library_search"), &mut view.query, app.tr("render.library_search"), false, 128);
        if ui_kit::button(ui, app.tr("render.library_search"), ui_kit::Button::Normal).clicked() {
            view.offset = 0;
            view.loaded = false;
        }
        if !view.loaded
            && let Err(e) = refresh(app, &mut view)
        {
            view.loaded = true;
            app.set_status(e, true);
        }
        ui.small(format!("{}: {}", app.tr("render.library_results"), view.total));
        let mut choose = None;
        egui::ScrollArea::vertical().id_salt("render_library_items").max_height(210.0).show(ui, |ui| {
            for entry in &view.entries {
                let Some(id) = entry["id"].as_str() else { continue };
                let name = entry["name"].as_str().unwrap_or(id);
                let response = ui_kit::list_picker(ui, id, &format!("{id} · {name}"), view.selected.as_deref() == Some(id));
                crate::scenario::publish_handle(&format!("render:library:{id}"), response.rect.center());
                if response.clicked() {
                    choose = Some(id.to_string());
                }
            }
        });
        ui.horizontal(|ui| {
            if ui_kit::button(ui, app.tr("render.library_previous"), ui_kit::Button::Enabled(view.offset > 0)).clicked() {
                view.offset = view.offset.saturating_sub(48);
                view.loaded = false;
            }
            if ui_kit::button(ui, app.tr("render.library_next"), ui_kit::Button::Enabled(view.next.is_some())).clicked() {
                view.offset = view.next.unwrap_or(view.offset);
                view.loaded = false;
            }
        });
        if let Some(id) = choose {
            match app.run("render.library.describe", json!({"kind":view.kind,"id":id})) {
                Ok(reply) => {
                    view.details = Some(reply);
                    view.selected = Some(id);
                    view.acknowledge_projection = false;
                }
                Err(e) => app.set_status(e, true),
            }
        }
        if let Some(id) = view.selected.clone() {
            resource_controls(app, ui, &mut view, &id);
        }
        app.render_workspace.view.library = view;
    });
}

fn resource_controls(app: &mut SolveApp, ui: &mut egui::Ui, view: &mut View, id: &str) {
    let Some(detail) = view.details.clone() else { return };
    let source = &detail["resource"];
    if let Some(description) = source["description"].as_str().or_else(|| source["purpose"].as_str()) {
        ui.small(description);
    }
    ui.small(app.tr("render.library_authored_help"));
    match view.kind.as_str() {
        "surfaces" | "selene_appearances" => {
            target_controls(app, ui, view);
            ui_kit::checkbox(ui, app.tr("render.library_uniform_ack"), &mut view.acknowledge_projection);
            if ui_kit::button(
                ui,
                app.tr("render.library_apply_uniform"),
                ui_kit::Button::Enabled(current_target(app, view) && view.acknowledge_projection),
            )
            .clicked()
                && let Some(target) = view.target.clone()
            {
                match app.run("render.library.appearance.apply", json!({"id":id,"variant":"uniform_opaque","bodies":[target.name]})) {
                    Ok(_) => {
                        view.target_revision = None;
                        app.set_status(app.tr("render.library_applied").to_string(), false);
                    }
                    Err(e) => app.set_status(e, true),
                }
            }
            if view.kind == "surfaces" {
                ui.small(app.tr("render.library_maps_help"));
                let candidate = patch(app);
                let enabled = current_target(app, view) && app.render_workspace.view.method == ImageMethod::GpuRaytrace && candidate.is_ok();
                if ui_kit::button(ui, app.tr("render.library_save_maps"), ui_kit::Button::Enabled(enabled)).clicked()
                    && let (Some(target), Ok(set)) = (view.target.clone(), candidate)
                {
                    let name = app.render_workspace.view.name.clone();
                    match app.run("render.library.surface.bind",json!({"name":name,"set":set,"surface":{"id":id,"body_index":target.ordinal,"body_name":target.name},"replace":view.acknowledge_projection})) {
                        Ok(_)=>{setups::load(app);view.target_revision=None;app.set_status(app.tr("render.library_applied").to_string(),false);},
                        Err(e)=>app.set_status(e,true),
                    }
                }

                let enabled = current_target(app, view) && app.render_workspace.view.method == ImageMethod::GpuRaytrace;
                if ui_kit::button(ui, app.tr("render.library_apply_maps"), ui_kit::Button::Enabled(enabled)).clicked()
                    && let Some(target) = view.target.clone()
                {
                    match materials::install_library_surface(app, id, target) {
                        Ok(()) => app.set_status(app.tr("render.library_applied").to_string(), false),
                        Err(e) => app.set_status(e, true),
                    }
                }
            }
        }
        "hdr" => {
            ui.small(app.tr("render.library_hdr_help"));
            ui_kit::checkbox(ui, app.tr("render.library_replace_environment"), &mut view.replace_environment);
            let candidate = patch(app);
            if let Err(e) = &candidate {
                ui.small(e);
            }
            let enabled = candidate.is_ok() && app.render_workspace.view.method != ImageMethod::Raster;
            if ui_kit::button(ui, app.tr("render.library_apply_hdr"), ui_kit::Button::Enabled(enabled)).clicked()
                && let Ok(set) = candidate
            {
                let name = app.render_workspace.view.name.clone();
                match app
                    .run("render.library.environment.apply", json!({"id":id,"name":name,"set":set,"replace_environment":view.replace_environment}))
                {
                    Ok(_) => {
                        setups::load(app);
                        app.set_status(app.tr("render.library_applied").to_string(), false);
                    }
                    Err(e) => app.set_status(e, true),
                }
            }
        }
        "rigs" => {
            let blocked = detail["projection"]["status"] == "blocked";
            if blocked {
                ui.colored_label(Tokens::get().warning, app.tr("render.library_blocked"));
                return;
            }
            ui.small(app.tr("render.library_rig_help"));
            if ui_kit::button(ui, app.tr("render.library_scene_bounds"), ui_kit::Button::Normal).clicked()
                && let Err(e) = refresh_targets(app, view)
            {
                app.set_status(e, true);
            }
            ui.label(app.tr("render.library_center"));
            for c in &mut view.center {
                ui_kit::number_field(ui, c, -1e12..=1e12, 1.0, " mm");
            }
            ui.label(app.tr("render.library_extent"));
            ui_kit::number_field(ui, &mut view.extent, 0.001..=1e6, 1.0, " mm");
            let cards = detail["projection"]["passive_cards_omitted"].as_u64().unwrap_or(0);
            if cards > 0 {
                ui_kit::checkbox(ui, app.tr("render.library_emitters_only_ack"), &mut view.acknowledge_projection);
            }
            ui_kit::checkbox(ui, app.tr("render.library_replace_studio"), &mut view.replace_studio);
            let candidate = patch(app);
            let enabled = !blocked
                && (cards == 0 || view.acknowledge_projection)
                && view.replace_studio
                && candidate.is_ok()
                && app.render_workspace.view.method == ImageMethod::GpuRaytrace;
            if ui_kit::button(ui, app.tr("render.library_apply_studio"), ui_kit::Button::Enabled(enabled)).clicked()
                && let Ok(set) = candidate
            {
                let name = app.render_workspace.view.name.clone();
                let rig = json!({"id":id,"variant":if cards==0 {"complete"} else {"emitters_only"},"center_mm":view.center,"subject_extent_mm":view.extent});
                match app.run("render.library.studio.apply", json!({"rig":rig,"name":name,"set":set,"replace_lighting_and_environment":true})) {
                    Ok(_) => {
                        setups::load(app);
                        app.set_status(app.tr("render.library_applied").to_string(), false);
                    }
                    Err(e) => app.set_status(e, true),
                }
            }
        }
        _ => {
            ui.colored_label(Tokens::get().warning, app.tr("render.library_specification_only"));
        }
    }
}

fn saved_surface_controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    let title = app.tr("render.saved_surfaces").to_string();
    let bindings = app.render_workspace.view.draft.surfaces.clone();
    if bindings.is_empty() {
        return;
    }
    ui_kit::section(ui, "render_saved_surfaces", &title, false, |ui| {
        ui.small(app.tr("render.saved_surfaces_help"));
        for (index, binding) in bindings.iter().enumerate() {
            ui.label(format!(
                "{} - {} ({}): {:?}",
                binding.source.id, binding.target.definition, binding.target.feature, binding.target.occurrence_path
            ));
            let response = ui_kit::button(ui, app.tr("render.saved_surface_remove"), ui_kit::Button::Normal);
            crate::scenario::publish_handle(&format!("render:saved-surface-remove:{index}"), response.rect.center());
            if response.clicked() {
                let name = app.render_workspace.view.name.clone();
                match patch(app).and_then(|set| app.run("render.library.surface.unbind", json!({"name":name,"target":binding.target,"set":set}))) {
                    Ok(_) => setups::load(app),
                    Err(e) => app.set_status(e, true),
                }
            }
        }
    });
}
