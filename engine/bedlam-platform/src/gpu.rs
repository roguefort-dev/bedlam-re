//! The wgpu parity pipeline (D20 initial target: upload, palette
//! expand, fullscreen-triangle scaler).
//!
//! Resources per ParityPipeline: an R8Uint 640x480 index texture
//! (re-uploaded every frame), an R32Uint 256x1 palette texture
//! (re-uploaded only on frame.palette_dirty - the 004ee9b6 handshake
//! analog, DESIGN-RENDER sec 2 fact 7), a params uniform (filter +
//! expansion policy) and a uv-rect uniform (Fill-mode source crop).
//! The shader expands 6-bit entries to 8-bit per the policy, then
//! either point-samples or bilinear-mixes the EXPANDED RGB of four
//! neighbors (indices are never interpolated).

use bedlam_render::{Frame, VgaExpand, CANON_H, CANON_W};

use crate::scale::{scale_rect, uv_rect, FilterMode, PresentConfig};

const SHADER: &str = r#"
struct Params {
    filter_linear: u32,
    expand_full: u32,
    pad0: u32,
    pad1: u32,
};

struct UvRect {
    r: vec4<f32>,
};

@group(0) @binding(0) var indices_tex: texture_2d<u32>;
@group(0) @binding(1) var pal_tex: texture_2d<u32>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<uniform> uv_rect: UvRect;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VOut {
    var tri = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    var o: VOut;
    let xy = tri[vi];
    o.pos = vec4<f32>(xy, 0.0, 1.0);
    // NDC (-1, 1) is the top-left; uv (0, 0) is frame row 0.
    o.uv = vec2<f32>(xy.x * 0.5 + 0.5, 0.5 - xy.y * 0.5);
    return o;
}

fn expand_channel(v6: u32) -> f32 {
    let e = select(v6 << 2u, (v6 << 2u) | (v6 >> 4u), params.expand_full != 0u);
    return f32(e) / 255.0;
}

fn lookup(t: vec2<i32>) -> vec3<f32> {
    let idx = textureLoad(indices_tex, t, 0).x;
    let packed = textureLoad(pal_tex, vec2<i32>(i32(idx), 0), 0).x;
    return vec3<f32>(
        expand_channel(packed & 63u),
        expand_channel((packed >> 6u) & 63u),
        expand_channel((packed >> 12u) & 63u),
    );
}

fn sample_uv(uv: vec2<f32>) -> vec3<f32> {
    let dims = vec2<f32>(textureDimensions(indices_tex));
    if (params.filter_linear == 0u) {
        var t = vec2<i32>(floor(uv * dims));
        t = clamp(t, vec2<i32>(0, 0), vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1));
        return lookup(t);
    }
    let p = uv * dims - vec2<f32>(0.5, 0.5);
    let p0 = floor(p);
    let f = p - p0;
    let d = vec2<i32>(dims) - vec2<i32>(1, 1);
    let t00 = clamp(vec2<i32>(p0), vec2<i32>(0, 0), d);
    let t10 = clamp(vec2<i32>(p0) + vec2<i32>(1, 0), vec2<i32>(0, 0), d);
    let t01 = clamp(vec2<i32>(p0) + vec2<i32>(0, 1), vec2<i32>(0, 0), d);
    let t11 = clamp(vec2<i32>(p0) + vec2<i32>(1, 1), vec2<i32>(0, 0), d);
    let c00 = lookup(t00);
    let c10 = lookup(t10);
    let c01 = lookup(t01);
    let c11 = lookup(t11);
    return mix(mix(c00, c10, f.x), mix(c01, c11, f.x), f.y);
}

@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(
        mix(uv_rect.r.x, uv_rect.r.z, in.uv.x),
        mix(uv_rect.r.y, uv_rect.r.w, in.uv.y),
    );
    return vec4<f32>(sample_uv(uv), 1.0);
}
"#;

/// Uniform bytes for the params buffer: [filter_linear, expand_full,
/// 0, 0] little-endian u32 x 4 (16 B, no padding surprises).
fn params_bytes(cfg: &PresentConfig) -> [u8; 16] {
    let filter = u32::from(cfg.filter == FilterMode::Linear);
    let expand = u32::from(cfg.expand == VgaExpand::Full);
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&filter.to_le_bytes());
    out[4..8].copy_from_slice(&expand.to_le_bytes());
    out
}

