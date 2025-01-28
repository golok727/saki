use std::{
    future::Future,
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    task::Waker,
};

use anyhow::Result;

use parking_lot::Mutex;
use skie_math::Size;
use wgpu::{BufferAsyncError, Maintain};

use crate::GpuContext;

use super::Canvas;

pub type CanvasSnapshotResult = Result<CanvasSnapshot, BufferAsyncError>;

// TODO: allow config
pub trait CanvasSnapshotSource {
    fn get_output_texture(&self) -> wgpu::Texture;

    fn read_texture_data_async(
        &self,
        canvas: &Canvas,
        on_data: impl FnOnce() + Send + 'static,
    ) -> Receiver<CanvasSnapshotResult> {
        let source_texture = self.get_output_texture();
        let size = Size {
            width: source_texture.width(),
            height: source_texture.height(),
        };

        let gpu = canvas.renderer.gpu();

        let (sender, receiver) = mpsc::channel::<CanvasSnapshotResult>();

        read_texels_async(gpu, &source_texture, move |res| {
            if sender
                .send(res.map(|data| CanvasSnapshot { size, data }))
                .is_ok()
            {
                on_data()
            }
        });

        receiver
    }

    fn snapshot_sync(&self, canvas: &Canvas) -> CanvasSnapshotResult {
        let receiver = self.read_texture_data_async(canvas, || {});

        canvas.renderer.gpu().device.poll(Maintain::Wait);

        if let Ok(res) = receiver.recv() {
            res
        } else {
            Err(BufferAsyncError)
        }
    }

    fn snapshot(&self, canvas: &Canvas) -> CanvasSnapshotAsync {
        let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));

        let gpu = canvas.renderer.gpu();

        let receiver = self.read_texture_data_async(canvas, {
            let waker = waker.clone();
            move || {
                if let Some(waker) = waker.lock().take() {
                    waker.wake()
                }
            }
        });

        while !gpu.device.poll(wgpu::Maintain::Poll).is_queue_empty() {}

        CanvasSnapshotAsync { waker, receiver }
    }
}

// TODO use Image instead
pub struct CanvasSnapshot {
    pub size: Size<u32>,
    pub data: Vec<u8>,
}

pub struct CanvasSnapshotAsync {
    waker: Arc<Mutex<Option<Waker>>>,
    receiver: Receiver<CanvasSnapshotResult>,
}

impl Future for CanvasSnapshotAsync {
    type Output = CanvasSnapshotResult;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Ok(snapshot) = self.receiver.try_recv() {
            std::task::Poll::Ready(snapshot)
        } else {
            let mut thing = self.waker.lock();
            *thing = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}

impl Canvas {
    pub fn snapshot_sync<Source: CanvasSnapshotSource>(
        &self,
        source: &Source,
    ) -> CanvasSnapshotResult {
        source.snapshot_sync(self)
    }
    pub async fn snapshot<Source: CanvasSnapshotSource>(
        &self,
        source: &Source,
    ) -> CanvasSnapshotResult {
        source.snapshot(self).await
    }
}

fn read_texels_async(
    gpu: &GpuContext,
    src: &wgpu::Texture,
    read: impl FnOnce(Result<Vec<u8>, BufferAsyncError>) + Send + 'static,
) {
    let buffer_size = (src.width() * src.height() * 4) as u64; // 4 bytes per pixel (RGBA8
                                                               //
    let output_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = gpu.create_command_encoder(Some("Command Encoder"));

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: src,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(src.width() * 4),
                rows_per_image: Some(src.height()),
            },
        },
        wgpu::Extent3d {
            width: src.width(),
            height: src.height(),
            depth_or_array_layers: 1,
        },
    );

    // Submit the commands
    gpu.queue.submit(Some(encoder.finish()));
    let buffer_slice = output_buffer.slice(..);

    buffer_slice.map_async(wgpu::MapMode::Read, {
        let buffer = output_buffer.clone();
        move |res| {
            let data = buffer.slice(..).get_mapped_range();
            let res = res.map(|_| data.to_vec());
            read(res)
        }
    });
}
