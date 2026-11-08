//! Optional output filtering; never writes the Monte Carlo accumulation buffer.
use super::{Context, Result, SHADER, TIMEOUT, Task, checked, placeholder_bind, scopes, warm_up};
use solidcraft_render::job::{DenoiseDiagnostics, DenoiseGuideClass, resolve_denoise_class};
use std::sync::atomic::{AtomicBool, Ordering};
use wgpu::util::DeviceExt;

pub(super) const GUIDE_BYTES_PER_PIXEL: u64 = 96; // two complete surface guides
pub(super) const BYTES_PER_PIXEL: u64 = GUIDE_BYTES_PER_PIXEL + 32; // two colour vec4s
const FILTER: &str = include_str!("denoise.wgsl");
const GUIDES: &str = include_str!("denoise_guides.wgsl");
#[cfg(test)]
#[path = "denoise_tests.rs"]
mod tests;

// Shared by every denoised job on a device: the guide pipeline compiles the whole
// path shader, so it is built and warmed once rather than per job.
pub(super) struct Pipelines {
    guide: wgpu::ComputePipeline,
    filter: wgpu::ComputePipeline,
    classify: wgpu::ComputePipeline,
    filter_layout: wgpu::BindGroupLayout,
}

impl Pipelines {
    fn new(context: &Context) -> Result<Self> {
        let device = &context.device;
        let error_scopes = scopes(device);
        let guide_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("denoise guides"),
            source: wgpu::ShaderSource::Wgsl(format!("{SHADER}\n{GUIDES}").into()),
        });
        let guide_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("denoise guides"),
            bind_group_layouts: &[Some(&context.layout)],
            immediate_size: 0,
        });
        let guide = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("denoise guides"),
            layout: Some(&guide_layout),
            module: &guide_shader,
            entry_point: Some("denoise_guides"),
            compilation_options: wgpu::PipelineCompilationOptions { constants: &[("HAS_TEXTURES", 1.0)], ..Default::default() },
            cache: None,
        });
        let entries: Vec<_> = (0..4)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 0 { wgpu::BufferBindingType::Uniform } else { wgpu::BufferBindingType::Storage { read_only: binding != 3 } },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let filter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("denoise filter"), entries: &entries });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("denoise filter"),
            bind_group_layouts: &[Some(&filter_layout)],
            immediate_size: 0,
        });
        let shader = device
            .create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("denoise filter"), source: wgpu::ShaderSource::Wgsl(FILTER.into()) });
        let filter = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("denoise filter"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("atrous"),
            compilation_options: Default::default(),
            cache: None,
        });
        let classify = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("denoise guide classes"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("guide_classes"),
            compilation_options: Default::default(),
            cache: None,
        });
        checked(error_scopes)?;
        warm_up(device, &context.queue, &guide, &placeholder_bind(device, &context.layout, 7)?, &context.healthy)?;
        warm_up(device, &context.queue, &filter, &placeholder_bind(device, &filter_layout, 4)?, &context.healthy)?;
        warm_up(device, &context.queue, &classify, &placeholder_bind(device, &filter_layout, 4)?, &context.healthy)?;
        Ok(Self { guide, filter, classify, filter_layout })
    }

    fn shared(context: &Context) -> Result<&Self> {
        if let Some(pipelines) = context.denoise.get() {
            return Ok(pipelines);
        }
        let built = Self::new(context)?;
        Ok(context.denoise.get_or_init(|| built))
    }
}

pub(super) struct Resources {
    adaptive: Option<AdaptiveInput>,
    guides_ready: AtomicBool,
    guide_pipeline: wgpu::ComputePipeline,
    guide_bind: wgpu::BindGroup,
    filter_pipeline: wgpu::ComputePipeline,
    classify_pipeline: wgpu::ComputePipeline,
    filter_uniform: wgpu::Buffer,
    filter_binds: Vec<wgpu::BindGroup>,
    output: wgpu::Buffer,
    temporary: wgpu::Buffer,
    // Retained for diagnostic tests; bind groups also retain the guide allocation.
    _guides: wgpu::Buffer,
}

