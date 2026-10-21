//! The render passes of one frame: one pass, or the backdrop's passes when it holds glass.

use std::ops::Range;

use crate::{Frame, Renderer, Result};

impl Renderer {
    pub(crate) fn encode(
        &self,
        view: &wgpu::TextureView,
        frame: &Frame,
    ) -> Result<wgpu::CommandBuffer> {
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let all = 0..frame.layers.len();
        let backdrop = frame.backdrop().zip(self.backdrop.targets());
        match backdrop {
            None => {
                let mut pass = begin(&mut encoder, view, clear(frame));
                self.draw_layers(&mut pass, all, None)?;
            }
            Some((cut, targets)) => {
                {
                    let mut pass = begin(&mut encoder, targets.scene(), clear(frame));
                    self.draw_layers(&mut pass, 0..cut, None)?;
                }
                self.backdrop.blur(&mut encoder, targets);
                let mut pass = begin(&mut encoder, view, clear(frame));
                self.backdrop.copy(&mut pass, targets);
                self.draw_layers(&mut pass, cut..all.end, Some(targets.blurred()))?;
            }
        }
        Ok(encoder.finish())
    }

    fn draw_layers(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        layers: Range<usize>,
        blurred: Option<&wgpu::BindGroup>,
    ) -> Result<()> {
        for layer in layers {
            self.veils.draw(layer, pass, self.size);
            if let Some(blurred) = blurred {
                self.glass.draw(layer, pass, blurred, self.size);
            }
            self.quads.draw(layer, pass, self.size);
            self.text.draw(layer, pass)?;
        }
        Ok(())
    }
}

fn begin<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    clear: wgpu::Color,
) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("frame"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
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
