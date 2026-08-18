//! bedlam-platform: wgpu presentation skeleton (P3, D20).
//!
//! PARITY path (default, implemented): upload the canonical 640x480
//! indexed frame + 6-bit palette to the GPU, palette-expand + scale in
//! a single fullscreen-triangle pass (D20 initial target: wgpu
//! upload / palette-expand / fullscreen scaler). Letterbox, pillarbox,
//! fit, fill and smooth filtering are presentation concerns resolved
//! here (DESIGN-RENDER sec 8); the canonical frame itself never
//! changes with resolution or mode.
//!
//! ENHANCED path (D20/D21): RESERVED, not implemented in this
//! skeleton. Native-output-resolution world/UI passes land
//! incrementally in later units, always non-parity and UI-flagged,
//! sharing sim + assets with parity mode. Enhanced layouts are
//! authored 16:10 with a 16:9 safe region; widescreen viewport stays a
//! separate explicit option.
//!
//! Boundary rules (D12/D17, enforced by construction): this crate
//! consumes bedlam-render Frames only, never bedlam-core; resolution,
//! GPU timing, backend selection and interpolation never feed the
//! simulation or any hashed state. Timing is owned by the future
//! window host (fixed 60 Hz sim accumulator + present-paced frames);
//! nothing here reads a clock.

#![forbid(unsafe_code)]

pub mod gpu;
pub mod scale;

pub use gpu::{ParityGpu, ParityPipeline};
pub use scale::{scale_rect, uv_rect, FilterMode, PresentConfig, Rect, ScaleMode};

/// Re-export so hosts and tests need no direct wgpu dependency pin.
pub use wgpu;
