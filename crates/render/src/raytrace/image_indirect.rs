//! One prepaid opaque path continuation. No ray queue, recursion or image storage.
use super::*;
use crate::raytrace::indirect::{self, Rectangle, multiply, power_weight};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    New,
    Trace,
    Emitters,
    Resolve,
    Effects,
    Emission,
    Directional,
    Punctual,
    Area,
    Environment,
    Shadow,
    ShadowEmitters,
    Add,
    Scatter,
    Commit,
}
pub(super) struct State {
    stage: Stage,
    ray: Option<Ray>,
    cursor: Option<TraceCursor>,
    hit: Option<super::super::Hit>,
    emitter: Option<(usize, f64, Vec3)>,
    emitter_cursor: usize,
    effect_cursor: usize,
    effect: Option<usize>,
    emission_cursor: usize,
    light: usize,
    sample: u32,
    depth: u8,
    throughput: Vec3,
    radiance: Vec3,
    previous_pdf: f64,
    previous_position: Vec3,
    contribution: Vec3,
    after_shadow: Stage,
    shadow_ray: Option<Ray>,
    shadow_emitter: usize,
    shadow_excluded: Option<usize>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            stage: Stage::New,
            ray: None,
            cursor: None,
            hit: None,
            emitter: None,
            emitter_cursor: 0,
            effect_cursor: 0,
            effect: None,
            emission_cursor: 0,
            light: 0,
            sample: 0,
            depth: 0,
            throughput: Vec3::new(1.0, 1.0, 1.0),
            radiance: Vec3::ZERO,
            previous_pdf: 0.0,
            previous_position: Vec3::ZERO,
            contribution: Vec3::ZERO,
            after_shadow: Stage::Directional,
            shadow_ray: None,
            shadow_emitter: 0,
            shadow_excluded: None,
        }
    }
}
#[cfg(test)]
impl State {
    pub(super) fn stage_name(&self) -> &'static str {
        match self.stage {
            Stage::New => "new",
            Stage::Trace => "trace",
            Stage::Emitters => "emitters",
            Stage::Resolve => "resolve",
            Stage::Effects => "effects",
            Stage::Emission => "emission",
            Stage::Directional => "directional",
            Stage::Punctual => "punctual",
            Stage::Area => "area",
            Stage::Environment => "environment",
            Stage::Shadow => "shadow",
            Stage::ShadowEmitters => "shadow_emitters",
            Stage::Add => "add",
            Stage::Scatter => "scatter",
            Stage::Commit => "commit",
        }
    }
}
fn variates(task: &ImageTask, state: &State, domain: u64) -> job::Result<[f64; 2]> {
    let p = task.progressive.as_ref().ok_or_else(|| invalid("CPU path needs progressive film"))?;
    Ok(progressive::variates(
        p.options.seed,
        task.pixel as u64,
        p.film.samples(),
        (0x5041_5448_0000_u64 + u64::from(state.depth) * 0x10000).wrapping_add(domain),
    ))
}
fn continuing(task: &ImageTask, state: &State) -> job::Result<bool> {
    Ok(state.depth + 1 < task.options.path.ok_or_else(|| invalid("CPU path settings disappeared"))?.max_depth)
}
fn shared<'a>(task: &'a ImageTask, state: &State) -> job::Result<super::super::prepared::SharedHit<'a>> {
    let bvh = task.bvh.as_ref().ok_or_else(|| invalid("CPU path BVH missing"))?;
    let scene = PreparedScene { bvh, primitives: &task.looks, source: &task.scene };
    let mut hit = scene
        .resolve_hit(state.ray.ok_or_else(|| invalid("CPU path ray missing"))?, state.hit.ok_or_else(|| invalid("CPU path hit missing"))?)
        .map_err(error)?;
    if let Some(index) = state.effect {
        let (_, effect) = task.effects.get(index).ok_or_else(|| invalid("CPU path effect missing"))?;
        let changed = effect.apply(hit.surface, hit.shading_normal, hit.oriented_geometric_normal(), hit.position)?;
        hit.surface = changed.surface;
        hit.shading_normal = changed.shading_normal;
    }
    Ok(hit)
}
fn response(task: &ImageTask, state: &State, direction: Vec3) -> job::Result<Vec3> {
    let hit = shared(task, state)?;
    let view = -state.ray.ok_or_else(|| invalid("CPU path ray missing"))?.direction();
    if hit.oriented_geometric_normal().dot(direction) <= 0.0 || hit.oriented_geometric_normal().dot(view) <= 0.0 {
        return Ok(Vec3::ZERO);
    }
    hit.surface
        .brdf(hit.shading_normal, view, direction)
        .map(|f| f * hit.shading_normal.dot(direction).max(0.0))
        .map_err(|_| invalid("CPU path direct response"))
}
fn light_weight(task: &ImageTask, state: &State, direction: Vec3, pdf: f64, samples: u32) -> job::Result<f64> {
    if !continuing(task, state)? {
        return Ok(1.0);
    }
    let hit = shared(task, state)?;
    let view = -state.ray.ok_or_else(|| invalid("CPU path ray missing"))?.direction();
    let bsdf = indirect::surface_pdf(hit.surface, hit.shading_normal, view, direction).map_err(error)?;
    power_weight(f64::from(samples) * pdf, bsdf).map_err(error)
}
fn queue(task: &ImageTask, state: &mut State, value: Vec3, ray: Option<Ray>, next: Stage) -> job::Result<()> {
    state.contribution = multiply(state.throughput, value);
    super::super::lighting::validate_radiance(state.contribution).map_err(error)?;
    state.after_shadow = next;
    state.shadow_excluded = None;
    if task.options.shadows
        && value != Vec3::ZERO
        && let Some(ray) = ray
    {
        state.shadow_ray = Some(ray);
        state.shadow_emitter = 0;
        state.cursor = Some(TraceCursor::new(ray, true));
        state.stage = Stage::Shadow;
    } else {
        state.stage = Stage::Add;
    }
    Ok(())
}
fn bounded_shadow(hit: super::super::prepared::SharedHit<'_>, point: Vec3, direction: Vec3) -> job::Result<Option<Ray>> {
    let origin = hit.spawn(direction).map_err(error)?.origin();
    let delta = point - origin;
    let distance = delta.len();
    if delta.dot(direction) <= 0.0 || !distance.is_finite() || distance.next_down() <= 0.0_f64.next_up() {
        return Ok(None);
    }
    // Geometry wins equal-distance model/emitter ties in Stage::Emitters. Keep
    // the sampled endpoint in model visibility too; only the analytic source
    // itself is excluded separately from the emitter-occluder scan.
    Ray::new(origin, delta, 0.0_f64.next_up(), distance).map(Some).map_err(error)
}

