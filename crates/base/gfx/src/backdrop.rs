//! What lies under the glass: the layers below it drawn into a texture, then blurred.

use crate::Size;

/// Levels under the full frame, each half the size of the one above it.
const LEVELS: usize = 3;

pub(crate) struct Backdrop {
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    copy: wgpu::RenderPipeline,
    targets: Option<Targets>,
}

/// The full-size scene and the levels under it, made on the first frame with glass.
pub(crate) struct Targets {
    size: Size,
    scene: Level,
    levels: Vec<Level>,
}

struct Level {
    view: wgpu::TextureView,
    read: wgpu::BindGroup,
}

impl Backdrop {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = source_layout(device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blur.wgsl").into()),
        });
        let made = |entry| fullscreen(device, &layout, &shader, entry, format);
        Self {
            down: made("down"),
            up: made("up"),
            copy: made("copy"),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("backdrop"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            format,
            layout,
            targets: None,
        }
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn targets(&self) -> Option<&Targets> {
        self.targets.as_ref()
    }

    /// Makes the textures for a frame of `size`, or keeps the ones already that size.
    pub fn fit(&mut self, device: &wgpu::Device, size: Size) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let level = |size| self.level(device, size);
        let scene = level(size);
        let levels = (1..=LEVELS).map(|n| level(halved(size, n))).collect();
        self.targets = Some(Targets {
            size,
            scene,
            levels,
        });
    }

    /// Blurs the scene down the levels and back up to half its size.
    pub fn blur(&self, encoder: &mut wgpu::CommandEncoder, t: &Targets) {
        let mut from = &t.scene.read;
        for level in &t.levels {
            fill(encoder, &self.down, from, &level.view);
            from = &level.read;
        }
        for level in t.levels.iter().rev().skip(1) {
            fill(encoder, &self.up, from, &level.view);
            from = &level.read;
        }
    }

    /// Draws the scene as it was, under what the pass draws next.
    pub fn copy(&self, pass: &mut wgpu::RenderPass<'_>, t: &Targets) {
        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, &t.scene.read, &[]);
        pass.draw(0..3, 0..1);
    }

    fn level(&self, device: &wgpu::Device, size: Size) -> Level {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("backdrop"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let read = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backdrop"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        Level { view, read }
    }
}

impl Targets {
    pub fn scene(&self) -> &wgpu::TextureView {
        &self.scene.view
    }

    pub fn blurred(&self) -> &wgpu::BindGroup {
        self.levels.first().map_or(&self.scene.read, |l| &l.read)
    }
}

/// `size` halved `n` times, never under a pixel.
fn halved(size: Size, n: usize) -> Size {
    let cut = |v: u32| (v >> n).max(1);
    Size::new(cut(size.width), cut(size.height))
}

fn fill(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    from: &wgpu::BindGroup,
    to: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("blur"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: to,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, from, &[]);
    pass.draw(0..3, 0..1);
}

/// A texture and its sampler, what each blur step and the glass read.
fn source_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("backdrop"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn fullscreen(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    entry: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("blur"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("blur"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
