//! GPU studio editors. Numeric intent is portable; HDR resources cross the engine capture boundary.
use serde_json::{Value, json};
use solidcraft_engine::render::job::{AreaLight, AreaLights, StudioLighting, Sun, ThinLens, ToneMap};
use solidcraft_engine::render::lighting::environment::SkyGradient;

use super::hdr;
use crate::{SolveApp, ui_kit};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Environment {
    #[default]
    None,
    Sky,
    Hdr,
}

#[derive(Clone, Debug)]
pub(super) struct View {
    pub tone_map: ToneMap,
    pub denoise: bool,
    pub lens_enabled: bool,
    pub lens: ThinLens,
    pub environment: Environment,
    pub hdr: hdr::Input,
    pub assume_linear_srgb: bool,
    pub rotation_rad: f64,
    pub gradient: SkyGradient,
    pub intensity: f64,
    pub lighting_enabled: bool,
    pub ambient: [f64; 3],
    pub sun_enabled: bool,
    pub sun: Sun,
    pub area_lights: Vec<AreaLight>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            tone_map: ToneMap::Reinhard,
            denoise: false,
            lens_enabled: false,
            lens: ThinLens { aperture_radius_mm: 1.0, focus_distance_mm: 100.0 },
            environment: Environment::None,
            hdr: hdr::Input::default(),
            assume_linear_srgb: false,
            rotation_rad: 0.0,
            gradient: SkyGradient::default(),
            intensity: 1.0,
            lighting_enabled: false,
            ambient: [0.5; 3],
            sun_enabled: false,
            sun: Sun { direction: [-0.4, -0.6, 0.7], angular_radius_rad: 0.02, radiance: [1000.0, 950.0, 850.0] },
            area_lights: Vec::new(),
        }
    }
}

pub(super) fn params(app: &SolveApp, params: &mut Value) -> Result<(), String> {
    let view = &app.render_workspace.view.studio;
    params["tone_map"] = json!(view.tone_map);
    params["denoise"] = json!(view.denoise);
    if view.lens_enabled {
        view.lens.validate().map_err(|error| error.to_string())?;
        if view.lens.aperture_radius_mm > 0.0 && app.cam.fov == 0.0 {
            return Err(app.tr("render.studio_lens_perspective").to_string());
        }
        params["lens"] = json!(view.lens);
    }
    if view.lighting_enabled {
        let lighting = StudioLighting {
            environment: view.ambient,
            sun: view.sun_enabled.then_some(view.sun),
            area_lights: AreaLights::new(&view.area_lights).map_err(|error| error.to_string())?,
        };
        lighting.validate().map_err(|error| error.to_string())?;
        params["lighting"] = json!(lighting);
    }
    if view.environment != Environment::None {
        if !view.intensity.is_finite() || !(0.0..=100.0).contains(&view.intensity) || !view.rotation_rad.is_finite() || view.rotation_rad.abs() > 1e6
        {
            return Err(app.tr("render.studio_environment_bounds").to_string());
        }
        let mut environment = json!({"kind":"sky","rotation_rad":view.rotation_rad,"intensity":view.intensity});
        if view.environment == Environment::Hdr {
            environment = hdr::request(app, hdr::Target::Gpu)?;
            environment["rotation_rad"] = json!(view.rotation_rad);
            environment["intensity"] = json!(view.intensity);
            environment["assume_linear_srgb"] = json!(view.assume_linear_srgb);
        }
        if view.environment == Environment::Sky {
            environment["gradient"] = json!(view.gradient);
        }
        params["environment"] = environment;
    }
    Ok(())
}

fn vector(ui: &mut egui::Ui, label: &str, value: &mut [f64; 3], range: std::ops::RangeInclusive<f64>, speed: f64) {
    ui.label(label);
    ui.horizontal(|ui| {
        for component in value {
            ui_kit::number_field(ui, component, range.clone(), speed, "");
        }
    });
}

