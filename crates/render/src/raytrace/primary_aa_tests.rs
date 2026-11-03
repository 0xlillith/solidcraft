//! Spatial camera sampling and CPU output presentation regressions.
use super::*;
use crate::job::{ReflectionBody, ReflectionMode, ReflectionOptions};
use crate::{ProductMesh, StandardView};
use solidcraft_geom::{Aabb3, Mesh};
use std::sync::Arc;

fn progressive_fixture(samples: u32, seed: u32, left: Option<bool>, exposure: f64) -> ImageTask {
    // Reuse the analytic half-plane camera/geometry, not fixed-AA output.
    let fixed = fixture(1, 1, left);
    let mut options = fixed.options.clone();
    options.progressive = Some(job::CpuProgressiveOptions { samples_per_pixel: samples, seed });
    options.exposure_ev = exposure;
    let mut request = fixed.request.clone();
    request.raytrace = Some(options.clone());
    ImageTask::new(fixed.scene.clone(), request, CancelToken::default(), options).unwrap()
}

type ProgressiveOutput = (Vec<(u32, Vec<u8>)>, Vec<u8>, u64);
fn progressive_complete(mut task: ImageTask, quantum: usize) -> ProgressiveOutput {
    let mut previews = Vec::new();
    for _ in 0..200_000 {
        let result = task.step_with_previews(quantum).unwrap();
        let progress = task.progress();
        assert!(progress.done <= progress.total);
        match result {
            ImagePoll::Pending(_) => {}
            ImagePoll::Preview { png, samples_per_pixel } => previews.push((samples_per_pixel, png)),
            ImagePoll::Complete(png) => return (previews, png, task.work.used()),
        }
    }
    panic!("bounded progressive fixture did not finish");
}

#[test]
fn progressive_real_png_has_stable_prefixes_quanta_and_fresh_primary_samples() {
    let (previews, final_png, work) = progressive_complete(progressive_fixture(8, 123, Some(true), 0.0), 1);
    assert_eq!(previews.iter().map(|(samples, _)| *samples).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 6, 7]);
    let (other, other_final, other_work) = progressive_complete(progressive_fixture(8, 123, Some(true), 0.0), 16_384);
    assert_eq!(other, previews);
    assert_eq!(other_final, final_png);
    assert_eq!(other_work, work, "quanta cannot reset or change the cumulative budget");
    for (samples, checkpoint) in &previews {
        let (_, prefix, _) = progressive_complete(progressive_fixture(*samples, 123, Some(true), 0.0), 3);
        assert_eq!(&prefix, checkpoint, "target SPP must not alter an earlier prefix");
    }
    let mut task = progressive_fixture(8, 123, Some(true), 0.0);
    let mut rays = Vec::new();
    for _ in 0..200_000 {
        let absent = task.pending.is_none();
        let result = task.step_with_previews(1).unwrap();
        if absent && let Some(pending) = task.pending.as_ref() {
            assert!(!pending.repeatable_primary, "jittered primary samples cannot use fixed-centre reuse");
            rays.push([pending.ray.origin().x.to_bits(), pending.ray.origin().y.to_bits()]);
        }
        if matches!(result, ImagePoll::Complete(_)) {
            break;
        }
    }
    assert_eq!(rays.len(), 8);
    rays.sort();
    rays.dedup();
    assert_eq!(rays.len(), 8, "each absolute ordinal traces a new camera sample");
}

#[test]
fn progressive_coverage_keeps_backplate_outside_exposure_and_tone_mapping() {
    for exposure in [-16.0, 0.0, 16.0] {
        let (_, background, _) = progressive_complete(progressive_fixture(5, 9, None, exposure), 4);
        assert_eq!(::image::load_from_memory(&background).unwrap().to_rgba8().get_pixel(0, 0).0, [255; 4]);
        let (_, half, _) = progressive_complete(progressive_fixture(64, 9, Some(true), exposure), 4);
        let covered = (0..64).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
        assert!(covered > 0 && covered < 64);
        let expected = encode8(1.0 - covered as f64 / 64.0);
        assert_eq!(::image::load_from_memory(&half).unwrap().to_rgba8().get_pixel(0, 0).0, [expected, expected, expected, 255]);
    }
    let (previews, _, _) = progressive_complete(progressive_fixture(5, 9, Some(true), 0.0), 1);
    assert_eq!(previews.iter().map(|(samples, _)| *samples).collect::<Vec<_>>(), [1, 2, 3, 4], "every complete nonfinal pass is published");
}

#[test]
fn progressive_resolves_the_mean_radiance_once_instead_of_averaging_display_pixels() {
    let left = fixture(1, 1, Some(true));
    let right = fixture(1, 1, Some(false));
    let mut source = left.scene.clone();
    let mut other = right.scene.meshes[0].clone();
    other.name = "Bright half".into();
    source.bounds = source.bounds.union(&other.mesh.bounds());
    source.meshes.push(other);
    let mut options = left.options.clone();
    options.reflections = None;
    options.progressive = Some(job::CpuProgressiveOptions { samples_per_pixel: 64, seed: 9 });
    options.exposure_ev = 1.0;
    options.emissions = Some(vec![
        job::SurfaceEmission { body_index: 0, body: "Black half".into(), color: [1.0, 0.5, 0.25], strength: 1.0, sides: job::EmissionSides::Both },
        job::SurfaceEmission { body_index: 1, body: "Bright half".into(), color: [1.0, 0.5, 0.25], strength: 9.0, sides: job::EmissionSides::Both },
    ]);
    let request = ImageRequest { raytrace: Some(options.clone()), ..left.request.clone() };
    let task = ImageTask::new(source, request, CancelToken::default(), options).unwrap();
    let (_, png, _) = progressive_complete(task, 4);
    let left_count = (0..64).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
    let mean = (left_count as f64 + (64 - left_count) as f64 * 9.0) / 64.0;
    let expected = [1.0, 0.5, 0.25].map(|color| {
        let exposed = mean * color * 2.0;
        encode8(exposed / (1.0 + exposed))
    });
    let actual = ::image::load_from_memory(&png).unwrap().to_rgba8().get_pixel(0, 0).0;
    assert_eq!(actual, [expected[0], expected[1], expected[2], 255]);
    let fraction = left_count as f64 / 64.0;
    let prematurely_mapped = encode8(fraction * (2.0 / 3.0) + (1.0 - fraction) * (18.0 / 19.0));
    assert_ne!(actual[0], prematurely_mapped, "each sample must remain linear until the whole pass resolves");
}

#[test]
fn progressive_film_and_resolve_share_cancellation_and_one_cumulative_work_budget() {
    let mut task = progressive_fixture(4, 0, Some(true), 0.0);
    task.work = WorkBudget::new(1).unwrap();
    assert!(matches!(task.step_with_previews(1).unwrap(), ImagePoll::Pending(_)));
    assert_eq!(task.step_with_previews(1).err().unwrap().code, ErrorCode::LimitExceeded);
    for during_resolve in [false, true] {
        let mut task = progressive_fixture(4, 0, Some(true), 0.0);
        if during_resolve {
            for _ in 0..200_000 {
                if task.progressive.as_ref().is_some_and(|p| p.resolving) {
                    break;
                }
                assert!(matches!(task.step_with_previews(1).unwrap(), ImagePoll::Pending(_)));
            }
            assert!(task.progressive.as_ref().unwrap().resolving);
        }
        task.cancel.cancel();
        assert_eq!(task.step_with_previews(4).err().unwrap().code, ErrorCode::Cancelled);
        assert!(!task.done);
    }
}

fn progressive_area_fixture(samples: u32, led: bool) -> ImageTask {
    let mut camera = crate::Camera { distance: 10.0, fov: 0.0, ..Default::default() };
    camera.set_view(StandardView::Top);
    let receiver = ProductMesh {
        transmission: None,
        name: "diffuse area receiver".into(),
        mesh: Arc::new(Mesh {
            positions: vec![Vec3::new(-20.0, -20.0, 0.0), Vec3::new(20.0, -20.0, 0.0), Vec3::new(20.0, 20.0, 0.0), Vec3::new(-20.0, 20.0, 0.0)],
            normals: vec![Vec3::Z; 4],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            tri_face: vec![0, 0],
            ..Default::default()
        }),
        color: [255; 3],
        opacity: 1.0,
        surface: Some(OpaqueSurface::new(Vec3::new(0.8, 0.6, 0.4), 0.0, 0.7, 1.5).unwrap()),
        faces: Vec::new(),
    };
    let mut options = RaytraceOptions {
        progressive: Some(job::CpuProgressiveOptions { samples_per_pixel: samples, seed: 9 }),
        lights: Some(Vec::new()),
        shadows: true,
        ..Default::default()
    };
    let strip = job::led_strip::LedStripOptions {
        center_mm: [0.0, 0.0, 10.0],
        length_mm: 4.0,
        width_mm: 2.0,
        length_axis: [1.0, 0.0, 0.0],
        width_axis: [0.0, -1.0, 0.0],
        color: [1.0, 0.5, 0.25],
        intensity: 8.0,
    };
    if led {
        strip.append_to(&mut options, 4).unwrap();
    } else {
        options.area_lights = Some(vec![job::RectangleLightOptions {
            center_mm: [0.0, 0.0, 10.0],
            edge_u_mm: [4.0, 0.0, 0.0],
            edge_v_mm: [0.0, -2.0, 0.0],
            color: [1.0, 0.5, 0.25],
            intensity: 8.0,
        }]);
        options.area_samples = Some(4);
    }
    options.seed = Some(73);
    let bounds = receiver.mesh.bounds();
    let request = ImageRequest {
        studio_weights: None,
        environment: None,
        textures: None,
        integrator: job::Integrator::Raytrace,
        raytrace: Some(options.clone()),
        camera,
        width: 1,
        height: 1,
        background: [13, 27, 41],
        transparent_background: false,
    };
    ImageTask::new(ProductScene { meshes: vec![receiver], bounds }, request, CancelToken::default(), options).unwrap()
}

fn progressive_area_expected(task: &ImageTask, samples: u32) -> [u8; 4] {
    let light = super::super::area::RectangleLight::new(
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(0.0, -2.0, 0.0),
        Vec3::new(1.0, 0.5, 0.25),
        8.0,
    )
    .unwrap();
    let surface = task.scene.meshes[0].surface.unwrap();
    let mut sum = Vec3::ZERO;
    for ordinal in 0..samples {
        let [dx, dy] = progressive::variates(9, 0, ordinal, 0);
        let (origin, direction) = task.request.camera.ray(dx, dy, 1.0, 1.0);
        // This infinite receiver is z=0. Its closed-form hit does not use the BVH.
        let point = origin + direction * (-origin.z / direction.z);
        let key = area_sample_key(0, 0, 1, ordinal).unwrap();
        for index in 0..4 {
            let emitter = light.sample(point, key, 0, index, 4, 73).unwrap().unwrap().position();
            let delta = emitter - point;
            let distance_squared = delta.len2();
            let incoming = delta / distance_squared.sqrt();
            // A*cos(emitter)/(r²*N), independently of the image light cursor.
            let incident = Vec3::new(1.0, 0.5, 0.25) * (8.0 * 8.0 * incoming.z / distance_squared / 4.0);
            let brdf = surface.brdf(Vec3::Z, -direction, incoming).unwrap();
            sum += multiply(brdf, incident) * incoming.z;
        }
    }
    let mean = sum / f64::from(samples);
    [encode8(mean.x / (1.0 + mean.x)), encode8(mean.y / (1.0 + mean.y)), encode8(mean.z / (1.0 + mean.z)), 255]
}