impl Resources {
    pub(super) fn new(
        context: &Context,
        path_uniform: &wgpu::Buffer,
        scene: [&wgpu::Buffer; 5],
        accumulation: &wgpu::Buffer,
        size: u64,
        adaptive_preview: bool,
    ) -> Result<Self> {
        let pipelines = Pipelines::shared(context)?;
        let device = &context.device;
        let error_scopes = scopes(device);
        let buffer = |label, bytes| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let guides = buffer("denoise first-hit guides", size * (GUIDE_BYTES_PER_PIXEL / 16));
        let output = buffer("denoise filtered colour", size);
        let temporary = buffer("denoise intermediate colour", size);
        let [triangles, nodes, environment, textures, failures] = scene;
        let entries: Vec<_> = [path_uniform, triangles, nodes, &guides, environment, textures, failures]
            .into_iter()
            .enumerate()
            .map(|(binding, value)| wgpu::BindGroupEntry { binding: binding as u32, resource: value.as_entire_binding() })
            .collect();
        let guide_bind =
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("denoise guides"), layout: &context.layout, entries: &entries });
        let filter_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("denoise parameters"),
            contents: &[0; 32],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let filter_binds = [(accumulation, &output), (&output, &temporary), (&temporary, &output)]
            .into_iter()
            .map(|(source, destination)| {
                let entries: Vec<_> = [&filter_uniform, &guides, source, destination]
                    .into_iter()
                    .enumerate()
                    .map(|(binding, value)| wgpu::BindGroupEntry { binding: binding as u32, resource: value.as_entire_binding() })
                    .collect();
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("denoise filter"),
                    layout: &pipelines.filter_layout,
                    entries: &entries,
                })
            })
            .collect();
        let adaptive = adaptive_preview.then(|| AdaptiveInput::new(context, pipelines, accumulation, &guides, &filter_uniform, &output, size));
        checked(error_scopes)?;
        Ok(Self {
            adaptive,
            guides_ready: AtomicBool::new(false),
            guide_pipeline: pipelines.guide.clone(),
            guide_bind,
            filter_pipeline: pipelines.filter.clone(),
            classify_pipeline: pipelines.classify.clone(),
            filter_uniform,
            filter_binds,
            output,
            temporary,
            _guides: guides,
        })
    }

    pub(super) fn output(&self, passes: u32) -> &wgpu::Buffer {
        if passes == 2 { &self.temporary } else { &self.output }
    }

    pub(super) fn run(&self, task: &Task) -> Result<()> {
        self.run_with_input(task, false)
    }
    pub(super) fn run_adaptive_preview(&self, task: &Task) -> Result<()> {
        self.adaptive.as_ref().ok_or_else(|| super::gpu("missing adaptive denoise preview input"))?.normalize(task)?;
        self.run_with_input(task, true)
    }
    fn run_with_input(&self, task: &Task, preview: bool) -> Result<()> {
        self.ensure_guides(task)?;
        self.run_filter(task, preview)
    }

    fn ensure_guides(&self, task: &Task) -> Result<()> {
        // Each job owns immutable scene, camera and material inputs. Once all
        // guide tiles finish, every checkpoint can reuse them.
        if !self.guides_ready.load(Ordering::Relaxed) {
            self.generate_guides(task)?;
            self.guides_ready.store(true, Ordering::Relaxed);
        }
        task.cancel.check()
    }

    fn generate_guides(&self, task: &Task) -> Result<()> {
        let pixels = task.request.width * task.request.height;
        // The guide entry point uses sampling.w for optical transport modes.
        // Transport dispatches write their own complete uniform block.
        let guide_flags = u32::from(task.photons.is_some()) | (u32::from(task.optical_resolve.is_some()) << 1);
        let mut budget = super::dispatch::Budget::default();
        let mut offset = 0;
        while offset < pixels {
            task.cancel.check()?;
            let count = budget.count(pixels - offset);
            let mut parameters = task.parameters(count)?;
            if let Some(bytes) = parameters.get_mut(12..16) {
                bytes.copy_from_slice(&offset.to_le_bytes());
            }
            if let Some(bytes) = parameters.get_mut(28..32) {
                bytes.copy_from_slice(&guide_flags.to_le_bytes());
            }
            task.context.queue.write_buffer(&task.uniform, 0, &parameters);
            let started = std::time::Instant::now();
            dispatch(&task.context, &self.guide_pipeline, &self.guide_bind, count)?;
            budget.observe(count, started.elapsed());
            offset += count;
        }
        task.cancel.check()
    }

    /// Read exact guide classifications through the already admitted colour
    /// scratch/readback buffers. No image-sized JSON or second guide plane exists.
    pub(super) fn diagnostics(&self, task: &Task) -> Result<(DenoiseDiagnostics, Vec<u8>)> {
        self.ensure_guides(task)?;
        let pixels = task.request.width * task.request.height;
        let bind = self.filter_binds.first().ok_or_else(|| super::gpu("missing diagnostic bind group"))?;
        let mut budget = super::dispatch::Budget::default();
        let mut offset = 0;
        while offset < pixels {
            task.cancel.check()?;
            let count = budget.count(pixels - offset);
            let mut bytes = Vec::with_capacity(32);
            for word in [task.request.width, task.request.height, 0, offset, count, task.normalization_samples(), 0, 0] {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            task.context.queue.write_buffer(&self.filter_uniform, 0, &bytes);
            let started = std::time::Instant::now();
            dispatch(&task.context, &self.classify_pipeline, bind, count)?;
            budget.observe(count, started.elapsed());
            offset += count;
        }
        let context = &task.context;
        let byte_count = u64::from(pixels) * 16;
        let error_scopes = scopes(&context.device);
        let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("denoise mask readback") });
        encoder.copy_buffer_to_buffer(&self.output, 0, &task.readback, 0, byte_count);
        let submission = context.queue.submit([encoder.finish()]);
        checked(error_scopes)?;
        let slice = task.readback.slice(..byte_count);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        context.wait(submission, TIMEOUT)?;
        rx.try_recv().map_err(super::gpu)?.map_err(super::gpu)?;
        let data = slice.get_mapped_range().map_err(super::gpu)?;
        let result = (|| {
            let mut report = DenoiseDiagnostics::new(task.request.width, task.request.height);
            let output = &task.output_request;
            let capacity = usize::try_from(u64::from(output.width) * u64::from(output.height) * 4)
                .map_err(|_| super::gpu("denoise mask exceeds address space"))?;
            let mut rgba = super::scene::reserved(capacity)?;
            let scale = task.options.supersampling as usize;
            for y in 0..output.height as usize {
                task.cancel.check()?;
                for x in 0..output.width as usize {
                    let mut classes = [DenoiseGuideClass::Unavailable; 4];
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let pixel = (y * scale + dy) * task.request.width as usize + x * scale + dx;
                            let bytes: [u8; 4] = data
                                .get(pixel * 16..pixel * 16 + 4)
                                .and_then(|bytes| bytes.try_into().ok())
                                .ok_or_else(|| super::gpu("missing denoise guide classification"))?;
                            let code = f32::from_le_bytes(bytes);
                            if !code.is_finite() || code < 0.0 || code.fract() != 0.0 {
                                return Err(super::gpu("invalid denoise guide classification"));
                            }
                            let class =
                                DenoiseGuideClass::from_code(code as u32).ok_or_else(|| super::gpu("unknown denoise guide classification"))?;
                            report.record(class);
                            *classes.get_mut(dy * scale + dx).ok_or_else(|| super::gpu("invalid diagnostic supersample scale"))? = class;
                        }
                    }
                    let class = resolve_denoise_class(classes.into_iter().take(scale * scale));
                    rgba.extend_from_slice(&class.color());
                }
            }
            task.cancel.check()?;
            Ok((report, rgba))
        })();
        drop(data);
        task.readback.unmap();
        result
    }

    fn run_filter(&self, task: &Task, preview: bool) -> Result<()> {
        let pixels = task.request.width * task.request.height;
        let settings = task.options.denoise_settings;
        settings.validate()?;
        for (pass, bind) in self.filter_binds.iter().take(settings.passes as usize).enumerate() {
            let bind = if preview && pass == 0 {
                &self.adaptive.as_ref().ok_or_else(|| super::gpu("missing adaptive denoise preview input"))?.filter_bind
            } else {
                bind
            };
            let mut budget = super::dispatch::Budget::default();
            let mut offset = 0;
            while offset < pixels {
                task.cancel.check()?;
                let count = budget.count(pixels - offset);
                let mut bytes = Vec::with_capacity(32);
                for word in [
                    task.request.width,
                    task.request.height,
                    1 << pass,
                    offset,
                    count,
                    task.normalization_samples(),
                    (settings.strength as f32).to_bits(),
                    u32::from(settings.firefly_suppression),
                ] {
                    bytes.extend_from_slice(&word.to_le_bytes());
                }
                task.context.queue.write_buffer(&self.filter_uniform, 0, &bytes);
                let started = std::time::Instant::now();
                dispatch(&task.context, &self.filter_pipeline, bind, count)?;
                budget.observe(count, started.elapsed());
                offset += count;
            }
        }
        task.cancel.check()
    }
}

