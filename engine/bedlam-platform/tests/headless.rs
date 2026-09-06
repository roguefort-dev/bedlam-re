//! GPU smoke test: a real offscreen present of a canonical frame
//! through the parity pipeline. SKIPS (does not fail) when no wgpu
//! adapter exists, e.g. pure-CI containers.

use bedlam_platform::gpu::{ParityGpu, ParityPipeline};
use bedlam_platform::wgpu;
use bedlam_platform::PresentConfig;
use bedlam_render::{Frame, Vga6};

const TW: u32 = 1280;
const TH: u32 = 960;

fn test_palette() -> [Vga6; 256] {
    let mut p = [[0u8; 3]; 256];
    p[10] = [63, 0, 0];
    p[20] = [0, 63, 0];
    p
}

fn test_frame() -> Frame {
    let mut frame = Frame::new(test_palette());
    frame.palette_dirty = true;
    for y in 0..480u32 {
        for x in 0..640u32 {
            frame.set(x, y, if ((x / 8) ^ (y / 8)) & 1 == 0 { 10 } else { 20 });
        }
    }
    frame
}

/// Read the whole offscreen target back as RGBA8 bytes.
fn readback(gpu: &ParityGpu, target: &wgpu::Texture) -> Vec<u8> {
    let readback = gpu.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("test readback"),
        size: u64::from(TW * TH * 4),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = gpu
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("test readback enc"),
        });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TW * 4),
                rows_per_image: Some(TH),
            },
        },
        wgpu::Extent3d {
            width: TW,
            height: TH,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue().submit([enc.finish()]);
    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    gpu.device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(5)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    slice.get_mapped_range().to_vec()
}

fn make_target(gpu: &ParityGpu, format: wgpu::TextureFormat) -> wgpu::Texture {
    gpu.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("test target"),
        size: wgpu::Extent3d {
            width: TW,
            height: TH,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn px(data: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
    let i = (y * TW + x) as usize * 4;
    (data[i], data[i + 1], data[i + 2], data[i + 3])
}

#[test]
fn parity_offscreen_roundtrip() {
    let Some(gpu) = ParityGpu::new_headless() else {
        eprintln!("skip: no wgpu adapter on this host");
        return;
    };
    let mut pipeline = ParityPipeline::new(&gpu, wgpu::TextureFormat::Rgba8Unorm);

    // Pass 1: dirty palette (first upload).
    let frame = test_frame();
    pipeline.upload_frame(&frame);
    let target = make_target(&gpu, wgpu::TextureFormat::Rgba8Unorm);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let cb = pipeline.draw(&view, TW, TH, &PresentConfig::default());
    gpu.queue().submit([cb]);
    gpu.device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(5)),
        })
        .unwrap();
    let data = readback(&gpu, &target);

    // 2x integer scale, 8 px checkerboard: both probes sit deep inside
    // blocks. Original expansion: 63 -> 252, alpha opaque.
    // (100,100) -> src (50,50) -> cells (6,6) -> xor 0 -> index 10 red.
    assert_eq!(px(&data, 100, 100), (252, 0, 0, 255));
    // (116,100) -> src (58,50) -> cells (7,6) -> xor 1 -> index 20 green.
    assert_eq!(px(&data, 116, 100), (0, 252, 0, 255));

    // Pass 2: palette_dirty = false must reuse the uploaded palette and
    // still render correctly (the 004ee9b6 skip path).
    let mut frame2 = test_frame();
    frame2.palette_dirty = false;
    pipeline.upload_frame(&frame2);
    let cb = pipeline.draw(&view, TW, TH, &PresentConfig::default());
    gpu.queue().submit([cb]);
    gpu.device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(5)),
        })
        .unwrap();
    let data2 = readback(&gpu, &target);
    assert_eq!(px(&data2, 100, 100), (252, 0, 0, 255));
    assert_eq!(px(&data2, 116, 100), (0, 252, 0, 255));
}

#[test]
fn srgb_surface_preserves_palette_display_values() {
    let Some(gpu) = ParityGpu::new_headless() else {
        eprintln!("skip: no wgpu adapter on this host");
        return;
    };
    let mut palette = [[0; 3]; 256];
    for (i, entry) in palette.iter_mut().take(64).enumerate() {
        *entry = [i as u8, (63 - i) as u8, (i / 2) as u8];
    }
    let mut frame = Frame::new(palette);
    frame.palette_dirty = true;
    for y in 0..480 {
        for x in 0..640 {
            frame.set(x, y, (x / 10) as u8);
        }
    }
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        let mut pipeline = ParityPipeline::new(&gpu, format);
        pipeline.upload_frame(&frame);
        let target = make_target(&gpu, format);
        let view = target.create_view(&Default::default());
        for expand in [
            bedlam_render::VgaExpand::Original,
            bedlam_render::VgaExpand::Full,
        ] {
            let cfg = PresentConfig {
                expand,
                ..Default::default()
            };
            gpu.queue().submit([pipeline.draw(&view, TW, TH, &cfg)]);
            let data = readback(&gpu, &target);
            for i in 0..64u32 {
                let rgba = px(&data, i * 20 + 5, 100);
                let expected = palette[i as usize].map(|v| match expand {
                    bedlam_render::VgaExpand::Original => v << 2,
                    bedlam_render::VgaExpand::Full => (v << 2) | (v >> 4),
                });
                for (actual, expected) in [rgba.0, rgba.1, rgba.2].into_iter().zip(expected) {
                    assert!(
                        actual.abs_diff(expected) <= 1,
                        "{format:?} {expand:?} index {i}: got {actual}, expected {expected}"
                    );
                }
                assert_eq!(rgba.3, 255);
            }
        }
    }
}