#[test]
fn progressive_area_and_led_pngs_match_numeric_means_prefixes_and_quanta() {
    let mut rectangle = None;
    for led in [false, true] {
        let expected: Vec<_> = (1..=5).map(|samples| progressive_area_expected(&progressive_area_fixture(5, led), samples)).collect();
        let (previews, final_png, work) = progressive_complete(progressive_area_fixture(5, led), 1);
        assert_eq!(previews.iter().map(|(samples, _)| *samples).collect::<Vec<_>>(), [1, 2, 3, 4]);
        assert_eq!((previews.clone(), final_png.clone(), work), progressive_complete(progressive_area_fixture(5, led), 16_384));
        for (samples, checkpoint) in &previews {
            let (_, prefix, _) = progressive_complete(progressive_area_fixture(*samples, led), 3);
            assert_eq!(&prefix, checkpoint);
            assert_eq!(::image::load_from_memory(checkpoint).unwrap().to_rgba8().get_pixel(0, 0).0, expected[(*samples - 1) as usize]);
        }
        assert_eq!(::image::load_from_memory(&final_png).unwrap().to_rgba8().get_pixel(0, 0).0, expected[4]);
        if let Some(previous) = &rectangle {
            assert_eq!(&(previews, final_png, work), previous, "a continuous LED uses exactly the same rectangular transport");
        } else {
            rectangle = Some((previews, final_png, work));
        }
    }
}