pub(super) fn finishing_controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    ui.label(app.tr("render.studio_tone_map"));
    let selected = app.render_workspace.view.studio.tone_map;
    let label = match selected {
        ToneMap::Reinhard => "Reinhard",
        ToneMap::AcesFitted => "ACES fitted",
    };
    ui_kit::dropdown_menu(ui, "render_tone_map", label, |ui| {
        for (map, label) in [(ToneMap::Reinhard, "Reinhard"), (ToneMap::AcesFitted, "ACES fitted")] {
            if ui_kit::list_picker(ui, label, label, map == selected).clicked() {
                app.render_workspace.view.studio.tone_map = map;
                ui.close();
            }
        }
    });
    let denoise = app.tr("render.studio_denoise").to_string();
    ui_kit::checkbox(ui, &denoise, &mut app.render_workspace.view.studio.denoise);
    if app.render_workspace.view.studio.denoise {
        ui.small(app.tr("render.studio_denoise_help"));
    }
}

pub(super) fn lens_controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    let lens = app.tr("render.studio_lens").to_string();
    ui_kit::checkbox(ui, &lens, &mut app.render_workspace.view.studio.lens_enabled);
    if app.render_workspace.view.studio.lens_enabled {
        ui.label(app.tr("render.studio_aperture"));
        ui_kit::number_field(ui, &mut app.render_workspace.view.studio.lens.aperture_radius_mm, 0.0..=1e7, 0.1, " mm");
        ui.label(app.tr("render.studio_focus"));
        ui_kit::number_field(ui, &mut app.render_workspace.view.studio.lens.focus_distance_mm, 1e-6..=1e9, 1.0, " mm");
    }
}

pub(super) fn controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    finishing_controls(app, ui);
    lens_controls(app, ui);
    let title = app.tr("render.studio_environment").to_string();
    ui_kit::section(ui, "render_environment", &title, false, |ui| environment_controls(app, ui));
    let title = app.tr("render.studio_lighting").to_string();
    ui_kit::section(ui, "render_studio_lighting", &title, false, |ui| lighting_controls(app, ui));
    ui.small(app.tr("render.studio_runtime_help"));
}

pub(super) fn environment_controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    let variants =
        [(Environment::None, "render.studio_environment_none"), (Environment::Sky, "render.studio_sky"), (Environment::Hdr, "render.studio_hdr")];
    let selected = app.render_workspace.view.studio.environment;
    let key = match selected {
        Environment::None => "render.studio_environment_none",
        Environment::Sky => "render.studio_sky",
        Environment::Hdr => "render.studio_hdr",
    };
    let label = app.tr(key).to_string();
    ui_kit::dropdown_menu(ui, "render_environment_kind", &label, |ui| {
        for (value, key) in variants {
            if ui_kit::list_picker(ui, key, app.tr(key), value == selected).clicked() {
                app.render_workspace.view.studio.environment = value;
                ui.close();
            }
        }
    });
    if app.render_workspace.view.studio.environment == Environment::None {
        return;
    }
    if app.render_workspace.view.studio.environment == Environment::Hdr {
        hdr::controls(app, ui, hdr::Target::Gpu);
        let assume = app.tr("render.studio_hdr_srgb").to_string();
        ui_kit::checkbox(ui, &assume, &mut app.render_workspace.view.studio.assume_linear_srgb);
        ui.small(app.tr("render.studio_hdr_help"));
    } else {
        ui.small(app.tr("render.studio_sky_help"));
        for (index, key) in [(0, "render.sky_zenith"), (1, "render.sky_horizon"), (2, "render.sky_ground")] {
            let label = app.tr(key).to_string();
            let gradient = &mut app.render_workspace.view.studio.gradient;
            let field = match index {
                0 => &mut gradient.zenith,
                1 => &mut gradient.horizon,
                _ => &mut gradient.ground,
            };
            let mut editable = field.map(f64::from);
            let before = editable;
            vector(ui, &label, &mut editable, 0.0..=1e6, 0.01);
            if editable != before {
                *field = editable.map(|value| value as f32);
            }
        }
    }
    ui.label(app.tr("render.studio_intensity"));
    ui_kit::number_field(ui, &mut app.render_workspace.view.studio.intensity, 0.0..=100.0, 0.1, "");
    ui.label(app.tr("render.studio_rotation"));
    let mut degrees = app.render_workspace.view.studio.rotation_rad.to_degrees();
    if ui_kit::number_field(ui, &mut degrees, -1e6_f64.to_degrees()..=1e6_f64.to_degrees(), 1.0, "°").changed() {
        app.render_workspace.view.studio.rotation_rad = degrees.to_radians();
    }
}