/// A ready-to-use wgpu device + queue pair, headless (no surface).
/// None when no adapter is available (pure-CI containers): callers
/// skip GPU tests, they do not fail.
pub struct ParityGpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl ParityGpu {
    /// Request a low-power adapter with no compatible surface and open
    /// the device with default limits and NO optional features (the
    /// parity path needs none). Blocks internally via pollster.
    pub fn new_headless() -> Option<ParityGpu> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bedlam-platform parity device"),
            ..Default::default()
        }))
        .ok()?;
        Some(ParityGpu { device, queue })
    }

    /// The window-host half: request an adapter that can PRESENT to
    /// `surface`, then open the same low-power / default-limits /
    /// no-features device as the headless path so both hosts behave
    /// alike. The adapter is returned alongside (the caller needs it
    /// for surface capabilities). None when no present-capable
    /// adapter exists. Blocks internally via pollster - window-host
    /// only, never on the sim path.
    pub fn new_for_surface(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
    ) -> Option<(wgpu::Adapter, ParityGpu)> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bedlam-platform parity device"),
            ..Default::default()
        }))
        .ok()?;
        Some((adapter, ParityGpu { device, queue }))
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

/// Owns the GPU parity resources and encodes presents.
///
/// Backend-agnostic by construction (wgpu selects Vulkan/DX12/Metal);
/// the target format is given at construction so the same pipeline
/// serves surfaces and offscreen tests.
pub struct ParityPipeline {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    index_tex: wgpu::Texture,
    pal_tex: wgpu::Texture,
    params_buf: wgpu::Buffer,
    uv_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    palette_uploaded: bool,
}

impl ParityPipeline {
    /// Create the pipeline for a target with the given color format.
    pub fn new(gpu: &ParityGpu, target_format: wgpu::TextureFormat) -> ParityPipeline {
        let device = gpu.device().clone();
        let queue = gpu.queue().clone();

        let index_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bedlam parity indices"),
            size: wgpu::Extent3d {
                width: CANON_W,
                height: CANON_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let index_view = index_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let pal_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bedlam parity palette"),
            size: wgpu::Extent3d {
                width: 256,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let pal_view = pal_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bedlam parity params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uv_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bedlam parity uv rect"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bedlam parity bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bedlam parity bind group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&index_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&pal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uv_buf.as_entire_binding(),
                },
            ],
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bedlam parity shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bedlam parity pipeline layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bedlam parity pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(target_format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        ParityPipeline {
            device,
            queue,
            pipeline,
            index_tex,
            pal_tex,
            params_buf,
            uv_buf,
            bind_group,
            palette_uploaded: false,
        }
    }

    /// Upload a frame: indices always; the palette only when
    /// frame.palette_dirty or on the first upload (the 004ee9b6
    /// handshake analog - presentation re-uploads, render just flags).
    pub fn upload_frame(&mut self, frame: &Frame) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.index_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.indices[..],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(CANON_W),
                rows_per_image: Some(CANON_H),
            },
            wgpu::Extent3d {
                width: CANON_W,
                height: CANON_H,
                depth_or_array_layers: 1,
            },
        );
        if frame.palette_dirty || !self.palette_uploaded {
            let mut packed = [0u32; 256];
            for (i, c) in frame.palette.iter().enumerate() {
                packed[i] = u32::from(c[0] & 0x3f)
                    | (u32::from(c[1] & 0x3f) << 6)
                    | (u32::from(c[2] & 0x3f) << 12);
            }
            let mut bytes = Vec::with_capacity(1024);
            for v in packed {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.pal_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(1024),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 256,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            self.palette_uploaded = true;
        }
    }

    /// Encode one present into a returned command buffer: clear to
    /// black (the bars), set the scale viewport, draw the fullscreen
    /// triangle. A zero-size scale rect yields an empty buffer (no
    /// draw) rather than an invalid viewport. The caller submits.
    pub fn draw(
        &mut self,
        target: &wgpu::TextureView,
        target_w: u32,
        target_h: u32,
        cfg: &PresentConfig,
    ) -> wgpu::CommandBuffer {
        let rect = scale_rect(cfg.scale, CANON_W, CANON_H, target_w, target_h);
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bedlam parity present"),
            });
        if rect.w == 0 || rect.h == 0 {
            return enc.finish();
        }
        let uv = uv_rect(cfg.scale, CANON_W, CANON_H, target_w, target_h);
        self.queue
            .write_buffer(&self.params_buf, 0, &params_bytes(cfg));
        let mut uvb = [0u8; 16];
        uvb[0..4].copy_from_slice(&uv[0].to_le_bytes());
        uvb[4..8].copy_from_slice(&uv[1].to_le_bytes());
        uvb[8..12].copy_from_slice(&uv[2].to_le_bytes());
        uvb[12..16].copy_from_slice(&uv[3].to_le_bytes());
        self.queue.write_buffer(&self.uv_buf, 0, &uvb);
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bedlam parity pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.set_viewport(
                rect.x as f32,
                rect.y as f32,
                rect.w as f32,
                rect.h as f32,
                0.0,
                1.0,
            );
            rpass.draw(0..3, 0..1);
        }
        enc.finish()
    }
}