#[test]
fn progressive_area_and_led_shadow_continuations_observe_cancel_and_cumulative_work() {
    for led in [false, true] {
        for cancel in [false, true] {
            let mut task = progressive_area_fixture(3, led);
            for _ in 0..100_000 {
                if task.pending.as_ref().is_some_and(|pending| pending.area_shadow) {
                    break;
                }
                assert!(matches!(task.step_with_previews(1).unwrap(), ImagePoll::Pending(_)));
            }
            assert!(task.pending.as_ref().unwrap().area_shadow);
            assert_eq!(task.progressive.as_ref().unwrap().film.samples(), 0);
            if cancel {
                task.cancel.cancel();
            } else {
                task.work.limit = task.work.used();
            }
            assert_eq!(task.step_with_previews(1).err().unwrap().code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
            assert!(!task.done);
            assert_eq!(task.progressive.as_ref().unwrap().film.samples(), 0, "an interrupted area integral cannot publish a pass");
        }
    }
}

fn fixture(samples: u8, r: u8, left: Option<bool>) -> ImageTask {
    let mut camera = crate::Camera { distance: 10.0, fov: 0.0, ..Default::default() };
    camera.set_view(StandardView::Top);
    let meshes = if let Some(left) = left {
        let (x0, x1) = if left { (0.0, 0.5) } else { (0.5, 1.0) };
        let point = |x, y| {
            let (o, _) = camera.ray(x, y, 1.0, 1.0);
            Vec3::new(o.x, o.y, 0.0)
        };
        vec![ProductMesh {
            transmission: None,
            name: "Black half".into(),
            mesh: Arc::new(Mesh {
                positions: vec![point(x0, 0.0), point(x1, 0.0), point(x1, 1.0), point(x0, 1.0)],
                normals: vec![Vec3::Z; 4],
                triangles: vec![[0, 1, 2], [0, 2, 3]],
                tri_face: vec![0, 0],
                ..Default::default()
            }),
            color: [0; 3],
            opacity: 1.0,
            surface: Some(OpaqueSurface::new(Vec3::ZERO, 1.0, 1.0, 1.5).unwrap()),
            faces: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    let bounds = meshes.iter().fold(Aabb3::EMPTY, |b, m| b.union(&m.mesh.bounds()));
    let options = RaytraceOptions {
        primary_samples: samples,
        lights: Some(Vec::new()),
        shadows: false,
        reflections: left.map(|_| ReflectionOptions {
            bodies: vec![ReflectionBody { body: "Black half".into(), mode: ReflectionMode::RoughSpecular }],
            samples: r,
            max_bounces: 2,
        }),
        ..Default::default()
    };
    let request = ImageRequest {
        studio_weights: None,
        environment: None,
        textures: None,
        integrator: job::Integrator::Raytrace,
        raytrace: Some(options.clone()),
        camera,
        width: 1,
        height: 1,
        background: [255; 3],
        transparent_background: false,
    };
    ImageTask::new(ProductScene { meshes, bounds }, request, CancelToken::default(), options).unwrap()
}
fn until_emit(task: &mut ImageTask) -> usize {
    let mut primaries = 0;
    let mut previous_primary = None;
    for _ in 0..200_000 {
        let absent = task.pending.is_none();
        let before = task.work.used();
        assert!(matches!(task.step(1).unwrap(), TaskPoll::Pending(_)));
        assert!(task.work.used() - before <= 4);
        if absent && task.pending.as_ref().is_some_and(|p| p.bounce == 0) {
            primaries += 1;
            let pending = task.pending.as_ref().unwrap();
            assert!(pending.repeatable_primary, "fixed generator must certify only intra-spatial R invariance");
            let o = pending.ray.origin();
            let d = pending.ray.direction();
            let ray = [o.x.to_bits(), o.y.to_bits(), o.z.to_bits(), d.x.to_bits(), d.y.to_bits(), d.z.to_bits()];
            if let Some((pixel, spatial, previous_ray)) = previous_primary {
                if (pixel, spatial) == (task.pixel, task.spatial) {
                    assert_eq!(ray, previous_ray, "fixed primary ray changed between R attempts");
                } else {
                    assert_ne!(ray, previous_ray, "distinct spatial samples must not reuse their primary ray");
                }
            }
            previous_primary = Some((task.pixel, task.spatial, ray));
        }
        if let Some(p) = task.pending.as_ref() {
            if p.bounce != 0 {
                assert!(!p.repeatable_primary, "secondary paths must revoke primary reuse");
            } else if let Some(hit) = p.hit {
                let shared =
                    PreparedScene { bvh: task.bvh.as_ref().unwrap(), primitives: &task.looks, source: &task.scene }.resolve_hit(p.ray, hit).unwrap();
                if task.options.reflections.as_ref().is_some_and(|r| r.bodies.iter().any(|b| b.body == shared.body_name)) {
                    assert!(!p.repeatable_primary, "selected primary hit must revoke reuse even when its reflection is null");
                }
            }
        }
        assert!(task.pixels.is_empty(), "early primary hit/miss emitted pixel");
        if task.progress().phase == Phase::Running {
            assert_eq!(task.progress().done, 0, "subsamples counted as completed pixels");
        }
        if task.emit_spatial_pixel {
            return primaries;
        }
    }
    panic!("bounded AA fixture did not reach emission");
}
fn complete(mut task: ImageTask, quantum: usize) -> (Vec<u8>, u64) {
    for _ in 0..200_000 {
        if let TaskPoll::Complete(png) = task.step(quantum).unwrap() {
            return (png, task.work.used());
        }
    }
    panic!("bounded AA fixture did not finish");
}

#[test]
fn transient_options_are_strict_default_one_and_do_not_serialize_a_new_default_field() {
    let omitted: RaytraceOptions = serde_json::from_str("{}").unwrap();
    let explicit: RaytraceOptions = serde_json::from_str(r#"{"primary_samples":1}"#).unwrap();
    assert_eq!(omitted, explicit);
    assert_eq!(omitted.primary_samples, 1);
    assert!(!serde_json::to_value(&omitted).unwrap().as_object().unwrap().contains_key("primary_samples"));
    for n in [0, 2, 8, 17, 255] {
        assert!(RaytraceOptions { primary_samples: n, ..Default::default() }.validate().is_err());
    }
    for raw in [
        r#"{"primary_samples":-1}"#,
        r#"{"primary_samples":1.5}"#,
        r#"{"primary_samples":null}"#,
        r#"{"primary_samples":"4"}"#,
        r#"{"primary_samples":4,"unknown":0}"#,
    ] {
        assert!(serde_json::from_str::<RaytraceOptions>(raw).is_err());
    }
}

#[test]
fn fixed_grids_have_independent_analytic_halfplane_and_triangle_coverage() {
    assert_eq!(primary_offset(1, 0).unwrap(), [0.5, 0.5]);
    assert_eq!(primary_offset(4, 0).unwrap(), [0.25, 0.25]);
    assert_eq!(primary_offset(4, 3).unwrap(), [0.75, 0.75]);
    for (s, half, triangle) in [(4, 2, 1), (16, 8, 3)] {
        let points: Vec<_> = (0..s).map(|i| primary_offset(s, i).unwrap()).collect();
        assert!(points.iter().flatten().all(|v| *v > 0.0 && *v < 1.0));
        assert_eq!(points.iter().filter(|p| p[0] < 0.5).count(), half);
        assert_eq!(points.iter().filter(|p| p[0] + p[1] < 5.0 / 8.0).count(), triangle);
        assert!(primary_offset(s, s).is_err());
    }
    assert!(primary_offset(8, 0).is_err());
    assert!(spatial_key(u64::MAX, 0, 16).is_err());
    assert_ne!(spatial_key(7, 0, 4).unwrap(), spatial_key(7, 1, 4).unwrap());
}

#[test]
fn compositor_keeps_conditional_scene_mean_then_display_linear_coverage() {
    // White L=1, Reinhard=1/2; half coverage over black gives1/4 display-linear.
    assert_eq!(composite_spatial(Vec3::new(0.5, 0.5, 0.5), 2, 4, [0; 3], 0.0).unwrap(), [encode8(0.25), encode8(0.25), encode8(0.25), 255]);
    assert_ne!(encode8(0.25), encode8(1.0 / 3.0));
    // Full coverage of L=0 and L=2 usesT(mean)=1/2, not mean(T)=1/3.
    assert_eq!(composite_spatial(Vec3::new(1.0, 1.0, 1.0), 4, 4, [255; 3], 0.0).unwrap(), [encode8(0.5), encode8(0.5), encode8(0.5), 255]);
    for ev in [-16.0, 0.0, 16.0] {
        assert_eq!(composite_spatial(Vec3::ZERO, 0, 16, [13, 27, 41], ev).unwrap(), [13, 27, 41, 255]);
    }
    let gray = srgb8_to_linear(128);
    assert_eq!(composite_spatial(Vec3::ZERO, 2, 4, [128; 3], 0.0).unwrap(), [encode8(gray * 0.5), encode8(gray * 0.5), encode8(gray * 0.5), 255]);
    assert!(composite_spatial(Vec3::ZERO, 5, 4, [0; 3], 0.0).is_err());
    assert!(composite_spatial(Vec3::X, 0, 4, [0; 3], 0.0).is_err());
}

#[test]
fn real_hit_first_and_miss_first_silhouettes_keep_all_attempts_and_null_coverage() {
    for s in [4, 16] {
        for r in [1, 4, 16, 64] {
            for left in [false, true] {
                let mut task = fixture(s, r, Some(left));
                let primaries = until_emit(&mut task);
                assert_eq!(primaries, usize::from(s) * usize::from(r));
                assert_eq!(task.primary_coverage, u16::from(s) * u16::from(r) / 2);
                let before = task.work.used();
                task.step(1).unwrap();
                assert_eq!(task.work.used(), before + 1);
                assert_eq!(task.pixels, [encode8(0.5), encode8(0.5), encode8(0.5), 255]);
                assert_eq!(task.pixel, 1);
            }
        }
    }
}

#[test]
fn miss_only_aa_retains_exact_background_and_quantum_independent_png() {
    for s in [4, 16] {
        let mut task = fixture(s, 1, None);
        assert_eq!(until_emit(&mut task), usize::from(s));
        assert_eq!(task.primary_coverage, 0);
        let small = complete(task, 1);
        let large = complete(fixture(s, 1, None), 16384);
        assert_eq!(small, large);
        let image = ::image::load_from_memory(&small.0).unwrap().to_rgba8();
        assert_eq!(image.get_pixel(0, 0).0, [255; 4]);
    }
}

#[test]
fn spatial_finalization_and_emission_observe_shared_work_and_cancellation() {
    for emit in [false, true] {
        for cancel in [false, true] {
            let mut task = fixture(16, 4, Some(false));
            for _ in 0..200_000 {
                if (emit && task.emit_spatial_pixel) || (!emit && task.finish_spatial) {
                    break;
                }
                task.step(1).unwrap();
            }
            assert!(if emit { task.emit_spatial_pixel } else { task.finish_spatial });
            let used = task.work.used();
            if cancel {
                task.cancel.cancel();
            } else {
                task.work.limit = used;
            }
            let code = task.step(1).err().unwrap().code;
            assert_eq!(code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
            assert_eq!(task.work.used(), used);
            assert!(!task.done && task.pixels.is_empty());
        }
    }
}

#[test]
fn legacy_single_primary_ignores_aa_state_and_preserves_default_bytes_work_and_order() {
    for left in [None, Some(true), Some(false)] {
        for r in [1, 4, 16, 64] {
            let control = fixture(1, r, left);
            let mut isolated = fixture(1, r, left);
            isolated.spatial = 16;
            isolated.primary_coverage = 1024;
            isolated.spatial_radiance = Vec3::new(f64::NAN, 0.0, 0.0);
            isolated.finish_spatial = true;
            isolated.emit_spatial_pixel = true;
            assert_eq!(complete(control, 1), complete(isolated, 16384));
        }
    }
    assert!(std::mem::size_of::<ImageTask>() <= 4096);
    assert_eq!(crate::raytrace::MAX_WORK, 1_000_000_000);
    assert_eq!(job::image::MAX_JOB_BYTES, 384 * 1024 * 1024);
    let legacy = fixture(1, 4, Some(false));
    let aa = fixture(16, 4, Some(false));
    assert_eq!(legacy.admission.estimated_total_bytes, aa.admission.estimated_total_bytes);
}

#[test]
fn full_coverage_black_plane_never_leaks_display_background() {
    for samples in [4, 16] {
        let mut initial = fixture(samples, 16, Some(true));
        let top = initial.request.camera.ray(1.0, 0.0, 1.0, 1.0).0;
        let bottom = initial.request.camera.ray(1.0, 1.0, 1.0, 1.0).0;
        let mesh = Arc::get_mut(&mut initial.scene.meshes[0].mesh).unwrap();
        mesh.positions[1] = Vec3::new(top.x, top.y, 0.0);
        mesh.positions[2] = Vec3::new(bottom.x, bottom.y, 0.0);
        initial.scene.bounds = mesh.bounds();
        let mut task = ImageTask::new(initial.scene, initial.request, initial.cancel, initial.options).unwrap();
        assert_eq!(until_emit(&mut task), usize::from(samples) * 16);
        assert_eq!(task.primary_coverage, u16::from(samples) * 16);
        task.step(1).unwrap();
        assert_eq!(task.pixels, [0, 0, 0, 255]);
    }
}

#[test]
fn later_spatial_attempts_do_not_reset_work_and_never_publish_when_interrupted() {
    for cancel in [false, true] {
        let mut task = fixture(16, 64, Some(true));
        for _ in 0..200_000 {
            if task.spatial >= 1 && task.sample >= 1 {
                break;
            }
            task.step(1).unwrap();
        }
        assert!(task.spatial >= 1 && task.sample >= 1);
        let used = task.work.used();
        assert!(used > 64, "no cumulative work across spatial samples");
        if cancel {
            task.cancel.cancel();
        } else {
            task.work.limit = used;
        }
        assert_eq!(task.step(1).err().unwrap().code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
        assert_eq!(task.work.used(), used);
        assert!(!task.done && task.pixels.is_empty());
    }
}

#[test]
fn actual_colored_lit_partial_coverage_wires_positive_radiance_exposure_and_background() {
    for samples in [4, 16] {
        for left in [false, true] {
            let mut initial = fixture(samples, 1, Some(left));
            initial.scene.meshes[0].surface = Some(OpaqueSurface::new(Vec3::new(1.0, 1.0, 1.0), 1.0, 1.0, 1.5).unwrap());
            initial.options.reflections = None;
            initial.options.lights = Some(vec![job::DirectionalLightOptions { direction: [0.0, 0.0, 1.0], color: [0.25, 0.5, 1.0], intensity: 2.0 }]);
            initial.options.exposure_ev = 1.0;
            initial.request.background = [13, 73, 139];
            initial.request.raytrace = Some(initial.options.clone());
            let mut task = ImageTask::new(initial.scene, initial.request, initial.cancel, initial.options).unwrap();
            assert_eq!(until_emit(&mut task), usize::from(samples));
            assert_eq!(task.primary_coverage, u16::from(samples) / 2);
            // At normal incidence, unit-alpha white conductor: D=1/pi,
            // G1=F=1, f=1/(4pi), incoming radiance=2*colored light.
            let linear = [0.5, 1.0, 2.0].map(|v| v / (4.0 * std::f64::consts::PI));
            let expected_p = Vec3::new(linear[0], linear[1], linear[2]) * 0.5;
            assert!((task.spatial_radiance - expected_p).len() < 1e-12);
            let expected: [u8; 3] = std::array::from_fn(|i| {
                let exposed = linear[i] * 2.0;
                let display = 0.5 * exposed / (1.0 + exposed) + 0.5 * srgb8_to_linear([13, 73, 139][i]);
                encode8(display)
            });
            task.step(1).unwrap();
            assert_eq!(task.pixels, [expected[0], expected[1], expected[2], 255]);
        }
    }
}

#[test]
fn workstation_progressive_resolves_every_checkpoint_through_its_admitted_cap() {
    let fixed = fixture(1, 1, Some(true));
    let cap = job::CpuBudget::Workstation.limits().max_samples_per_pixel;
    assert_eq!(cap, 4096);
    for samples in [2048, cap] {
        let mut options = fixed.options.clone();
        options.budget = job::CpuBudget::Workstation;
        options.progressive = Some(job::CpuProgressiveOptions { samples_per_pixel: samples, seed: 123 });
        let request = ImageRequest { raytrace: Some(options.clone()), ..fixed.request.clone() };
        let task = ImageTask::new(fixed.scene.clone(), request, CancelToken::default(), options).unwrap();
        let (previews, final_png, _) = progressive_complete(task, 16_384);
        let expected: Vec<_> = (1..samples).collect();
        assert_eq!(previews.iter().map(|(n, _)| *n).collect::<Vec<_>>(), expected);
        for (_, png) in &previews {
            assert_eq!(::image::load_from_memory(png).unwrap().to_rgba8().dimensions(), (1, 1));
        }
        assert_eq!(::image::load_from_memory(&final_png).unwrap().to_rgba8().dimensions(), (1, 1));
    }
    assert!(composite_spatial(Vec3::ZERO, 0, 4097, [0; 3], 0.0).is_err());
}

#[test]
fn area_light_points_include_spatial_and_film_sample_identity() {
    let light = super::super::area::RectangleLight::new(Vec3::Z * 10.0, Vec3::X * 4.0, -Vec3::Y * 4.0, Vec3::new(1.0, 1.0, 1.0), 1.0).unwrap();
    let mut keys = std::collections::BTreeSet::new();
    let mut points = std::collections::BTreeSet::new();
    for film in [0, 1, 1023, 2047, 4095] {
        for spatial in 0..16 {
            let key = area_sample_key(41, spatial, 16, film).unwrap();
            assert!(keys.insert(key));
            let point = light.sample(Vec3::ZERO, key, 2, 0, 4, 73).unwrap().unwrap().position();
            assert!(points.insert((point.x.to_bits(), point.y.to_bits())));
            assert_eq!(light.sample(Vec3::ZERO, key, 2, 0, 4, 73).unwrap().unwrap().position(), point);
        }
    }
    assert_eq!(area_sample_key(41, 0, 1, 0).unwrap(), 41);
    assert!(area_sample_key(41, 0, 1, 4096).is_err());
    assert!(area_sample_key(41, 4, 4, 0).is_err());
    assert!(area_sample_key(u64::MAX, 0, 4, 0).is_err());
}

fn presentation_fixture(samples: Option<u32>, primary: u8, output: Option<job::OutputOptions>, exposure: f64) -> ImageTask {
    let initial = fixture(primary, 1, Some(true));
    let mut options = initial.options.clone();
    options.reflections = None;
    options.progressive = samples.map(|samples_per_pixel| job::CpuProgressiveOptions { samples_per_pixel, seed: 9 });
    options.output = output.map(Box::new);
    options.exposure_ev = exposure;
    options.emissions = Some(vec![job::SurfaceEmission {
        body_index: 0,
        body: "Black half".into(),
        color: [1.0, 0.5, 0.25],
        strength: 9.0,
        sides: job::EmissionSides::Both,
    }]);
    let request = ImageRequest { raytrace: Some(options.clone()), ..initial.request.clone() };
    ImageTask::new(initial.scene.clone(), request, CancelToken::default(), options).unwrap()
}
type PresentedOutput = (Vec<(u32, Vec<u8>)>, Vec<u8>, Option<Vec<u8>>, u64);
fn complete_presentation(mut task: ImageTask, units: usize) -> PresentedOutput {
    let mut previews = Vec::new();
    for _ in 0..200_000 {
        match task.step_with_previews(units).unwrap() {
            ImagePoll::Pending(_) => {}
            ImagePoll::Preview { png, samples_per_pixel } => {
                assert!(task.take_linear_exr().is_none(), "provisional images never expose a raw master");
                previews.push((samples_per_pixel, png));
            }
            ImagePoll::Complete(png) => {
                let master = task.take_linear_exr();
                assert!(task.take_linear_exr().is_none(), "raw ownership transfers exactly once");
                return (previews, png, master, task.work.used());
            }
        }
    }
    panic!("bounded CPU presentation did not finish");
}
fn creative_output() -> job::OutputOptions {
    job::OutputOptions {
        tone_map: job::ToneMap::AcesFitted,
        linear_exr: true,
        finishing: job::finishing::Finishing {
            rgb_gain: [1.3, 0.8, 1.1],
            contrast: 1.2,
            saturation: 0.7,
            bloom: Some(job::bloom::Bloom { strength: 0.2, threshold: 0.5, radius: 1 }),
            photographic: job::photographic::Photographic { grain: 0.03, seed: 42, ..Default::default() },
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn cpu_identity_output_and_raw_retention_preserve_original_f64_pngs() {
    for (samples, primary) in [(None, 1), (None, 4), (Some(7), 1)] {
        let original = complete_presentation(presentation_fixture(samples, primary, None, 1.5), 1);
        for linear_exr in [false, true] {
            let identity = job::OutputOptions { linear_exr, ..Default::default() };
            let actual = complete_presentation(presentation_fixture(samples, primary, Some(identity), 1.5), 1);
            assert_eq!(actual.0, original.0, "even raw retention must preserve every original provisional PNG byte");
            assert_eq!(actual.1, original.1, "identity output must preserve the original f64 PNG exactly");
            assert_eq!(actual.2.is_some(), linear_exr);
            if let Some(master) = actual.2 {
                assert!(master.windows(b"SolidCraft native CPU raytrace".len()).any(|w| w == b"SolidCraft native CPU raytrace"));
            }
        }
    }
}

#[test]
fn cpu_shared_presentation_keeps_every_real_prefix_and_worker_quantum() {
    let output = creative_output();
    let first = complete_presentation(presentation_fixture(Some(7), 1, Some(output), -1.0), 1);
    assert_eq!(first.0.iter().map(|(samples, _)| *samples).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 6]);
    assert_eq!(first, complete_presentation(presentation_fixture(Some(7), 1, Some(output), -1.0), 16_384));
    for (samples, png) in &first.0 {
        let prefix = complete_presentation(presentation_fixture(Some(*samples), 1, Some(output), -1.0), 3);
        assert_eq!(&prefix.1, png, "EXR work at a final target cannot change its shared presentation");
    }
    let legacy = complete_presentation(presentation_fixture(Some(7), 1, None, -1.0), 1);
    assert_ne!(first.1, legacy.1, "the selected presentation must affect the actual PNG");
}

#[test]
fn cpu_raw_master_keeps_hdr_and_coverage_before_every_presentation_operator() {
    let identity = job::OutputOptions { linear_exr: true, ..Default::default() };
    let original = complete_presentation(presentation_fixture(Some(7), 1, Some(identity), -8.0), 1);
    let mut creative = creative_output();
    creative.backdrop = Some(job::finishing::Backdrop::default());
    let changed = complete_presentation(presentation_fixture(Some(7), 1, Some(creative), 8.0), 1);
    assert_eq!(original.2, changed.2, "raw radiance/coverage cannot contain exposure, bloom, backdrop, tone map or photographic effects");
    let master = original.2.unwrap();
    // The one-pixel uncompressed scanline ends in its canonical A,B,G,R float channels.
    let tail = &master[master.len() - 16..];
    let channel = |at| f32::from_le_bytes(tail[at..at + 4].try_into().unwrap());
    let covered = (0..7).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count() as f64;
    let alpha = (covered / 7.0) as f32;
    assert_eq!(channel(0), alpha);
    for (actual, scale) in [(channel(12), 1.0), (channel(8), 0.5), (channel(4), 0.25)] {
        assert_eq!(actual, (covered / 7.0 * 9.0 * scale) as f32);
    }
    assert!(channel(12) > 1.0, "raw HDR radiance must survive display clipping");
    assert_ne!(original.1, changed.1);
}

#[test]
fn cpu_output_stages_share_cancel_and_one_cumulative_work_ceiling() {
    for stage in ["exr", "bloom", "display", "photographic", "png"] {
        for cancel in [false, true] {
            let mut output = creative_output();
            // The final target also exercises EXR; other stages remain reachable afterward.
            output.linear_exr = true;
            let mut task = presentation_fixture(Some(3), 1, Some(output), 0.0);
            for _ in 0..200_000 {
                if task.output.first().is_some_and(|output| output.stage_name() == stage) {
                    break;
                }
                assert!(!matches!(task.step_with_previews(1).unwrap(), ImagePoll::Complete(_)));
            }
            assert_eq!(task.output.first().unwrap().stage_name(), stage);
            if cancel {
                task.cancel.cancel();
            } else {
                task.work.limit = task.work.used();
            }
            assert_eq!(task.step_with_previews(1).err().unwrap().code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
            assert!(!task.done);
            assert!(task.take_linear_exr().is_none(), "interrupted output cannot expose a final master");
        }
    }
}

fn cpu_lens_fixture(samples: u32, lens: Option<job::ThinLens>) -> ImageTask {
    let fixed = fixture(1, 1, Some(true));
    let mut options = fixed.options.clone();
    options.reflections = None;
    options.progressive = Some(job::CpuProgressiveOptions { samples_per_pixel: samples, seed: 9 });
    options.lens = lens;
    let mut request = ImageRequest { raytrace: Some(options.clone()), ..fixed.request.clone() };
    // At z=0 this perspective projection has the same eight-mm image span as
    // the analytic half-plane fixture. Positive pupil radius changes transport only.
    request.camera.fov = 2.0 * 0.4f64.atan();
    ImageTask::new(fixed.scene.clone(), request, CancelToken::default(), options).unwrap()
}

#[test]
fn cpu_thin_lens_rays_match_rolled_axial_focus_and_uniform_area_pupil() {
    let mut request = fixture(1, 1, None).request;
    request.width = 8;
    request.height = 6;
    for (yaw, pitch, roll) in [(0.0, 0.0, 0.0), (-0.45, 0.5, 0.7)] {
        request.camera = crate::Camera { target: Vec3::new(2.0, 3.0, 4.0), distance: 12.0, fov: 0.7, yaw, pitch, roll, ..Default::default() };
        let camera = request.camera;
        let (right, up, back) = camera.basis();
        let lens = job::ThinLens { aperture_radius_mm: 0.4, focus_distance_mm: 9.0 };
        for (x, y) in [(0.25, 1.5), (4.0, 3.0), (7.8, 5.7)] {
            let nx = x / f64::from(request.width) * 2.0 - 1.0;
            let ny = 1.0 - y / f64::from(request.height) * 2.0;
            let focal = camera.eye() - back * lens.focus_distance_mm
                + (right * (nx * f64::from(request.width) / f64::from(request.height)) + up * ny)
                    * ((camera.fov * 0.5).tan() * lens.focus_distance_mm);
            for pupil in [[0.0, 0.0], [0.25, 0.25], [0.81, 0.5]] {
                let disk = (right * (std::f64::consts::TAU * pupil[1]).cos() + up * (std::f64::consts::TAU * pupil[1]).sin())
                    * (lens.aperture_radius_mm * pupil[0].sqrt());
                let ray = lens_primary_ray(&request, x, y, (0.1, 100.0), lens, pupil).unwrap();
                for depth in [0.0, lens.focus_distance_mm * 0.5, lens.focus_distance_mm, lens.focus_distance_mm * 2.0] {
                    let start_depth = (ray.origin() - camera.eye()).dot(-back);
                    let hit = ray.origin() + ray.direction() * ((depth - start_depth) / ray.direction().dot(-back));
                    let pinhole = camera.eye() + (focal - camera.eye()) * (depth / lens.focus_distance_mm);
                    let expected = pinhole + disk * (1.0 - depth / lens.focus_distance_mm);
                    assert!(hit.dist(expected) < 1e-10, "axial depth={depth}, pupil={pupil:?}: {hit:?} != {expected:?}");
                }
            }
        }
    }
}

#[test]
fn cpu_disabled_lens_preserves_fixed_and_every_progressive_png_byte_and_work_unit() {
    for output in [None, Some(job::OutputOptions { linear_exr: true, ..Default::default() })] {
        for (samples, primary) in [(None, 1), (None, 4), (None, 16), (Some(7), 1)] {
            let original = presentation_fixture(samples, primary, output, 1.5);
            let mut options = original.options.clone();
            options.lens = Some(job::ThinLens { aperture_radius_mm: 0.0, focus_distance_mm: 8.0 });
            let request = ImageRequest { raytrace: Some(options.clone()), ..original.request.clone() };
            let zero = ImageTask::new(original.scene.clone(), request, CancelToken::default(), options).unwrap();
            assert_eq!(zero.admission.estimated_total_bytes, original.admission.estimated_total_bytes, "a zero lens needs no extra buffer");
            assert_eq!(complete_presentation(original, 1), complete_presentation(zero, 1));
        }
    }
}

#[test]
fn cpu_thin_lens_image_has_numeric_defocus_stable_prefixes_and_no_exposure_factor() {
    let radius = 4.0;
    let focused = Some(job::ThinLens { aperture_radius_mm: radius, focus_distance_mm: 10.0 });
    let defocused = Some(job::ThinLens { aperture_radius_mm: radius, focus_distance_mm: 20.0 });
    let pinhole = progressive_complete(cpu_lens_fixture(64, None), 1);
    let sharp = progressive_complete(cpu_lens_fixture(64, focused), 1);
    assert_eq!((sharp.0, sharp.1), (pinhole.0, pinhole.1), "every pupil must meet the same focused surface without an aperture-area exposure factor");
    let actual = progressive_complete(cpu_lens_fixture(64, defocused), 1);
    assert_eq!(actual, progressive_complete(cpu_lens_fixture(64, defocused), 16_384));
    for (samples, png) in &actual.0 {
        assert_eq!(*png, progressive_complete(cpu_lens_fixture(*samples, defocused), 3).1);
    }
    let task = cpu_lens_fixture(64, defocused);
    let camera = task.request.camera;
    let (right, up, back) = camera.basis();
    let bounds = task.scene.meshes[0].mesh.bounds();
    let covered = (0..64)
        .filter(|ordinal| {
            let [x, y] = progressive::variates(9, 0, *ordinal, 0);
            let [u, v] = progressive::variates(9, 0, *ordinal, 0x4c454e53);
            let disk = (right * (std::f64::consts::TAU * v).cos() + up * (std::f64::consts::TAU * v).sin()) * (radius * u.sqrt());
            let point = camera.eye() - back * camera.distance
                + (right * (2.0 * x - 1.0) + up * (1.0 - 2.0 * y)) * camera.half_height()
                + disk * (1.0 - camera.distance / 20.0);
            point.x >= bounds.min.x && point.x <= bounds.max.x && point.y >= bounds.min.y && point.y <= bounds.max.y
        })
        .count();
    let expected = encode8(1.0 - covered as f64 / 64.0);
    let rgba = ::image::load_from_memory(&actual.1).unwrap().to_rgba8();
    assert_eq!(rgba.get_pixel(0, 0).0, [expected, expected, expected, 255]);
    assert_ne!(actual.1, progressive_complete(cpu_lens_fixture(64, None), 1).1, "physical defocus must change the actual image");
}

#[test]
fn cpu_lens_pupil_continuations_observe_cancel_and_one_cumulative_work_budget() {
    for pupil_only in [true, false] {
        for cancel in [false, true] {
            let mut task = cpu_lens_fixture(7, Some(job::ThinLens { aperture_radius_mm: 0.5, focus_distance_mm: 20.0 }));
            for _ in 0..100_000 {
                let before = task.work.used();
                assert!(matches!(task.step_with_previews(1).unwrap(), ImagePoll::Pending(_)));
                assert!(task.work.used() - before <= 1, "pupil generation is a separate bounded continuation");
                if if pupil_only { task.pupil_sample.is_some() } else { task.pending.is_some() } {
                    break;
                }
            }
            if pupil_only {
                assert!(task.pupil_sample.is_some());
                assert!(task.pending.is_none(), "the pupil continuation yields before constructing the primary ray");
            } else {
                assert!(task.pending.is_some(), "the real pupil ray must have been generated");
                assert!(!task.pending.as_ref().unwrap().repeatable_primary);
                assert!(task.pupil_sample.is_none());
            }
            let before = task.work.used();
            if cancel {
                task.cancel.cancel();
            } else {
                task.work.limit = before;
            }
            assert_eq!(task.step_with_previews(1).err().unwrap().code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
            assert_eq!(task.work.used(), before);
            assert_eq!(task.progressive.as_ref().unwrap().film.samples(), 0);
            assert!(task.pixels.is_empty());
            assert!(task.take_linear_exr().is_none());
        }
    }
}

#[test]
fn cpu_lens_admission_keeps_the_whole_pupil_outside_certified_glass() {
    let scene = dielectric::tests::scene(dielectric::tests::box_body(8.0));
    let options = RaytraceOptions {
        progressive: Some(job::CpuProgressiveOptions { samples_per_pixel: 4, seed: 9 }),
        lens: Some(job::ThinLens { aperture_radius_mm: 1.99, focus_distance_mm: 10.0 }),
        dielectrics: vec![dielectric::tests::input(job::DielectricBoundary::ClosedBox)],
        ..Default::default()
    };
    let mut request = dielectric::tests::request(options.clone());
    request.camera.distance = 10.0;
    request.camera.fov = 0.7;
    admit(&scene, &request, &options).unwrap();
    for radius in [2.0, 2.01, 10.0] {
        let options = RaytraceOptions { lens: Some(job::ThinLens { aperture_radius_mm: radius, focus_distance_mm: 10.0 }), ..options.clone() };
        let request = ImageRequest { raytrace: Some(options.clone()), ..request.clone() };
        assert!(admit(&scene, &request, &options).unwrap_err().message.contains("aperture sphere"));
    }
}

#[test]
fn cpu_lens_extreme_valid_physical_settings_refuse_before_ray_transport() {
    let mut task = cpu_lens_fixture(4, None);
    task.options.lens = Some(job::ThinLens { aperture_radius_mm: 1e7, focus_distance_mm: 1e-6 });
    task.request.raytrace = Some(task.options.clone());
    let error = admit(&task.scene, &task.request, &task.options).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidRequest);
    assert!(error.message.contains("bounded ray"));
    assert_eq!(task.progressive.as_ref().unwrap().film.samples(), 0);
}

fn alpha_fixture(samples: Option<u32>, primary: u8, transparent: bool, output: Option<job::OutputOptions>, exposure: f64) -> ImageTask {
    let original = presentation_fixture(samples, primary, output, exposure);
    let request = ImageRequest { transparent_background: transparent, ..original.request.clone() };
    ImageTask::new(original.scene, request, original.cancel, original.options).unwrap()
}
fn alpha_pixel(png: &[u8]) -> [u8; 4] {
    ::image::load_from_memory(png).unwrap().to_rgba8().get_pixel(0, 0).0
}
fn alpha_encoded_work(output: &PresentedOutput) -> u64 {
    output.0.iter().map(|(_, png)| png.len() as u64).sum::<u64>() + output.1.len() as u64
}

#[test]
fn cpu_transparent_png_uses_analytic_straight_rgb_and_fixed_or_film_coverage() {
    for primary in [4, 16] {
        for exposure in [-2.0, 0.0, 1.5] {
            let clear = complete_presentation(alpha_fixture(None, primary, true, None, exposure), 1);
            let opaque = complete_presentation(alpha_fixture(None, primary, false, None, exposure), 1);
            let exposed = 2.0f64.powf(exposure);
            let rgb = [9.0, 4.5, 2.25].map(|linear| encode8(linear * exposed / (1.0 + linear * exposed)));
            assert_eq!(
                alpha_pixel(&clear.1),
                [rgb[0], rgb[1], rgb[2], 128],
                "straight RGB is the covered scene mean, never multiplied by edge alpha"
            );
            assert_eq!(clear.3, opaque.3, "a background flag adds no transport or work debit");
            assert_ne!(clear.1, opaque.1);
        }
    }
    let complete = complete_presentation(alpha_fixture(Some(16), 1, true, None, 1.5), 1);
    let rgb = [9.0, 4.5, 2.25].map(|linear| {
        let value = linear * 2.0f64.powf(1.5);
        encode8(value / (1.0 + value))
    });
    for (samples, png) in complete.0.iter().chain(std::iter::once(&(16, complete.1.clone()))) {
        let covered = (0..*samples).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
        let alpha = (covered as f64 / f64::from(*samples) * 255.0).round() as u8;
        assert_eq!(alpha_pixel(png), if alpha == 0 { [0; 4] } else { [rgb[0], rgb[1], rgb[2], alpha] });
    }
    for primary in [1, 4, 16] {
        let original = fixture(primary, 1, None);
        let request = ImageRequest { transparent_background: true, ..original.request.clone() };
        let task = ImageTask::new(original.scene, request, original.cancel, original.options).unwrap();
        assert_eq!(alpha_pixel(&progressive_complete(task, 1).1), [0; 4]);
    }
    assert!(composite_spatial_alpha(Vec3::X, 0, 4, [1, 2, 3], 0.0, true).is_err());
    assert!(composite_spatial_alpha(Vec3::ZERO, 5, 4, [1, 2, 3], 0.0, true).is_err());
    let empty = fixture(1, 1, None);
    let options = RaytraceOptions {
        output: Some(Box::new(job::OutputOptions {
            finishing: job::finishing::Finishing { contrast: 0.5, ..Default::default() },
            ..Default::default()
        })),
        ..empty.options
    };
    let request = ImageRequest { transparent_background: true, raytrace: Some(options.clone()), ..empty.request };
    let clear = ImageTask::new(empty.scene, request, empty.cancel, options).unwrap();
    assert_eq!(
        alpha_pixel(&complete_presentation(clear, 1).1),
        [0; 4],
        "allowed colour finishing cannot leave lifted-black RGB in a zero-alpha pixel"
    );
}

#[test]
fn cpu_alpha_keeps_opaque_f64_bytes_work_and_all_raw_exr_bits() {
    let identity = job::OutputOptions { linear_exr: true, ..Default::default() };
    for (samples, primary) in [(None, 1), (None, 4), (None, 16), (Some(7), 1)] {
        let original = complete_presentation(presentation_fixture(samples, primary, Some(identity), 1.5), 1);
        let false_flag = complete_presentation(alpha_fixture(samples, primary, false, Some(identity), 1.5), 1);
        assert_eq!(false_flag, original, "false stays exactly on the legacy f64 PNG/master/work path");
        let source = alpha_fixture(samples, primary, false, Some(identity), 1.5);
        let transparent = alpha_fixture(samples, primary, true, Some(identity), 1.5);
        assert_eq!(
            source.admission.estimated_total_bytes, transparent.admission.estimated_total_bytes,
            "alpha uses existing coverage/RGBA storage only"
        );
        let alpha = complete_presentation(transparent, 1);
        assert_eq!(alpha.2, original.2, "the scene-linear premultiplied raw master is independent of display alpha choice");
        assert_eq!(
            alpha.3 - alpha_encoded_work(&alpha),
            original.3 - alpha_encoded_work(&original),
            "only actual encoded-byte digest work can differ with PNG content"
        );
        let no_master = complete_presentation(alpha_fixture(samples, primary, true, None, 1.5), 1);
        assert_eq!((alpha.0, alpha.1), (no_master.0, no_master.1), "raw retention cannot change any straight-alpha PNG prefix");
    }
}

#[test]
fn cpu_alpha_shared_finishing_preserves_prefixes_quanta_and_raw_values() {
    let output = job::OutputOptions {
        tone_map: job::ToneMap::AcesFitted,
        finishing: job::finishing::Finishing { rgb_gain: [1.2, 0.8, 1.1], ..Default::default() },
        linear_exr: true,
        ..Default::default()
    };
    let actual = complete_presentation(alpha_fixture(Some(7), 1, true, Some(output), -1.0), 1);
    assert_eq!(actual, complete_presentation(alpha_fixture(Some(7), 1, true, Some(output), -1.0), 16384));
    for (samples, png) in &actual.0 {
        assert_eq!(*png, complete_presentation(alpha_fixture(Some(*samples), 1, true, Some(output), -1.0), 3).1);
    }
    let opaque = complete_presentation(alpha_fixture(Some(7), 1, false, Some(output), -1.0), 1);
    assert_eq!(actual.2, opaque.2);
    assert_eq!(
        actual.3 - alpha_encoded_work(&actual),
        opaque.3 - alpha_encoded_work(&opaque),
        "alpha adds no film, conversion or presentation work beyond actual PNG byte hashing"
    );
    let coverage = (0..7).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
    let rgb = std::array::from_fn::<_, 3, _>(|i| {
        let x: f64 = [9.0, 4.5, 2.25][i] * 0.5 * [1.2, 0.8, 1.1][i];
        encode8(((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0))
    });
    assert_eq!(alpha_pixel(&actual.1), [rgb[0], rgb[1], rgb[2], (coverage as f64 / 7.0 * 255.0).round() as u8]);
}

#[test]
fn cpu_alpha_presentation_rounds_six_sample_half_ties_from_exact_hit_counts() {
    let covered = (0..6).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
    assert_eq!(covered, 5, "the fixture must exercise the exact 5/6 alpha half tie");
    let baseline =
        complete_presentation(alpha_fixture(Some(6), 1, true, Some(job::OutputOptions { linear_exr: true, ..Default::default() }), 1.5), 1);
    for tone_map in [job::ToneMap::Reinhard, job::ToneMap::AcesFitted] {
        for denoise in [false, true] {
            for creative in [false, true] {
                let mut finishing = job::finishing::Finishing::default();
                if creative {
                    finishing.rgb_gain = [1.2, 0.8, 1.1];
                    finishing.contrast = 1.1;
                    finishing.saturation = 0.7;
                    finishing.vignette = 0.2;
                    finishing.looks.push(job::looks::LookLayer { kind: job::looks::LookKind::Warm, strength: 0.4, enabled: true }).unwrap();
                }
                let make = |samples, retain| {
                    let output = job::OutputOptions { tone_map, finishing, linear_exr: retain, ..Default::default() };
                    let original = alpha_fixture(Some(samples), 1, true, Some(output), 1.5);
                    let options = RaytraceOptions { denoise, ..original.options.clone() };
                    let request = ImageRequest { raytrace: Some(options.clone()), ..original.request.clone() };
                    ImageTask::new(original.scene, request, original.cancel, options).unwrap()
                };
                let actual = complete_presentation(make(6, true), 1);
                assert_eq!(actual, complete_presentation(make(6, true), 16384));
                assert_eq!(alpha_pixel(&actual.1)[3], 213, "5/6 must round upwards at exactly 212.5");
                for (samples, png) in actual.0.iter().chain(std::iter::once(&(6, actual.1.clone()))) {
                    let hits = (0..*samples).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
                    let expected = (hits as f64 / f64::from(*samples) * 255.0).round() as u8;
                    assert_eq!(alpha_pixel(png)[3], expected, "sample{samples}, {tone_map:?}, denoise={denoise}, creative={creative}");
                    assert_eq!(*png, complete_presentation(make(*samples, true), 3).1);
                }
                assert_eq!(actual.2, baseline.2, "presentation and denoising must preserve every raw EXR bit");
                let no_master = complete_presentation(make(6, false), 1);
                assert_eq!((actual.0, actual.1), (no_master.0, no_master.1), "raw retention must not change PNG coverage");
            }
        }
    }
}

#[test]
fn cpu_alpha_retains_opaque_only_effect_guards_and_cancellable_completion() {
    let original = alpha_fixture(Some(3), 1, true, None, 0.0);
    let creative = creative_output();
    for output in [
        job::OutputOptions { backdrop: Some(Default::default()), ..Default::default() },
        job::OutputOptions { finishing: job::finishing::Finishing { bloom: creative.finishing.bloom, ..Default::default() }, ..Default::default() },
        job::OutputOptions {
            finishing: job::finishing::Finishing { photographic: creative.finishing.photographic, ..Default::default() },
            ..Default::default()
        },
    ] {
        let options = RaytraceOptions { output: Some(Box::new(output)), ..original.options.clone() };
        let request = ImageRequest { raytrace: Some(options.clone()), ..original.request.clone() };
        assert!(admit(&original.scene, &request, &options).is_err(), "alpha never silently strips opaque photographic/bloom/backdrop intent");
    }
    for stage in ["exr", "display", "png"] {
        for cancel in [false, true] {
            let output = job::OutputOptions { tone_map: job::ToneMap::AcesFitted, linear_exr: true, ..Default::default() };
            let mut task = alpha_fixture(Some(3), 1, true, Some(output), 0.0);
            for _ in 0..200000 {
                if task.output.first().is_some_and(|output| output.stage_name() == stage) {
                    break;
                }
                assert!(!matches!(task.step_with_previews(1).unwrap(), ImagePoll::Complete(_)));
            }
            assert_eq!(task.output.first().unwrap().stage_name(), stage);
            let before = task.work.used();
            if cancel {
                task.cancel.cancel();
            } else {
                task.work.limit = before;
            }
            assert_eq!(task.step_with_previews(1).err().unwrap().code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
            assert_eq!(task.work.used(), before);
            assert!(!task.done);
            assert!(task.take_linear_exr().is_none());
        }
    }
}

#[test]
fn cpu_disabled_denoise_preserves_every_png_exr_work_unit_and_buffer_admission() {
    for output in [None, Some(job::OutputOptions { linear_exr: true, ..Default::default() })] {
        for samples in [None, Some(5)] {
            let original = presentation_fixture(samples, 1, output, 1.5);
            let options: RaytraceOptions = serde_json::from_value(serde_json::json!({"denoise":false})).unwrap();
            assert!(!options.denoise);
            assert!(serde_json::to_value(options).unwrap().get("denoise").is_none());
            let mut options = original.options.clone();
            options.denoise = false;
            let request = ImageRequest { raytrace: Some(options.clone()), ..original.request.clone() };
            let explicit = ImageTask::new(original.scene.clone(), request, CancelToken::default(), options).unwrap();
            assert_eq!(original.denoise.capacity(), 0);
            assert_eq!(explicit.denoise.capacity(), 0);
            assert_eq!(original.admission.estimated_total_bytes, explicit.admission.estimated_total_bytes);
            assert_eq!(complete_presentation(original, 1), complete_presentation(explicit, 16384));
        }
    }
}

#[test]
fn cpu_alpha_and_denoise_compose_without_changing_matte_prefixes_or_raw_masters() {
    let output = job::OutputOptions { linear_exr: true, ..Default::default() };
    let composed = |samples, transparent, denoise| {
        let original = alpha_fixture(Some(samples), 1, transparent, Some(output), 1.5);
        let options = RaytraceOptions { denoise, ..original.options.clone() };
        let request = ImageRequest { raytrace: Some(options.clone()), ..original.request.clone() };
        ImageTask::new(original.scene, request, original.cancel, options).unwrap()
    };
    let actual = complete_presentation(composed(7, true, true), 1);
    assert_eq!(actual, complete_presentation(composed(7, true, true), 16384));
    assert_eq!(actual.0.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 6]);
    let rgb = [9.0, 4.5, 2.25].map(|radiance| {
        let exposed = radiance * 2.0f64.powf(1.5);
        encode8(exposed / (1.0 + exposed))
    });
    for (samples, png) in actual.0.iter().chain(std::iter::once(&(7, actual.1.clone()))) {
        let covered = (0..*samples).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
        let alpha = (covered as f64 / f64::from(*samples) * 255.0).round() as u8;
        assert_eq!(alpha_pixel(png), if alpha == 0 { [0; 4] } else { [rgb[0], rgb[1], rgb[2], alpha] });
        assert_eq!(*png, complete_presentation(composed(*samples, true, true), 3).1);
    }
    let unfiltered = complete_presentation(composed(7, true, false), 1);
    let opaque = complete_presentation(composed(7, false, true), 1);
    assert_eq!(actual.2, unfiltered.2, "denoising cannot enter the premultiplied raw companion");
    assert_eq!(actual.2, opaque.2, "PNG alpha choice cannot enter the premultiplied raw companion");
    assert_eq!((actual.0, actual.1), (unfiltered.0, unfiltered.1), "partial-coverage/emitter bypass retains the analytical straight matte");
    let source = composed(7, true, true);
    let creative = creative_output();
    for incompatible in [
        job::OutputOptions { backdrop: Some(Default::default()), ..output },
        job::OutputOptions { finishing: job::finishing::Finishing { bloom: creative.finishing.bloom, ..Default::default() }, ..output },
        job::OutputOptions { finishing: job::finishing::Finishing { photographic: creative.finishing.photographic, ..Default::default() }, ..output },
    ] {
        let options = RaytraceOptions { output: Some(Box::new(incompatible)), ..source.options.clone() };
        let request = ImageRequest { raytrace: Some(options.clone()), ..source.request.clone() };
        assert!(admit(&source.scene, &request, &options).is_err(), "the denoiser cannot erase an alpha-incompatible authored effect");
    }
}

fn path_fixture(samples: u32, depth: u8, ceiling: bool) -> ImageTask {
    let original = fixture(1, 1, Some(true));
    let quad = |z: f64, down: bool| Mesh {
        positions: vec![Vec3::new(-1e6, -1e6, z), Vec3::new(1e6, -1e6, z), Vec3::new(1e6, 1e6, z), Vec3::new(-1e6, 1e6, z)],
        normals: vec![if down { -Vec3::Z } else { Vec3::Z }; 4],
        triangles: if down { vec![[0, 2, 1], [0, 3, 2]] } else { vec![[0, 1, 2], [0, 2, 3]] },
        tri_face: vec![0; 2],
        ..Default::default()
    };
    let mut body = original.scene.meshes[0].clone();
    body.mesh = Arc::new(quad(0.0, false));
    body.surface = Some(OpaqueSurface::new(Vec3::new(0.5, 0.5, 0.5), 0.0, 1.0, 1.0).unwrap());
    let mut meshes = vec![body];
    if ceiling {
        let mut emitter = meshes[0].clone();
        emitter.name = "Red ceiling".into();
        // A real camera aperture keeps the emitter out of the primary path even
        // when the producer starts orthographic rays before the complete bounds.
        // Four downward-facing quads surround the 1x1 mm opening at z20.
        let mut ceiling = Mesh::default();
        for [x0, x1, y0, y1] in [[-1e6, -0.5, -1e6, 1e6], [0.5, 1e6, -1e6, 1e6], [-0.5, 0.5, -1e6, -0.5], [-0.5, 0.5, 0.5, 1e6]] {
            let base = ceiling.positions.len() as u32;
            ceiling.positions.extend([Vec3::new(x0, y0, 20.0), Vec3::new(x1, y0, 20.0), Vec3::new(x1, y1, 20.0), Vec3::new(x0, y1, 20.0)]);
            ceiling.normals.extend([-Vec3::Z; 4]);
            ceiling.triangles.extend([[base, base + 2, base + 1], [base, base + 3, base + 2]]);
            ceiling.tri_face.extend([0; 2]);
        }
        emitter.mesh = Arc::new(ceiling);
        emitter.surface = Some(OpaqueSurface::new(Vec3::ZERO, 0.0, 1.0, 1.0).unwrap());
        meshes.push(emitter);
    }
    let scene = ProductScene { bounds: meshes.iter().fold(Aabb3::EMPTY, |b, m| b.union(&m.mesh.bounds())), meshes };
    let options = RaytraceOptions {
        path: Some(job::CpuPathOptions { max_depth: depth }),
        budget: job::CpuBudget::Studio,
        progressive: Some(job::CpuProgressiveOptions { samples_per_pixel: samples, seed: 9 }),
        shadows: true,
        lights: Some(vec![]),
        environment: (!ceiling).then_some(job::RaytraceEnvironmentOptions { color: [1.0; 3], intensity: 1.0 }),
        emissions: ceiling.then_some(vec![job::SurfaceEmission {
            body_index: 1,
            body: "Red ceiling".into(),
            color: [1.0, 0.0, 0.0],
            strength: 2.0,
            sides: job::EmissionSides::Both,
        }]),
        ..Default::default()
    };
    // This fixture's independent GGX-mixture oracles assume exactly normal view.
    // StandardView::Top deliberately stops 1e-6 rad short of the pole, which
    // changes reflected proposals and their MIS density at depth >= 2.
    // Its 0.8x0.8 mm orthographic footprint fits inside the physical ceiling
    // aperture. No private clipping override or modified producer is needed.
    let camera = crate::Camera { distance: 1.0, yaw: 0.0, pitch: std::f64::consts::FRAC_PI_2, roll: 0.0, ..original.request.camera };
    assert!((camera.ray(0.5, 0.5, 1.0, 1.0).1 + Vec3::Z).len() < 1e-15, "analytical normal-view fixture");
    let request = ImageRequest { camera, raytrace: Some(options.clone()), ..original.request };
    ImageTask::new(scene, request, CancelToken::default(), options).unwrap()
}
fn path_mean(mut task: ImageTask) -> ([f64; 3], u64) {
    let target = task.options.progressive.unwrap().samples_per_pixel;
    for _ in 0..2_000_000 {
        if task.progressive.as_ref().is_some_and(|p| p.film.samples() == target) {
            let (mean, covered, samples) = task.progressive.as_ref().unwrap().film.pixel(0).unwrap();
            assert_eq!(u32::from(samples), target);
            assert_eq!(covered, samples);
            return (mean, task.work.used());
        }
        let path_step = task.bvh.is_some()
            && task.progressive.as_ref().is_some_and(|p| p.film.ready() && !p.resolving)
            && task.pixel == 0
            && task.output.is_empty()
            && task.encoder.is_none();
        let before = task.work.used();
        assert!(!matches!(task.step_with_previews(1).unwrap(), ImagePoll::Complete(_)));
        if path_step {
            assert!(task.work.used() - before <= 1, "each real path continuation consumes at most one unit");
        }
    }
    panic!("bounded path film did not complete");
}
fn independent_lambert_scatter(ordinal: u32) -> (f64, f64) {
    let [component, u] = progressive::variates(9, 0, ordinal, 0x5041_5448_3000);
    let cosine = if component < 0.5 { (1.0 - u).sqrt() } else { 1.0 - 2.0 * u };
    if cosine <= 0.0 {
        return (0.0, 0.0);
    }
    let pdf = 0.5 * cosine / std::f64::consts::PI + 0.5 / (4.0 * std::f64::consts::PI);
    (cosine, pdf)
}
#[test]
fn cpu_path_real_furnace_and_colored_bounce_match_independent_per_ordinal_estimators() {
    use std::f64::consts::PI;
    let samples = 1024;
    for depth in [1, 2] {
        let mut expected = 0.0;
        for ordinal in 0..samples {
            let [u, _] = progressive::variates(9, 0, ordinal, 0x5041_5448_2000);
            let z = 1.0 - 2.0 * u;
            if z > 0.0 {
                let light_pdf = 1.0 / (4.0 * PI);
                let bsdf = 0.5 * z / PI + 0.5 / (4.0 * PI);
                let weight = if depth == 1 { 1.0 } else { light_pdf * light_pdf / (light_pdf * light_pdf + bsdf * bsdf) };
                expected += 2.0 * z * weight;
            }
            if depth == 2 {
                let (cosine, pdf) = independent_lambert_scatter(ordinal);
                if pdf > 0.0 {
                    let light_pdf = 1.0 / (4.0 * PI);
                    expected += 0.5 * cosine / (PI * pdf) * (pdf * pdf / (pdf * pdf + light_pdf * light_pdf));
                }
            }
        }
        expected /= f64::from(samples);
        let (actual, _) = path_mean(path_fixture(samples, depth, false));
        for value in actual {
            assert!((value - expected).abs() < 1e-11, "depth{depth}: {value} vs independent estimator{expected}");
        }
        assert!((expected - 0.5).abs() < 0.1, "Lambert furnace expected0.5; fixed-seed bounded estimate{expected}");
    }
    let receiver = |depth| {
        let mut task = path_fixture(samples, depth, true);
        for _ in 0..100_000 {
            if task.bvh.is_some() {
                break;
            }
            assert!(matches!(task.step_with_previews(1).unwrap(), ImagePoll::Pending(_)));
        }
        let bvh = task.bvh.as_ref().expect("actual colored-bounce BVH must be ready before lighting");
        let prepared = PreparedScene { bvh, primitives: &task.looks, source: &task.scene };
        // Read-only diagnostics use a separate budget: neither film ordinals nor
        // the producer's cumulative work are consumed by these hit assertions.
        let mut work = WorkBudget::new(100_000).unwrap();
        let mut secondary_hits = 0;
        for ordinal in 0..samples {
            let [dx, dy] = progressive::variates(9, 0, ordinal, 0);
            let ray = primary_ray(&task.request, dx, dy, task.clip).unwrap();
            let aperture = ray.origin() + ray.direction() * ((20.0 - ray.origin().z) / ray.direction().z);
            assert!(aperture.x.abs() < 0.5 && aperture.y.abs() < 0.5, "primary must pass through the real ceiling opening");
            let hit = bvh.closest_hit(&ray, &mut work, &task.cancel).unwrap().expect("camera must see the receiver");
            let hit = prepared.resolve_hit(ray, hit).unwrap();
            assert_eq!(hit.body_index, 0, "primary must see the receiver, never its red ceiling");
            assert_eq!(hit.body_name, "Black half");
            assert!(hit.front_face);
            assert_eq!(hit.geometric_normal, Vec3::Z);
            assert_eq!(hit.shading_normal, Vec3::Z);
            assert_eq!(hit.position.z, 0.0);
            let [component, x] = progressive::variates(9, 0, ordinal, 0x5041_5448_3000);
            let [y, _] = progressive::variates(9, 0, ordinal, 0x5041_5448_3001);
            let secondary =
                super::super::indirect::sample(hit.surface, hit.shading_normal, hit.oriented_geometric_normal(), -ray.direction(), [component, x, y])
                    .unwrap();
            assert_eq!(secondary.is_some(), independent_lambert_scatter(ordinal).1 > 0.0, "the fixed oracle retains every null proposal");
            if let Some(secondary) = secondary {
                let ray = hit.spawn(secondary.direction).unwrap();
                let second = bvh
                    .closest_hit(&ray, &mut work, &task.cancel)
                    .unwrap()
                    .expect("every accepted secondary must see the red ceiling outside its opening");
                let second = prepared.resolve_hit(ray, second).unwrap();
                assert_eq!(second.body_index, 1);
                assert_eq!(second.body_name, "Red ceiling");
                assert!(second.front_face);
                assert_eq!(second.geometric_normal, -Vec3::Z);
                assert_eq!(second.shading_normal, -Vec3::Z);
                assert!(second.vertices.iter().all(|vertex| vertex.z == 20.0), "the secondary hit belongs to the exact ceiling plane");
                assert!(second.position.x.abs() > 0.5 || second.position.y.abs() > 0.5);
                secondary_hits += 1;
            }
        }
        assert_eq!(secondary_hits, 771, "the fixed seed exercises the complete accepted continuation set without retries");
        assert_eq!(task.progressive.as_ref().unwrap().film.samples(), 0, "no lighting sample was consumed by preparation or diagnostics");
        assert!(task.options.environment.is_none());
        assert!(task.options.lights.as_ref().unwrap().is_empty());
        assert!(task.options.emissions.as_ref().unwrap().iter().all(|emitter| emitter.body_index != 0));
        task
    };
    let direct = path_mean(receiver(1)).0;
    assert_eq!(direct, [0.0; 3], "no direct source illuminates the receiver");
    let indirect = path_mean(receiver(2)).0;
    let expected = (0..samples)
        .map(|ordinal| {
            let (cosine, pdf) = independent_lambert_scatter(ordinal);
            if pdf == 0.0 { 0.0 } else { cosine / (PI * pdf) }
        })
        .sum::<f64>()
        / f64::from(samples);
    assert!((indirect[0] - expected).abs() < 1e-11, "actual neighbor emission must travel through diffuse continuation");
    assert_eq!(indirect[1..], [0.0, 0.0]);
    assert!((expected - 1.0).abs() < 0.12);
}
#[test]
fn cpu_path_real_prefixes_quanta_admission_and_legacy_omission_are_stable() {
    let output = job::OutputOptions { linear_exr: true, ..Default::default() };
    let make = |samples| {
        let original = path_fixture(samples, 3, false);
        let options = RaytraceOptions { output: Some(Box::new(output)), ..original.options };
        let request = ImageRequest { raytrace: Some(options.clone()), ..original.request };
        ImageTask::new(original.scene, request, original.cancel, options).unwrap()
    };
    let actual = complete_presentation(make(7), 1);
    assert_eq!(actual, complete_presentation(make(7), 16384));
    for (samples, png) in &actual.0 {
        assert_eq!(*png, complete_presentation(make(*samples), 3).1);
    }
    assert!(actual.2.is_some());
    let task = make(3);
    assert_eq!(task.path.capacity(), 1);
    let options = RaytraceOptions { path: None, ..task.options.clone() };
    let request = ImageRequest { raytrace: Some(options.clone()), ..task.request.clone() };
    let old = ImageTask::new(task.scene.clone(), request, CancelToken::default(), options).unwrap();
    assert_eq!(old.path.capacity(), 0);
    assert_eq!(task.admission.estimated_total_bytes - old.admission.estimated_total_bytes, std::mem::size_of::<indirect_image::State>() as u64);
    let omitted: RaytraceOptions = serde_json::from_str("{}").unwrap();
    assert!(omitted.path.is_none());
    assert!(serde_json::to_value(&omitted).unwrap().get("path").is_none());
    assert_eq!(complete_presentation(old, 1), {
        let mut wire = serde_json::to_value(&task.options).unwrap();
        wire.as_object_mut().unwrap().remove("path");
        let options: RaytraceOptions = serde_json::from_value(wire).unwrap();
        let request = ImageRequest { raytrace: Some(options.clone()), ..task.request };
        complete_presentation(ImageTask::new(task.scene, request, CancelToken::default(), options).unwrap(), 16384)
    });
}
#[test]
fn cpu_path_cancellation_and_exhaustion_never_commit_an_incomplete_camera_path() {
    for stage in [
        "new",
        "trace",
        "emitters",
        "resolve",
        "effects",
        "emission",
        "directional",
        "punctual",
        "area",
        "environment",
        "shadow",
        "scatter",
        "commit",
    ] {
        for cancel in [false, true] {
            let mut task = path_fixture(7, 3, false);
            for _ in 0..200_000 {
                if task.bvh.is_some() && task.path.first().is_some_and(|p| p.stage_name() == stage) {
                    break;
                }
                assert!(!matches!(task.step_with_previews(1).unwrap(), ImagePoll::Complete(_)));
            }
            assert_eq!(task.path.first().unwrap().stage_name(), stage);
            let before = task.work.used();
            let samples = task.progressive.as_ref().unwrap().film.samples();
            if cancel {
                task.cancel.cancel();
            } else {
                task.work.limit = before;
            }
            assert_eq!(task.step_with_previews(1).err().unwrap().code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
            assert_eq!(task.work.used(), before);
            assert_eq!(task.progressive.as_ref().unwrap().film.samples(), samples);
            assert!(task.take_linear_exr().is_none());
            assert!(!task.done);
        }
    }
}

#[test]
fn cpu_path_real_rectangular_emitter_and_blocker_have_the_analytic_lambert_mean() {
    let expected = 0.5 * 4.0 / 5.0_f64.sqrt() * (1.0 / 5.0_f64.sqrt()).atan() / std::f64::consts::PI;
    for direct_samples in [1, 4, 16] {
        // z=2 is the exact emitter endpoint: opaque geometry wins BSDF
        // distance ties, so direct visibility must include that endpoint too.
        for blocker_height in [None, Some(1.0), Some(2.0)] {
            let original = path_fixture(1024, 2, false);
            let mut scene = original.scene;
            if let Some(height) = blocker_height {
                let mut occluder = scene.meshes[0].clone();
                occluder.name = "Black blocker".into();
                let mut mesh = (*occluder.mesh).clone();
                for p in &mut mesh.positions {
                    p.z = height;
                }
                occluder.mesh = Arc::new(mesh);
                occluder.surface = Some(OpaqueSurface::new(Vec3::ZERO, 0.0, 1.0, 1.0).unwrap());
                scene.meshes.push(occluder);
                scene.bounds = scene.meshes.iter().fold(Aabb3::EMPTY, |b, m| b.union(&m.mesh.bounds()));
            }
            let options = RaytraceOptions {
                environment: None,
                area_lights: Some(vec![job::RectangleLightOptions {
                    center_mm: [0.0, 0.0, 2.0],
                    edge_u_mm: [2.0, 0.0, 0.0],
                    edge_v_mm: [0.0, -2.0, 0.0],
                    color: [1.0; 3],
                    intensity: 1.0,
                }]),
                area_samples: Some(direct_samples),
                seed: Some(81),
                ..original.options
            };
            let camera = crate::Camera { distance: 0.1, fov: 0.001, ..original.request.camera };
            let request = ImageRequest { camera, raytrace: Some(options.clone()), ..original.request };
            let (actual, _) = path_mean(ImageTask::new(scene, request, CancelToken::default(), options).unwrap());
            if blocker_height.is_some() {
                assert_eq!(actual, [0.0; 3], "black geometry at {blocker_height:?} blocks both strategies, including endpoint ties");
            } else {
                for channel in actual {
                    assert!((channel - expected).abs() < 0.02, "N={direct_samples}, real rectangle MIS {channel}, analytic {expected}");
                }
            }
        }
    }
}
#[test]
fn cpu_path_refuses_mixed_transport_instead_of_erasing_selected_optical_data() {
    let original = path_fixture(3, 2, false);
    let reflection =
        ReflectionOptions { bodies: vec![ReflectionBody { body: "Black half".into(), mode: ReflectionMode::Mirror }], samples: 1, max_bounces: 2 };
    let options = RaytraceOptions { reflections: Some(reflection.clone()), ..original.options.clone() };
    let request = ImageRequest { raytrace: Some(options.clone()), ..original.request.clone() };
    assert!(admit(&original.scene, &request, &options).unwrap_err().message.contains("selected reflection"));
    assert_eq!(options.reflections, Some(reflection));
    for depth in [0, 17] {
        assert!(RaytraceOptions { path: Some(job::CpuPathOptions { max_depth: depth }), ..original.options.clone() }.validate().is_err());
    }
    assert!(RaytraceOptions { progressive: None, ..original.options.clone() }.validate().unwrap_err().message.contains("progressive"));
    assert!(RaytraceOptions { shadows: false, ..original.options.clone() }.validate().unwrap_err().message.contains("shadows"));
}

#[test]
fn cpu_path_overlapping_coplanar_emitters_refuse_without_mutating_authored_rows() {
    let original = path_fixture(3, 2, false);
    let first = job::RectangleLightOptions {
        center_mm: [0.0, 0.0, 2.0],
        edge_u_mm: [2.0, 0.0, 0.0],
        edge_v_mm: [0.0, -2.0, 0.0],
        color: [1.0; 3],
        intensity: 1.0,
    };
    for center in [[0.0, 0.0, 2.0], [1.0, 0.0, 2.0]] {
        let second = job::RectangleLightOptions { center_mm: center, ..first.clone() };
        let rows = vec![first.clone(), second];
        let options = RaytraceOptions { area_lights: Some(rows.clone()), ..original.options.clone() };
        let error = options.validate().unwrap_err();
        assert!(error.message.contains("overlapping"));
        assert_eq!(options.area_lights, Some(rows), "refusal retains both authored sources");
    }
    for center in [[2.0, 0.0, 2.0], [3.0, 0.0, 2.0], [0.0, 0.0, 3.0]] {
        let options = RaytraceOptions {
            area_lights: Some(vec![first.clone(), job::RectangleLightOptions { center_mm: center, ..first.clone() }]),
            ..original.options.clone()
        };
        assert!(options.validate().is_ok(), "disjoint or edge-adjacent supports remain valid");
    }
    let opposite = job::RectangleLightOptions { edge_v_mm: [0.0, 2.0, 0.0], ..first.clone() };
    assert!(
        RaytraceOptions { area_lights: Some(vec![first, opposite]), ..original.options }.validate().is_ok(),
        "opposite one-sided supports illuminate disjoint hemispheres"
    );
}

#[test]
fn cpu_path_real_hdr_source_counts_match_independent_per_ordinal_mis_and_texel_lookup() {
    use crate::lighting::{HdrSource, environment_input::EnvironmentInput};
    use std::f64::consts::{PI, TAU};
    // Deliberately nonconstant RGBE cells: the reference selects a texel from the
    // physical direction independently of Environment::sample/radiance/PDF.
    let texels: [[u8; 4]; 8] = [
        [128, 32, 16, 129],
        [16, 128, 32, 129],
        [32, 16, 128, 129],
        [64, 96, 128, 129],
        [96, 64, 32, 129],
        [32, 96, 64, 129],
        [64, 32, 96, 129],
        [128, 64, 96, 129],
    ];
    let radiance = |direction: Vec3| {
        let phi = direction.y.atan2(direction.x).rem_euclid(TAU);
        let x = ((phi / TAU * 4.0) as usize).min(3);
        let y = ((direction.z.clamp(-1.0, 1.0).acos() / PI * 2.0) as usize).min(1);
        let pixel = texels[y * 4 + x];
        Vec3::new(f64::from(pixel[0]), f64::from(pixel[1]), f64::from(pixel[2])) / 128.0
    };
    let samples = 128;
    for source_count in [1, 4, 16, 64] {
        let side = match source_count {
            1 => 1,
            4 => 2,
            16 => 4,
            _ => 8,
        };
        for depth in [1, 2] {
            let mut expected = Vec3::ZERO;
            for ordinal in 0..samples {
                for source_sample in 0..source_count {
                    let [dx, dy] = progressive::variates(9, 0, ordinal, 0x5041_5448_2000 + u64::from(source_sample));
                    let u = ((f64::from(source_sample / side) + dx) / f64::from(side)).max(0.0_f64.next_up()).min(1.0_f64.next_down());
                    let v = ((f64::from(source_sample % side) + dy) / f64::from(side)).max(0.0_f64.next_up()).min(1.0_f64.next_down());
                    let z = 1.0 - 2.0 * u;
                    if z > 0.0 {
                        let phi = TAU * v;
                        let radius = ((1.0 - z) * (1.0 + z)).sqrt();
                        let direction = Vec3::new(radius * phi.cos(), radius * phi.sin(), z);
                        let source_pdf = f64::from(source_count) / (4.0 * PI);
                        let bsdf_pdf = 0.5 * z / PI + 0.5 / (4.0 * PI);
                        let weight = if depth == 1 { 1.0 } else { source_pdf * source_pdf / (source_pdf * source_pdf + bsdf_pdf * bsdf_pdf) };
                        expected += radiance(direction) * (2.0 * z * weight / f64::from(source_count));
                    }
                }
                if depth == 2 {
                    let (z, pdf) = independent_lambert_scatter(ordinal);
                    if pdf > 0.0 {
                        let [angle, _] = progressive::variates(9, 0, ordinal, 0x5041_5448_3001);
                        let phi = TAU * angle;
                        let radius = (1.0 - z * z).sqrt();
                        let direction = Vec3::new(radius * phi.cos(), radius * phi.sin(), z);
                        let source_pdf = f64::from(source_count) / (4.0 * PI);
                        let weight = pdf * pdf / (pdf * pdf + source_pdf * source_pdf);
                        expected += radiance(direction) * (0.5 * z / (PI * pdf) * weight);
                    }
                }
            }
            expected = expected / f64::from(samples);
            let original = path_fixture(samples, depth, false);
            let options = RaytraceOptions { environment: None, ..original.options };
            let request = ImageRequest { raytrace: Some(options.clone()), ..original.request };
            let mut bytes = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\nPRIMARIES=0.64 0.33 0.30 0.60 0.15 0.06 0.3127 0.3290\n\n-Y 2 +X 4\n".to_vec();
            for pixel in texels {
                bytes.extend_from_slice(&pixel);
            }
            let input = EnvironmentInput::new(HdrSource::new(bytes).unwrap(), 0.0, 1.0, source_count).unwrap();
            let (actual, _) =
                path_mean(ImageTask::new_with_environment(original.scene, request, CancelToken::default(), options, Some(input)).unwrap());
            for (actual, expected) in actual.into_iter().zip([expected.x, expected.y, expected.z]) {
                assert!((actual - expected).abs() < 1e-11, "HDR N={source_count}, depth={depth}: {actual} vs independent {expected}");
            }
        }
    }
}

#[test]
fn composed_cpu_path_alpha_and_denoise_keep_analytic_matte_all_prefixes_and_raw_master() {
    let output = job::OutputOptions { linear_exr: true, ..Default::default() };
    let make = |samples, transparent, denoise, path: bool| {
        let original = alpha_fixture(Some(samples), 1, transparent, Some(output), 1.5);
        let options =
            RaytraceOptions { path: path.then_some(job::CpuPathOptions { max_depth: 1 }), denoise, shadows: true, ..original.options.clone() };
        let request = ImageRequest { raytrace: Some(options.clone()), ..original.request.clone() };
        ImageTask::new(original.scene, request, original.cancel, options).unwrap()
    };
    let actual = complete_presentation(make(7, true, true, true), 1);
    assert_eq!(actual, complete_presentation(make(7, true, true, true), 16384));
    assert_eq!(actual.0.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 6]);
    let rgb = [9.0, 4.5, 2.25].map(|radiance| {
        let exposed = radiance * 2.0f64.powf(1.5);
        encode8(exposed / (1.0 + exposed))
    });
    for (samples, png) in actual.0.iter().chain(std::iter::once(&(7, actual.1.clone()))) {
        let covered = (0..*samples).filter(|ordinal| progressive::variates(9, 0, *ordinal, 0)[0] < 0.5).count();
        let alpha = (covered as f64 / f64::from(*samples) * 255.0).round() as u8;
        assert_eq!(alpha_pixel(png), if alpha == 0 { [0; 4] } else { [rgb[0], rgb[1], rgb[2], alpha] });
        assert_eq!(*png, complete_presentation(make(*samples, true, true, true), 3).1);
    }
    let unfiltered = complete_presentation(make(7, true, false, true), 1);
    let opaque = complete_presentation(make(7, false, true, true), 1);
    let direct = complete_presentation(make(7, true, true, false), 1);
    assert_eq!(actual.2, unfiltered.2, "filtering cannot enter the GI raw companion");
    assert_eq!(actual.2, opaque.2, "background removal cannot enter the GI raw companion");
    assert_eq!(actual.2, direct.2, "a camera-visible emitter has the same raw radiance and coverage in either transport");
    assert_eq!((actual.0, actual.1), (unfiltered.0, unfiltered.1), "the partial-coverage emitter bypass keeps the analytic straight matte");
}

fn composed_panel_fixture(samples: u32, denoise: bool, path: bool, panel_z: f64, front: bool) -> ImageTask {
    let original = path_fixture(samples, 1, false);
    let point = |x, y| {
        let (origin, _) = original.request.camera.ray(x, y, 13.0, 13.0);
        Vec3::new(origin.x, origin.y, panel_z)
    };
    let corner = point(5.0, 2.0);
    let u = point(8.0, 2.0) - corner;
    let mut v = point(5.0, 11.0) - corner;
    let center = corner + (u + v) * 0.5;
    if (u.cross(v).z > 0.0) != front {
        v = -v;
    }
    let array = |p: Vec3| [p.x, p.y, p.z];
    let lights = (0..4)
        .map(|index| job::RectangleLightOptions {
            center_mm: array(center + Vec3::X * (100.0 * f64::from(index))),
            edge_u_mm: array(u),
            edge_v_mm: array(v),
            color: [0.5, 0.4, 0.3],
            intensity: 1.0,
        })
        .collect();
    let options = RaytraceOptions {
        denoise,
        path: path.then_some(job::CpuPathOptions { max_depth: 1 }),
        area_lights: Some(lights),
        area_samples: Some(1),
        seed: Some(31),
        output: Some(Box::new(job::OutputOptions { linear_exr: true, ..Default::default() })),
        ..original.options
    };
    let request = ImageRequest { width: 13, height: 13, raytrace: Some(options.clone()), ..original.request };
    ImageTask::new(original.scene, request, CancelToken::default(), options).unwrap()
}
fn composed_panel_guides(mut task: ImageTask) -> (Vec<job::denoise::Guide>, u64) {
    for _ in 0..300_000 {
        if task.denoise.first().is_some_and(|guides| guides.ready()) {
            assert_eq!(task.pixel, 0);
            assert_eq!(task.progressive.as_ref().unwrap().film.samples(), 0);
            return (task.denoise.first().unwrap().pixels().unwrap().to_vec(), task.work.used());
        }
        let guiding = task.bvh.is_some();
        let before = task.work.used();
        assert!(matches!(task.step_with_previews(1).unwrap(), ImagePoll::Pending(_)));
        if guiding {
            assert_eq!(task.work.used() - before, 1, "each guide ray/traversal/source/effect/store continuation owns one work unit");
        }
    }
    panic!("bounded composed guide prepass did not finish");
}
#[test]
fn composed_path_denoise_guides_bypass_visible_analytic_emitters_before_backing_geometry() {
    let (guides, work) = composed_panel_guides(composed_panel_fixture(4, true, true, 2.0, true));
    for y in 0..13 {
        for x in 0..13 {
            assert_eq!(guides[y * 13 + x].body == 0, (5..8).contains(&x) && (2..11).contains(&y), "guide ({x},{y})");
        }
    }
    let (legacy, old_work) = composed_panel_guides(composed_panel_fixture(4, true, false, 2.0, true));
    assert!(legacy.iter().all(|guide| guide.body == 1), "absent path keeps analytic lights invisible to the first-hit prepass");
    assert_eq!(
        work - old_work,
        13 * 13 * 4 - 3 * 9,
        "four source scans per pixel; each of the27 bypassed panel pixels skips the underlying body-guide resolution"
    );
    let original = composed_panel_fixture(4, true, false, 2.0, true);
    let options = RaytraceOptions { area_lights: None, area_samples: None, seed: None, ..original.options };
    let request = ImageRequest { raytrace: Some(options.clone()), ..original.request };
    let no_lights = composed_panel_guides(ImageTask::new(original.scene, request, original.cancel, options).unwrap());
    assert_eq!(no_lights.1, old_work, "legacy guide work does not scan direct-only analytic lights");
    for (z, front) in [(0.0, true), (-2.0, true), (2.0, false)] {
        let (guides, _) = composed_panel_guides(composed_panel_fixture(4, true, true, z, front));
        assert!(
            guides.iter().all(|guide| guide.body == 1),
            "geometry wins exact ties, nearer body hits and back-facing emitter supports; z={z}, front={front}"
        );
    }
}
#[test]
fn composed_path_denoise_keeps_all_foreground_emitter_pixels_prefixes_and_raw_master_exact() {
    let actual = complete_presentation(composed_panel_fixture(4, true, true, 2.0, true), 1);
    assert_eq!(actual, complete_presentation(composed_panel_fixture(4, true, true, 2.0, true), 16384));
    assert_eq!(actual.0.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 2, 3]);
    let raw = complete_presentation(composed_panel_fixture(4, false, true, 2.0, true), 1);
    assert_eq!(actual.2, raw.2, "emitter bypass and filtering cannot alter raw GI EXR radiance");
    assert_ne!(actual.1, raw.1, "the noisy diffuse backing must actually exercise the filter");
    let rgb = [0.5_f64, 0.4, 0.3].map(|radiance| encode8(radiance / (1.0 + radiance)));
    for ((samples, png), (raw_samples, raw_png)) in
        actual.0.iter().chain(std::iter::once(&(4, actual.1.clone()))).zip(raw.0.iter().chain(std::iter::once(&(4, raw.1.clone()))))
    {
        assert_eq!(samples, raw_samples);
        assert_eq!(*png, complete_presentation(composed_panel_fixture(*samples, true, true, 2.0, true), 3).1);
        let filtered = ::image::load_from_memory(png).unwrap().to_rgba8();
        let unfiltered = ::image::load_from_memory(raw_png).unwrap().to_rgba8();
        for y in 2..11 {
            for x in 5..8 {
                assert_eq!(filtered.get_pixel(x, y), unfiltered.get_pixel(x, y), "sample{samples}, panel({x},{y})");
                assert_eq!(filtered.get_pixel(x, y).0, [rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
}
#[test]
fn composed_path_guide_emitter_scans_cancel_and_exhaust_before_consuming_the_next_source() {
    for cancel in [false, true] {
        let mut task = composed_panel_fixture(4, true, true, 2.0, true);
        for _ in 0..300_000 {
            if task.denoise.first().is_some_and(|guide| guide.emitter_scans() == 2) {
                break;
            }
            assert!(matches!(task.step_with_previews(1).unwrap(), ImagePoll::Pending(_)));
        }
        let guide = task.denoise.first().unwrap();
        assert_eq!(guide.emitter_scans(), 2);
        assert!(!guide.ready_to_store(&task.options), "remaining emitter scans precede the actual guide store");
        assert_eq!(guide.completed_pixels(), 0);
        let used = task.work.used();
        if cancel {
            task.cancel.cancel();
        } else {
            task.work.limit = used;
        }
        assert_eq!(task.step_with_previews(1).err().unwrap().code, if cancel { ErrorCode::Cancelled } else { ErrorCode::LimitExceeded });
        assert_eq!(task.work.used(), used);
        let guide = task.denoise.first().unwrap();
        assert_eq!(guide.emitter_scans(), 2);
        assert!(!guide.ready_to_store(&task.options), "remaining emitter scans precede the actual guide store");
        assert_eq!(guide.completed_pixels(), 0);
        assert_eq!(task.progressive.as_ref().unwrap().film.samples(), 0);
        assert!(task.take_linear_exr().is_none());
    }
}
