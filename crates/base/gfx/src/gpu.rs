//! The device, its queue and the adapter they came from.

use crate::Result;

/// The device and its queue.
pub(crate) struct Gpu {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// A device that draws offscreen.
    pub fn new() -> Result<Self> {
        Self::on(&wgpu::Instance::default(), None)
    }

    /// A device that draws on `target`, with the surface made for it.
    pub fn new_for(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
    ) -> Result<(Self, wgpu::Surface<'static>)> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(target)?;
        let gpu = Self::on(&instance, Some(&surface))?;
        Ok((gpu, surface))
    }

    /// The low-power adapter of `instance`, one that can draw on `surface` when given, and its device.
    fn on(instance: &wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<Self> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: surface,
            ..Default::default()
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
        Ok(Self {
            adapter,
            device,
            queue,
        })
    }

    #[cfg(test)]
    pub fn wait(&self) {
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
    }
}
