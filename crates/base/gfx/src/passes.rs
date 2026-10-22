//! The render passes of one frame: one pass, or a backdrop blurred again for each layer of glass.

use std::ops::Range;

use wgpu::LoadOp;

use crate::{Frame, Renderer, Result};

impl Renderer {
    pub(crate) fn encode(
        &self,
        view: &wgpu::TextureView,
        frame: &Frame,
    ) -> Result<wgpu::CommandBuffer> {
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let end = frame.layers.len();
        let cuts: Vec<usize> = frame.glassed().collect();
        let (Some(&first), Some(targets)) = (cuts.first(), self.backdrop.targets()) else {
            {
                let mut pass = begin(&mut encoder, view, LoadOp::Clear(clear(frame)));
                self.draw_layers(&mut pass, 0..end, None)?;
            }
            return Ok(encoder.finish());
        };
        {
            let mut pass = begin(&mut encoder, targets.scene(), LoadOp::Clear(clear(frame)));
            self.draw_layers(&mut pass, 0..first, None)?;
            self.unders.draw(first, &mut pass, self.size);
        }
        for (at, &cut) in cuts.iter().enumerate() {
            self.backdrop.blur(&mut encoder, targets);
            let blurred = Some(targets.blurred());
            match cuts.get(at + 1) {
                Some(&next) => {
                    let mut pass = begin(&mut encoder, targets.scene(), LoadOp::Load);
                    self.over_backdrop(&mut pass, cut..next, blurred)?;
                    self.unders.draw(next, &mut pass, self.size);
                }
                None => {
                    let mut pass = begin(&mut encoder, view, LoadOp::Clear(clear(frame)));
                    self.backdrop.copy(&mut pass, targets);
                    self.over_backdrop(&mut pass, cut..end, blurred)?;
                }
            }
        }
        Ok(encoder.finish())
    }

    /// `layers` from a layer of glass, whose grounds under it the backdrop already holds.
    fn over_backdrop(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        layers: Range<usize>,
        blurred: Option<&wgpu::BindGroup>,
    ) -> Result<()> {
        let rest = layers.start + 1..layers.end;
        self.layer(pass, layers.start, blurred)?;
        self.draw_layers(pass, rest, blurred)
    }

    fn draw_layers(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        layers: Range<usize>,
        blurred: Option<&wgpu::BindGroup>,
    ) -> Result<()> {
        for layer in layers {
            self.unders.draw(layer, pass, self.size);
            self.layer(pass, layer, blurred)?;
        }
        Ok(())
    }

    /// One layer but its grounds under the glass: the veil, the glass, quads, then text.
    fn layer(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        layer: usize,
        blurred: Option<&wgpu::BindGroup>,
    ) -> Result<()> {
        self.veils.draw(layer, pass, self.size);
        if let Some(blurred) = blurred {
            self.glass.draw(layer, pass, blurred, self.size);
        }
        self.quads.draw(layer, pass, self.size);
        self.text.draw(layer, pass)
    }
}

fn begin<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    load: LoadOp<wgpu::Color>,
) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("frame"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
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
