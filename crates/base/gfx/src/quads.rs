//! The rounded-rect pipeline every quad and border draws through.

use std::ops::Range;

use crate::{Color, Rect, Shape, Size};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
    /// Where the vertex stands from the whole rect's centre, in pixels.
    local: [f32; 2],
    half: [f32; 2],
    /// The corners' radius, and the width of the edge alone drawn; none fills it.
    shape: [f32; 2],
}

/// Consecutive quads under one scissor.
struct Batch {
    clip: Rect,
    range: Range<u32>,
}

/// Every quad of the frame in one vertex buffer, drawn layer by layer.
pub(crate) struct QuadPass {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    verts: Vec<Vertex>,
    layers: Vec<Vec<Batch>>,
}

impl QuadPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            pipeline: pipeline(device, format),
            buffer: buffer(device, 6 * 1024),
            verts: Vec::new(),
            layers: Vec::new(),
        }
    }

    pub fn begin(&mut self, layers: usize) {
        self.verts.clear();
        self.layers.clear();
        self.layers.resize_with(layers, Vec::new);
    }

    pub fn push(
        &mut self,
        layer: usize,
        rect: Rect,
        color: Color,
        alpha: f32,
        clip: Rect,
        size: Size,
    ) {
        let plain = Shape::default();
        self.push_shaped(layer, (rect, plain), color, alpha, clip, size);
    }

    /// A quad cut to `shape`: its corners rounded, or only its edge drawn.
    pub fn push_shaped(
        &mut self,
        layer: usize,
        (whole, shape): (Rect, Shape),
        color: Color,
        alpha: f32,
        clip: Rect,
        size: Size,
    ) {
        let rect = whole.intersect(clip);
        if rect.is_empty() || color.is_transparent() {
            return;
        }
        let start = self.verts.len() as u32;
        let mut color = color.linear();
        color[3] *= alpha;
        push_vertices(&mut self.verts, (rect, whole), color, shape, size);
        let end = self.verts.len() as u32;
        let batches = &mut self.layers[layer];
        match batches.last_mut() {
            Some(last) if last.clip == clip => last.range.end = end,
            _ => batches.push(Batch {
                clip,
                range: start..end,
            }),
        }
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

    pub fn draw(&self, layer: usize, pass: &mut wgpu::RenderPass<'_>, size: Size) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        for batch in &self.layers[layer] {
            let Some((x, y, w, h)) = batch.clip.scissor(size) else {
                continue;
            };
            pass.set_scissor_rect(x, y, w, h);
            pass.draw(batch.range.clone(), 0..1);
        }
        pass.set_scissor_rect(0, 0, size.width, size.height);
    }
}

/// The part `r` of rectangle `whole`, in pixels, as two triangles in clip space.
fn push_vertices(
    verts: &mut Vec<Vertex>,
    (r, whole): (Rect, Rect),
    color: [f32; 4],
    shape: Shape,
    size: Size,
) {
    let ndc = |x: f32, y: f32| {
        [
            x / size.width as f32 * 2.0 - 1.0,
            1.0 - y / size.height as f32 * 2.0,
        ]
    };
    let half = [whole.w / 2.0, whole.h / 2.0];
    let centre = (whole.x + half[0], whole.y + half[1]);
    let (x0, y0, x1, y1) = (r.x, r.y, r.right(), r.bottom());
    for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y0), (x1, y1), (x0, y1)] {
        verts.push(Vertex {
            pos: ndc(x, y),
            color,
            local: [x - centre.0, y - centre.1],
            half,
            shape: [shape.radius, shape.stroke],
        });
    }
}

fn buffer(device: &wgpu::Device, vertices: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quads"),
        size: (vertices * std::mem::size_of::<Vertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("quads"),
        source: wgpu::ShaderSource::Wgsl(include_str!("quads.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("quads"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("quads"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2, 1 => Float32x4, 2 => Float32x2, 3 => Float32x2, 4 => Float32x2
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
