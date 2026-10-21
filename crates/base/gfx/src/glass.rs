//! The glass pipeline: each pane refracts the backdrop under it through a curved rim.

use std::ops::Range;

use crate::{Glass, Rect, Size};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    tint: [f32; 4],
    local: [f32; 2],
    half: [f32; 2],
    /// The corners' radius, then the rim's width.
    shape: [f32; 2],
    screen: [f32; 2],
}

struct Batch {
    clip: Rect,
    range: Range<u32>,
}

/// Every pane of the frame in one vertex buffer, drawn layer by layer.
pub(crate) struct GlassPass {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    verts: Vec<Vertex>,
    layers: Vec<Vec<Batch>>,
}

impl GlassPass {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        blurred: &wgpu::BindGroupLayout,
    ) -> Self {
        Self {
            pipeline: pipeline(device, format, blurred),
            buffer: buffer(device, 6 * 16),
            verts: Vec::new(),
            layers: Vec::new(),
        }
    }

    pub fn begin(&mut self, layers: usize) {
        self.verts.clear();
        self.layers.clear();
        self.layers.resize_with(layers, Vec::new);
    }

    pub fn push(&mut self, layer: usize, glass: &Glass, size: Size) {
        let rect = glass.rect.inset(-glass.bezel).intersect(glass.clip);
        if rect.is_empty() {
            return;
        }
        let start = self.verts.len() as u32;
        push_vertices(&mut self.verts, (rect, glass), size);
        self.layers[layer].push(Batch {
            clip: glass.clip,
            range: start..self.verts.len() as u32,
        });
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let bytes: &[u8] = bytemuck::cast_slice(&self.verts);
        if bytes.is_empty() {
            return;
        }
        if (bytes.len() as u64) > self.buffer.size() {
            self.buffer = buffer(device, self.verts.len() * 2);
        }
        queue.write_buffer(&self.buffer, 0, bytes);
    }

    pub fn draw(
        &self,
        layer: usize,
        pass: &mut wgpu::RenderPass<'_>,
        blurred: &wgpu::BindGroup,
        size: Size,
    ) {
        let batches = &self.layers[layer];
        if batches.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, blurred, &[]);
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        for batch in batches {
            let Some((x, y, w, h)) = batch.clip.scissor(size) else {
                continue;
            };
            pass.set_scissor_rect(x, y, w, h);
            pass.draw(batch.range.clone(), 0..1);
        }
        pass.set_scissor_rect(0, 0, size.width, size.height);
    }
}

/// The part `r` of the pane and its shadow, in pixels, as two triangles in clip space.
fn push_vertices(verts: &mut Vec<Vertex>, (r, glass): (Rect, &Glass), size: Size) {
    let screen = [size.width as f32, size.height as f32];
    let ndc = |x: f32, y: f32| [x / screen[0] * 2.0 - 1.0, 1.0 - y / screen[1] * 2.0];
    let whole = glass.rect;
    let half = [whole.w / 2.0, whole.h / 2.0];
    let centre = (whole.x + half[0], whole.y + half[1]);
    let tint = glass.tint.linear();
    let (x0, y0, x1, y1) = (r.x, r.y, r.right(), r.bottom());
    for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y0), (x1, y1), (x0, y1)] {
        verts.push(Vertex {
            pos: ndc(x, y),
            tint,
            local: [x - centre.0, y - centre.1],
            half,
            shape: [glass.radius, glass.bezel],
            screen,
        });
    }
}

fn buffer(device: &wgpu::Device, vertices: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glass"),
        size: (vertices * std::mem::size_of::<Vertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    blurred: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("glass"),
        source: wgpu::ShaderSource::Wgsl(include_str!("glass.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("glass"),
        bind_group_layouts: &[Some(blurred)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("glass"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2, 1 => Float32x4, 2 => Float32x2,
                    3 => Float32x2, 4 => Float32x2, 5 => Float32x2
                ],
            })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