/// A distinct normalized input for adaptive display only. The final filter stays bound
/// to the original production accumulation, and this pass never writes that film.
struct AdaptiveInput {
    _pixels: wgpu::Buffer,
    uniform: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    bind: wgpu::BindGroup,
    filter_bind: wgpu::BindGroup,
}
impl AdaptiveInput {
    fn new(
        context: &Context,
        pipelines: &Pipelines,
        accumulation: &wgpu::Buffer,
        guides: &wgpu::Buffer,
        filter_uniform: &wgpu::Buffer,
        output: &wgpu::Buffer,
        size: u64,
    ) -> Self {
        let device = &context.device;
        let pixels = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adaptive preview denoise input"),
            size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("adaptive preview parameters"),
            contents: &[0; 32],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let entries: Vec<_> = (0..3)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 0 { wgpu::BufferBindingType::Uniform } else { wgpu::BufferBindingType::Storage { read_only: binding == 1 } },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("adaptive preview normalization"), entries: &entries });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("adaptive preview normalization"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("adaptive preview normalization"),
            source: wgpu::ShaderSource::Wgsl(include_str!("adaptive_preview.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("adaptive preview normalization"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let entries: Vec<_> = [&uniform, accumulation, &pixels]
            .into_iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry { binding: binding as u32, resource: buffer.as_entire_binding() })
            .collect();
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adaptive preview normalization"),
            layout: &layout,
            entries: &entries,
        });
        let entries: Vec<_> = [filter_uniform, guides, &pixels, output]
            .into_iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry { binding: binding as u32, resource: buffer.as_entire_binding() })
            .collect();
        let filter_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adaptive preview filter"),
            layout: &pipelines.filter_layout,
            entries: &entries,
        });
        Self { _pixels: pixels, uniform, pipeline, bind, filter_bind }
    }
    fn normalize(&self, task: &Task) -> Result<()> {
        let options = task.options.adaptive.ok_or_else(|| super::gpu("missing adaptive preview options"))?;
        let pixels = task.request.width * task.request.height;
        let mut budget = super::dispatch::Budget::default();
        let mut offset = 0;
        while offset < pixels {
            task.cancel.check()?;
            let count = budget.count(pixels - offset);
            let mut bytes = Vec::with_capacity(32);
            for word in [task.request.width, task.request.height, task.sample, offset, count, options.pilot_samples, 0, 0] {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            task.context.queue.write_buffer(&self.uniform, 0, &bytes);
            let started = std::time::Instant::now();
            dispatch(&task.context, &self.pipeline, &self.bind, count)?;
            budget.observe(count, started.elapsed());
            offset += count;
        }
        task.cancel.check()
    }
}

fn dispatch(context: &Context, pipeline: &wgpu::ComputePipeline, bind: &wgpu::BindGroup, count: u32) -> Result<()> {
    context.ensure_available()?;
    let error_scopes = scopes(&context.device);
    let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("denoise tile") });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("denoise tile"), timestamp_writes: None });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
    }
    let submission = context.queue.submit([encoder.finish()]);
    checked(error_scopes)?;
    context.wait(submission, TIMEOUT)?;
    Ok(())
}
