//! Copies the offscreen texture back for the tests. Nothing else reads a frame.

use crate::renderer::Target;
use crate::{Frame, Renderer, Result, Size};

pub struct Image {
    pub size: Size,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.size.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

impl Renderer {
    pub(crate) fn snapshot(&mut self, frame: &Frame) -> Result<Image> {
        self.render(frame)?;
        let Target::Offscreen { texture } = &self.target else {
            unreachable!("snapshots are offscreen")
        };
        let size = self.size();
        let bytes_per_row = (size.width * 4).next_multiple_of(256);
        let out = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * size.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &out,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(size.height),
                },
            },
            texture.size(),
        );
        self.gpu.queue.submit([encoder.finish()]);

        let slice = out.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.gpu.wait();
        let data = slice.get_mapped_range().expect("mapped readback");
        let row = (size.width * 4) as usize;
        let rgba = (0..size.height as usize)
            .flat_map(|y| {
                let start = y * bytes_per_row as usize;
                data[start..start + row].iter().copied()
            })
            .collect();
        drop(data);
        out.unmap();
        Ok(Image { size, rgba })
    }
}
