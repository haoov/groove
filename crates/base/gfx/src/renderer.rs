use crate::gpu::Gpu;
use crate::quads::QuadPass;
use crate::text::TextPass;
use crate::{Error, Fonts, Frame, Result, Size, grid};

pub(crate) const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub(crate) enum Target {
    Surface {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    Offscreen {
        texture: wgpu::Texture,
    },
}

/// Draws a `Frame` on a window, or on a texture without one.
pub struct Renderer {
    pub(crate) gpu: Gpu,
    fonts: Fonts,
    quads: QuadPass,
    text: TextPass,
    pub(crate) target: Target,
    format: wgpu::TextureFormat,
    size: Size,
}

impl Renderer {
    pub fn windowed(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        size: Size,
        fonts: Fonts,
    ) -> Result<Self> {
        let (gpu, surface) = Gpu::new_for(window)?;
        let config = surface_config(&gpu, &surface, size)?;
        let format = config
            .view_formats
            .first()
            .copied()
            .unwrap_or(config.format);
        surface.configure(&gpu.device, &config);
        let target = Target::Surface { surface, config };
        Ok(Self::build(gpu, target, format, size, fonts))
    }

    pub fn headless(size: Size, fonts: Fonts) -> Result<Self> {
        let gpu = Gpu::new(None)?;
        let texture = offscreen(&gpu.device, size);
        let target = Target::Offscreen { texture };
        Ok(Self::build(gpu, target, OFFSCREEN_FORMAT, size, fonts))
    }

    fn build(
        gpu: Gpu,
        target: Target,
        format: wgpu::TextureFormat,
        size: Size,
        fonts: Fonts,
    ) -> Self {
        Self {
            quads: QuadPass::new(&gpu.device, format),
            text: TextPass::new(&gpu.device, &gpu.queue, format),
            gpu,
            fonts,
            target,
            format,
            size: size.clamped(),
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn fonts(&mut self) -> &mut Fonts {
        &mut self.fonts
    }

    pub fn resize(&mut self, size: Size) {
        self.size = size.clamped();
        match &mut self.target {
            Target::Surface { surface, config } => {
                config.width = self.size.width;
                config.height = self.size.height;
                surface.configure(&self.gpu.device, config);
            }
            Target::Offscreen { texture } => *texture = offscreen(&self.gpu.device, self.size),
        }
    }

    pub fn render(&mut self, frame: &Frame) -> Result<()> {
        if frame.size != self.size {
            self.resize(frame.size);
        }
        match &self.target {
            Target::Surface { .. } => self.render_to_surface(frame),
            Target::Offscreen { texture } => {
                let view = texture.create_view(&Default::default());
                self.prepare(frame)?;
                let commands = self.encode(&view, frame)?;
                self.gpu.queue.submit([commands]);
                self.text.end();
                Ok(())
            }
        }
    }

    fn render_to_surface(&mut self, frame: &Frame) -> Result<()> {
        use wgpu::CurrentSurfaceTexture::{
            Lost, Occluded, Outdated, Suboptimal, Success, Timeout, Validation,
        };
        let Target::Surface { surface, .. } = &self.target else {
            return Ok(());
        };
        let current = match surface.get_current_texture() {
            Success(t) | Suboptimal(t) => t,
            Outdated | Lost => {
                self.resize(self.size);
                return Ok(());
            }
            Timeout | Occluded | Validation => return Ok(()),
        };
        let view = current.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.format),
            ..Default::default()
        });
        self.prepare(frame)?;
        let commands = self.encode(&view, frame)?;
        self.gpu.queue.submit([commands]);
        self.gpu.queue.present(current);
        self.text.end();
        Ok(())
    }

    fn prepare(&mut self, frame: &Frame) -> Result<()> {
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        let layers = frame.layers.len();
        self.quads.begin(layers);
        self.text.begin(layers, device, queue, self.size);
        for (i, layer) in frame.layers.iter().enumerate() {
            for q in &layer.quads {
                self.quads.push(i, q.rect, q.color, 1.0, q.clip, self.size);
            }
            for run in &layer.texts {
                self.text.push_run(i, &mut self.fonts, run);
            }
            for g in &layer.grids {
                let cell = self.fonts.cell_size(g.font_size);
                grid::push_grid(
                    i,
                    g,
                    cell,
                    &mut self.quads,
                    &mut self.text,
                    &mut self.fonts,
                    self.size,
                );
            }
        }
        self.quads.upload(device, queue);
        self.text.prepare(device, queue, &mut self.fonts)
    }

    fn encode(&self, view: &wgpu::TextureView, frame: &Frame) -> Result<wgpu::CommandBuffer> {
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear(frame)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for layer in 0..frame.layers.len() {
                self.quads.draw(layer, &mut pass, self.size);
                self.text.draw(layer, &mut pass)?;
            }
        }
        Ok(encoder.finish())
    }
}

fn clear(frame: &Frame) -> wgpu::Color {
    let [r, g, b, a] = frame.background.linear();
    wgpu::Color {
        r: f64::from(r),
        g: f64::from(g),
        b: f64::from(b),
        a: f64::from(a),
    }
}

/// The surface's own format, viewed as sRGB.
fn surface_config(
    gpu: &Gpu,
    surface: &wgpu::Surface<'_>,
    size: Size,
) -> Result<wgpu::SurfaceConfiguration> {
    let size = size.clamped();
    let mut config = surface
        .get_default_config(&gpu.adapter, size.width, size.height)
        .ok_or(Error::SurfaceFormat)?;
    let srgb = config.format.add_srgb_suffix();
    if srgb != config.format {
        config.view_formats = vec![srgb];
    }
    Ok(config)
}

fn offscreen(device: &wgpu::Device, size: Size) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OFFSCREEN_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