pub(super) fn step(task: &mut ImageTask) -> job::Result<()> {
    // Taking this fixed slot avoids aliasing the source scene with the mutable worker.
    // Its exact capacity was reserved before transport; returning it cannot allocate.
    let mut state = task.path.pop().ok_or_else(|| invalid("CPU path continuation missing"))?;
    if state.stage == Stage::Commit {
        task.finish_sample(state.radiance)?;
        task.path.push(State::default());
        return Ok(());
    }
    let result = advance(task, &mut state);
    task.path.push(state);
    result
}
fn advance(task: &mut ImageTask, state: &mut State) -> job::Result<()> {
    if state.stage == Stage::Trace || state.stage == Stage::Shadow {
        let bvh = task.bvh.as_ref().ok_or_else(|| invalid("CPU path BVH missing"))?;
        let cursor = state.cursor.as_mut().ok_or_else(|| invalid("CPU path traversal cursor missing"))?;
        let Some(hit) = cursor.step(bvh, &mut task.work, &task.cancel).map_err(error)? else {
            return Ok(());
        };
        state.cursor = None;
        if state.stage == Stage::Shadow {
            if hit.is_some() {
                state.contribution = Vec3::ZERO;
                state.stage = Stage::Add;
            } else {
                state.stage = Stage::ShadowEmitters;
            }
        } else {
            state.hit = hit;
            state.emitter = None;
            state.emitter_cursor = 0;
            state.stage = Stage::Emitters;
        }
        return Ok(());
    }
    task.work.tick(&task.cancel).map_err(error)?;
    match state.stage {
        Stage::New => {
            let p = task.progressive.as_ref().ok_or_else(|| invalid("CPU path progressive film missing"))?;
            if task.options.lens.is_some_and(|l| l.aperture_radius_mm > 0.0) && task.pupil_sample.is_none() {
                task.pupil_sample = Some(progressive::variates(p.options.seed, task.pixel as u64, p.film.samples(), 0x4c454e53));
                return Ok(());
            }
            let [dx, dy] = progressive::variates(p.options.seed, task.pixel as u64, p.film.samples(), 0);
            let x = (task.pixel % task.request.width as usize) as f64 + dx;
            let y = (task.pixel / task.request.width as usize) as f64 + dy;
            let ray = if let Some(lens) = task.options.lens.filter(|l| l.aperture_radius_mm > 0.0) {
                lens_primary_ray(&task.request, x, y, task.clip, lens, task.pupil_sample.take().ok_or_else(|| invalid("CPU path pupil missing"))?)
                    .map_err(error)?
            } else {
                primary_ray(&task.request, x, y, task.clip).map_err(error)?
            };
            state.ray = Some(ray);
            state.cursor = Some(TraceCursor::new(ray, false));
            state.stage = Stage::Trace;
        }
        Stage::Emitters => {
            if let Some(input) = task.options.area_lights.as_ref().and_then(|v| v.get(state.emitter_cursor)) {
                let light = Rectangle::new(input).map_err(error)?;
                if let Some((distance, position)) = light.intersect(state.ray.ok_or_else(|| invalid("CPU path ray missing"))?).map_err(error)?
                    && state.hit.is_none_or(|hit| distance < hit.geometry.distance)
                    && state.emitter.is_none_or(|(_, prior, _)| distance < prior)
                {
                    state.emitter = Some((state.emitter_cursor, distance, position));
                }
                state.emitter_cursor += 1;
            } else {
                state.stage = Stage::Resolve;
            }
        }
        Stage::Resolve => {
            if let Some((index, _, position)) = state.emitter {
                let input = task.options.area_lights.as_ref().and_then(|v| v.get(index)).ok_or_else(|| invalid("CPU path emitter disappeared"))?;
                let light = Rectangle::new(input).map_err(error)?;
                let weight = if state.depth == 0 {
                    1.0
                } else {
                    power_weight(
                        state.previous_pdf,
                        f64::from(task.options.area_samples.unwrap_or(16)) * light.pdf(state.previous_position, position).map_err(error)?,
                    )
                    .map_err(error)?
                };
                state.radiance += multiply(state.throughput, light.radiance) * weight;
                if state.depth == 0 {
                    task.primary_coverage = 1;
                }
                state.stage = Stage::Commit;
            } else if state.hit.is_some() {
                if state.depth == 0 {
                    task.primary_coverage = 1;
                }
                state.effect_cursor = 0;
                state.effect = None;
                state.emission_cursor = 0;
                state.stage = Stage::Effects;
            } else {
                // A camera miss remains the separate unexposed display background.
                if state.depth > 0 {
                    let direction = state.ray.ok_or_else(|| invalid("CPU escaped path ray missing"))?.direction();
                    let (radiance, pdf, n) = if let Some(environment) = &task.environment {
                        let env = environment.view().map_err(environment_error)?;
                        (
                            env.radiance_linear_srgb(direction).map_err(environment_error)?,
                            env.pdf_solid_angle(direction).map_err(environment_error)?,
                            environment.samples,
                        )
                    } else if let Some(environment) = task.options.environment {
                        (
                            Vec3::new(environment.color[0], environment.color[1], environment.color[2]) * environment.intensity,
                            indirect::ENVIRONMENT_PDF,
                            1,
                        )
                    } else {
                        (Vec3::ZERO, 0.0, 0)
                    };
                    let weight = power_weight(state.previous_pdf, f64::from(n) * pdf).map_err(error)?;
                    state.radiance += multiply(state.throughput, radiance) * weight;
                }
                state.stage = Stage::Commit;
            }
            super::super::lighting::validate_radiance(state.radiance).map_err(error)?;
        }
        Stage::Effects => {
            let hit = shared(task, state)?;
            if let Some((body, _)) = task.effects.get(state.effect_cursor) {
                if *body == hit.body_index {
                    state.effect = Some(state.effect_cursor);
                    state.effect_cursor = task.effects.len();
                } else {
                    state.effect_cursor += 1;
                }
            } else {
                state.stage = Stage::Emission;
            }
        }
        Stage::Emission => {
            if let Some((body, emission)) = task.emissions.get(state.emission_cursor) {
                let hit = shared(task, state)?;
                if *body == hit.body_index {
                    state.radiance += multiply(state.throughput, emission.radiance(hit.front_face));
                }
                state.emission_cursor += 1;
                super::super::lighting::validate_radiance(state.radiance).map_err(error)?;
            } else {
                state.light = 0;
                state.stage = Stage::Directional;
            }
        }
        Stage::Directional => {
            if let Some(light) = task.lights.lights().get(state.light) {
                let value = multiply(response(task, state, light.direction())?, light.color()) * light.intensity();
                let ray = shared(task, state)?.spawn(light.direction()).map_err(error)?;
                state.light += 1;
                queue(task, state, value, Some(ray), Stage::Directional)?;
            } else {
                state.light = 0;
                state.stage = Stage::Punctual;
            }
        }
        Stage::Punctual => {
            if let Some(input) = task.options.punctual_lights.as_ref().and_then(|v| v.get(state.light)) {
                let hit = shared(task, state)?;
                let sample = input.light().map_err(error)?.sample(hit.position).map_err(error)?;
                state.light += 1;
                if let Some(why) = sample.omitted {
                    match why {
                        super::super::punctual::OmittedSample::NearSource => {
                            task.punctual_diagnostics.near_source = task.punctual_diagnostics.near_source.saturating_add(1)
                        }
                        super::super::punctual::OmittedSample::RadianceLimit => {
                            task.punctual_diagnostics.radiance_limit = task.punctual_diagnostics.radiance_limit.saturating_add(1)
                        }
                    }
                } else {
                    let value = multiply(response(task, state, sample.direction)?, sample.incident_linear_srgb);
                    if super::super::lighting::validate_radiance(value).is_err() {
                        task.punctual_diagnostics.radiance_limit = task.punctual_diagnostics.radiance_limit.saturating_add(1);
                    } else if let Some(ray) = sample.shadow_ray(hit).map_err(error)? {
                        queue(task, state, value, Some(ray), Stage::Punctual)?;
                    } else {
                        task.punctual_diagnostics.shadow_offset_crossed = task.punctual_diagnostics.shadow_offset_crossed.saturating_add(1);
                    }
                }
            } else {
                state.light = 0;
                state.sample = 0;
                state.stage = Stage::Area;
            }
        }
        Stage::Area => {
            if let Some(input) = task.options.area_lights.as_ref().and_then(|v| v.get(state.light)) {
                let count = task.options.area_samples.unwrap_or(16);
                if state.sample == count {
                    state.light += 1;
                    state.sample = 0;
                    return Ok(());
                }
                let side = match count {
                    1 => 1,
                    4 => 2,
                    16 => 4,
                    _ => 8,
                };
                let [dx, dy] = variates(
                    task,
                    state,
                    (0x1000 + state.light as u64 * 128 + u64::from(state.sample)) ^ (u64::from(task.options.seed.unwrap_or(0)) << 32),
                )?;
                let u = [
                    indirect::open_stratum(dx, state.sample / side, side).map_err(error)?,
                    indirect::open_stratum(dy, state.sample % side, side).map_err(error)?,
                ];
                let hit = shared(task, state)?;
                let light = Rectangle::new(input).map_err(error)?;
                state.sample += 1;
                if let Some(sample) = light.sample(hit.position, u).map_err(error)? {
                    let weight = light_weight(task, state, sample.direction, sample.pdf, count)?;
                    let value = multiply(response(task, state, sample.direction)?, light.radiance) * (weight / (f64::from(count) * sample.pdf));
                    if let Some(ray) = bounded_shadow(hit, sample.position, sample.direction)? {
                        queue(task, state, value, Some(ray), Stage::Area)?;
                        state.shadow_excluded = Some(state.light);
                    }
                }
            } else {
                state.sample = 0;
                state.stage = Stage::Environment;
            }
        }
        Stage::Environment => {
            let count = task.environment.as_ref().map_or_else(|| u32::from(task.options.environment.is_some()), |e| e.samples);
            if state.sample == count {
                state.stage = Stage::Scatter;
                return Ok(());
            }
            let side = match count {
                1 => 1,
                4 => 2,
                16 => 4,
                _ => 8,
            };
            let [dx, dy] = variates(task, state, 0x2000 + u64::from(state.sample))?;
            let u = [
                indirect::open_stratum(dx, state.sample / side, side).map_err(error)?,
                indirect::open_stratum(dy, state.sample % side, side).map_err(error)?,
            ];
            let (direction, emitted, pdf) = if let Some(environment) = &task.environment {
                let env = environment.view().map_err(environment_error)?;
                env.require_linear_srgb().map_err(environment_error)?;
                let sample = env.sample(u).map_err(environment_error)?;
                (sample.direction, sample.radiance_source_rgb, sample.pdf_solid_angle)
            } else {
                let environment = task.options.environment.ok_or_else(|| invalid("CPU path environment missing"))?;
                (
                    indirect::uniform_environment(u),
                    Vec3::new(environment.color[0], environment.color[1], environment.color[2]) * environment.intensity,
                    indirect::ENVIRONMENT_PDF,
                )
            };
            let weight = light_weight(task, state, direction, pdf, count)?;
            let value = multiply(response(task, state, direction)?, emitted) * (weight / (f64::from(count) * pdf));
            let ray = shared(task, state)?.spawn(direction).map_err(error)?;
            state.sample += 1;
            queue(task, state, value, Some(ray), Stage::Environment)?;
        }
        Stage::ShadowEmitters => {
            if let Some(input) = task.options.area_lights.as_ref().and_then(|v| v.get(state.shadow_emitter)) {
                let ray = state.shadow_ray.ok_or_else(|| invalid("CPU path finite shadow ray missing"))?;
                if state.shadow_excluded != Some(state.shadow_emitter)
                    && Rectangle::new(input).map_err(error)?.intersect(ray).map_err(error)?.is_some()
                {
                    state.contribution = Vec3::ZERO;
                    state.stage = Stage::Add;
                } else {
                    state.shadow_emitter += 1;
                }
            } else {
                state.stage = Stage::Add;
            }
        }
        Stage::Add => {
            state.radiance += state.contribution;
            super::super::lighting::validate_radiance(state.radiance).map_err(error)?;
            state.stage = state.after_shadow;
        }
        Stage::Scatter => {
            if !continuing(task, state)? {
                state.stage = Stage::Commit;
                return Ok(());
            }
            let hit = shared(task, state)?;
            let [component, x] = variates(task, state, 0x3000)?;
            let [y, _] = variates(task, state, 0x3001)?;
            let sample = indirect::sample(
                hit.surface,
                hit.shading_normal,
                hit.oriented_geometric_normal(),
                -state.ray.ok_or_else(|| invalid("CPU path ray missing"))?.direction(),
                [component, x, y],
            )
            .map_err(error)?;
            if let Some(sample) = sample {
                state.throughput = multiply(state.throughput, sample.weight);
                super::super::lighting::validate_radiance(state.throughput).map_err(error)?;
                if state.throughput != Vec3::ZERO {
                    state.previous_pdf = sample.pdf;
                    state.previous_position = hit.position;
                    let ray = hit.spawn(sample.direction).map_err(error)?;
                    state.ray = Some(ray);
                    state.cursor = Some(TraceCursor::new(ray, false));
                    state.hit = None;
                    state.depth += 1;
                    state.stage = Stage::Trace;
                    return Ok(());
                }
            }
            state.stage = Stage::Commit;
        }
        Stage::Trace | Stage::Shadow | Stage::Commit => return Err(invalid("CPU path phase bypassed its continuation")),
    }
    Ok(())
}
