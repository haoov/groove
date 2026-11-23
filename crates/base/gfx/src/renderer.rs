//! The renderer: a surface or an offscreen target, and one frame onto it.

use crate::backdrop::Backdrop;
use crate::frame::{IconDraw, Layer, TextRun};
use crate::glass::GlassPass;
use crate::gpu::Gpu;
use crate::quads::QuadPass;
use crate::text::{Cached, TextPass};
use crate::{Error, Family, Fonts, Frame, Result, Size, grid};

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
    pub(crate) glass: GlassPass,
    pub(crate) backdrop: Backdrop,
    pub(crate) unders: QuadPass,
    pub(crate) veils: QuadPass,
    pub(crate) quads: QuadPass,
    pub(crate) text: TextPass,
    pub(crate) target: Target,
    format: wgpu::TextureFormat,
    pub(crate) size: Size,
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
        let gpu = Gpu::new()?;
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
        let backdrop = Backdrop::new(&gpu.device, format);
        Self {
            glass: GlassPass::new(&gpu.device, format, backdrop.layout()),
            backdrop,
            unders: QuadPass::new(&gpu.device, format),
            veils: QuadPass::new(&gpu.device, format),
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

    /// What the text pass has shaped and kept.
    pub fn cached(&self) -> Cached {
        self.text.cached()
    }

    pub fn fonts(&mut self) -> &mut Fonts {
        &mut self.fonts
    }

    /// The next frame shapes all its text again in this mono family, if it changed.
    pub fn set_mono(&mut self, mono: Family) {
        if self.fonts.set_mono(mono) {
            self.text.forget();
        }
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
        self.unders.begin(layers);
        self.veils.begin(layers);
        self.glass.begin(layers);
        self.quads.begin(layers);
        self.text.begin(layers, device, queue, self.size);
        for (i, layer) in frame.layers.iter().enumerate() {
            self.push_layer(i, layer);
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
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        if frame.backdrop().is_some() {
            self.backdrop.fit(device, self.size);
        }
        self.unders.upload(device, queue);
        self.veils.upload(device, queue);
        self.glass.upload(device, queue);
        self.quads.upload(device, queue);
        self.text.prepare(device, queue, &mut self.fonts)
    }
}

impl Renderer {
    /// A layer's grounds, glass, quads, text and icons, each at the layer's opacity.
    fn push_layer(&mut self, i: usize, layer: &Layer) {
        let opacity = 1.0 - layer.fade;
        backs((&mut self.unders, &mut self.veils), i, layer, self.size);
        for g in &layer.glass {
            self.glass.push(i, (g, opacity), self.size);
        }
        for q in &layer.quads {
            let shaped = (q.rect, q.shape);
            self.quads
                .push_shaped(i, shaped, q.color, opacity, q.clip, self.size);
        }
        for run in &layer.texts {
            match layer.fade > 0.0 {
                true => self
                    .text
                    .push_run(i, &mut self.fonts, &faded_run(run, opacity)),
                false => self.text.push_run(i, &mut self.fonts, run),
            }
        }
        for icon in &layer.icons {
            let color = icon.color.faded(opacity);
            self.text.push_icon(i, &IconDraw { color, ..*icon });
        }
    }
}

/// What a layer draws before its glass: the grounds under it, then its veil.
fn backs((unders, veils): (&mut QuadPass, &mut QuadPass), i: usize, layer: &Layer, size: Size) {
    let all = size.rect();
    let opacity = 1.0 - layer.fade;
    for &(rect, color, shape) in &layer.under {
        unders.push_shaped(i, (rect, shape), color, opacity, all, size);
    }
    if let Some(veil) = layer.veil {
        veils.push(i, all, veil, opacity, all, size);
    }
}

/// `run` with its colour taken down to `opacity`.
fn faded_run(run: &TextRun, opacity: f32) -> TextRun {
    let mut faded = run.clone();
    faded.style.color = run.style.color.faded(opacity);
    faded
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