pub(super) fn lighting_controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    let label = app.tr("render.studio_lighting_enable").to_string();
    ui_kit::checkbox(ui, &label, &mut app.render_workspace.view.studio.lighting_enabled);
    if !app.render_workspace.view.studio.lighting_enabled {
        return;
    }
    let ambient = app.tr("render.studio_ambient_rgb").to_string();
    vector(ui, &ambient, &mut app.render_workspace.view.studio.ambient, 0.0..=1e6, 0.1);
    let label = app.tr("render.studio_sun").to_string();
    ui_kit::checkbox(ui, &label, &mut app.render_workspace.view.studio.sun_enabled);
    if app.render_workspace.view.studio.sun_enabled {
        let direction = app.tr("render.studio_direction").to_string();
        vector(ui, &direction, &mut app.render_workspace.view.studio.sun.direction, -1e8..=1e8, 0.01);
        let radiance = app.tr("render.studio_radiance").to_string();
        vector(ui, &radiance, &mut app.render_workspace.view.studio.sun.radiance, 0.0..=1e8, 1.0);
        ui.label(app.tr("render.studio_sun_radius"));
        let mut radius = app.render_workspace.view.studio.sun.angular_radius_rad.to_degrees();
        if ui_kit::number_field(ui, &mut radius, 0.001_f64.to_degrees()..=90.0, 0.05, "°").changed() {
            app.render_workspace.view.studio.sun.angular_radius_rad = radius.to_radians();
        }
    }
    let title = app.tr("render.studio_area").to_string();
    let mut remove = None;
    for index in 0..app.render_workspace.view.studio.area_lights.len() {
        ui_kit::section(ui, ("render_area", index), &format!("{title} {}", index + 1), false, |ui| {
            for (key, field) in [("render.studio_center", 0), ("render.studio_half_u", 1), ("render.studio_half_v", 2), ("render.studio_radiance", 3)]
            {
                let label = app.tr(key).to_string();
                let Some(light) = app.render_workspace.view.studio.area_lights.get_mut(index) else { continue };
                let value = match field {
                    0 => &mut light.center,
                    1 => &mut light.u,
                    2 => &mut light.v,
                    _ => &mut light.radiance,
                };
                vector(
                    ui,
                    &label,
                    value,
                    if field == 3 {
                        0.0..=1e6
                    } else if field == 0 {
                        -1e12..=1e12
                    } else {
                        -1e6..=1e6
                    },
                    1.0,
                );
            }
            if ui_kit::button(ui, app.tr("render.studio_remove_area"), ui_kit::Button::Normal).clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        app.render_workspace.view.studio.area_lights.remove(index);
    }
    if ui_kit::button(ui, app.tr("render.studio_add_area"), ui_kit::Button::Enabled(app.render_workspace.view.studio.area_lights.len() < 4)).clicked()
    {
        let target = app.cam.target;
        let size = (app.cam.distance * 0.25).clamp(1.0, 1e5);
        app.render_workspace.view.studio.area_lights.push(AreaLight {
            center: [target.x, target.y, target.z + size * 2.0],
            u: [size, 0.0, 0.0],
            v: [0.0, -size, 0.0],
            radiance: [8.0; 3],
        });
    }
    ui.small(app.tr("render.studio_area_help"));
}
