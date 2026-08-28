# RESEARCH — reproducible HD asset pipeline (2026-08-18; refreshed 2026-08-28)

Status: recommended implementation baseline for PLAN P6/D21 — the plan's own named
prerequisite doc ("exact package/model pins come from
docs/RESEARCH-HD-ASSET-PIPELINE.md"). This pipeline is optional, non-parity, and
authoring-only. It must not become a game runtime dependency.

This unit is RESEARCH ONLY: no generated assets, no engine change, no new binary
RE claims (the citations below carry their own external provenance), and the P6
behavior catalog stays EMPTY. Every external pin in §1, §2 and §5 was re-verified
first-hand against its primary source on 2026-08-28 (release pages, model cards,
repository READMEs, the Comfy-Org template listing API, and the official docs
site); claims not re-verified this pass say so explicitly.

## 1. Constraints and recommendation

Local hardware profile recorded on 2026-08-18 (unchanged; re-verify before any
acceptance run — see §9):

| Component | Value |
|---|---|
| GPU | NVIDIA RTX 5070 Ti Laptop GPU |
| VRAM | 12,227 MiB |
| Driver | 610.57.04 |
| Reported CUDA | 13.3 |
| System RAM | 30 GiB |
| Free storage | 3.5 TiB |
| System Python | 3.14.7 — do not install into it |

Use an isolated Python 3.12 environment managed by
[`uv`](https://docs.astral.sh/uv/guides/install-python/). Never invoke the
system Python's `pip`. CUDA reported by the driver is not the same thing as the
CUDA runtime bundled with a Torch wheel; record both.

Recommended baseline pins (verified 2026-08-28, first-hand):

- [ComfyUI v0.34.0](https://github.com/Comfy-Org/ComfyUI/releases/tag/v0.34.0)
  (released 2026-08-26, commit `12d5279438bfefc058a269eae805ceab6047777f`;
  repository LICENSE is GPL-3.0 — verified by fetching the LICENSE at the tag).
  The previous baseline
  [v0.33.1](https://github.com/Comfy-Org/ComfyUI/releases/tag/v0.33.1)
  (2026-08-13, commit `72865f4f27eaf5396f8f36370e0a2be3a9a090ee`) remains the
  pinned fallback. Note from the release history: since v0.32.0 the minimum
  officially supported PyTorch is 2.7, and v0.34.0 warns Python 3.10 is nearing
  EOL — keep the uv-managed 3.12 interpreter.
- Official [comfy-cli v1.18.0](https://github.com/Comfy-Org/comfy-cli/releases/tag/v1.18.0)
  (released 2026-08-24, commit `6a5e9d772453f27b0778b14e8a4b1e25ac5f949e`).
  v1.16.0 (2026-08-10) remains the pinned fallback. Caution first-hand from the
  v1.17.0 notes: API-workflow validation moved under `comfy workflow validate`;
  scripts must not depend on the old verb layout.
- Python 3.12 installed and isolated with `uv`.
- Core ComfyUI nodes only for the first acceptance pass (SeedVR2 is core-native
  since [PR #14424](https://github.com/Comfy-Org/ComfyUI/pull/14424) per the
  official [SeedVR2 tutorial](https://docs.comfy.org/tutorials/utility/seedvr2)
  — the one exception already in core).
- Shared authoring root `~/AI/ComfyUI/`, outside this repository and not
  Bedlam-specific.
- Reusable models/workflows under `~/AI/`; Bedlam-specific extracted inputs and
  generated outputs under an external `~/AI/projects/bedlam/` HD pack (or
  another user-selected pack root).
- Loopback-only server binding and `--disable-api-nodes`.

The upstream [ComfyUI README](https://github.com/Comfy-Org/ComfyUI/blob/master/README.md)
and [manual installation guide](https://docs.comfy.org/installation/manual_install)
are the installation authorities (cited 2026-08-18; not re-fetched this pass).
Do not replace the pins above with a moving branch. Record the resolved ComfyUI
commit, Python, Torch, CUDA runtime, and every package hash after installation.

This split keeps large, mutable tool state and prohibited asset pixels out of
the engine-only repository (`README.md`, PLAN P6, D21). Loopback binding reduces
API exposure, while disabling API nodes prevents accidental hosted-service use;
local core workflows are sufficient for the baseline.

## 2. Optional preferred bootstrap: Arctic Helper

[Arctic ComfyUI Helper](https://github.com/ArcticLatent/Arctic-Helper) is the
preferred optional bootstrap/manager path because it provides uv-managed
Python, GPU-aware Torch recommendations, ComfyUI/model placement, and verified
updates using signed manifests and SHA-256 checksums. It is Apache-2.0 licensed
(stated in the v0.2.7 release notes, verified first-hand 2026-08-28).

Pin [Arctic Helper v0.2.9](https://github.com/ArcticLatent/Arctic-Helper/releases/tag/v0.2.9)
(released 2026-08-08, commit `7d595dbef1e92627623dc5c155fe2c6ce3ff192d`;
re-verified as the latest release on 2026-08-28):

```text
URL: https://github.com/ArcticLatent/Arctic-Helper/releases/download/v0.2.9/arctic-comfyui-helper-0.2.9-1-x86_64.pkg.tar.zst
sha256: 91fbb27d4c35002c4c7a2d42de132e76bc25698ad3970a16b650761a4e61c1bd
```

(The package hash was recorded 2026-08-18; re-verify against the release
checksums before use.) v0.2.9 fixed the Arch package's binary-packaging defect:
the previous package contained a NixOS dynamic loader and Nix-store paths and
could not start on a normal Arch installation. The v0.2.9 release also added
checks for unexpected ELF interpreters and leaked build paths.

Use Arctic Helper to install or manage the pinned authoring instance at the
runtime root. It is not linked, bundled, or launched by the game. Select the
RTX 5070 Ti, review the proposed Torch stack, target `~/AI/ComfyUI`, and pin
ComfyUI v0.34.0. Keep every optional add-on/custom node disabled, including
SageAttention, FlashAttention, InsightFace, Nunchaku, Trellis, and ComfyUI
Manager. Independently capture versions, sources, revisions, licenses, and
SHA-256 hashes; a manager's catalog is not the provenance record.

The manual `uv` + official `comfy-cli` path remains supported and is the
fallback if Arctic Helper cannot reproduce these pins.

## 3. Repository and external-pack boundaries

Git contains only workflow JSON, recipes, masks, model/tool/version hashes,
seeds/prompts, manifests and provenance. Generated images live in a
user-selected external HD-pack directory, never in git: no original pixels,
extracted derivatives, intermediate images, generated images, or approved
replacements belong in git. (This is the D21 boundary the automated gate
enforces; PLAN §6 verbatim.)

Tracked sketch:

```text
bedlam-re/
  workflows/hd/
    menu-outpaint.{ui,api}.json
    sprite-restore.{ui,api}.json
    tile-restore.{ui,api}.json
    portrait-restore.{ui,api}.json
  presets/hd/
    menu-outpaint.toml, sprite-strict.toml
    sprite-restore.toml, tile-restore.toml, portrait-restore.toml
  masks/hd/                 # geometry only; no original pixels
  tools/hd/                 # CLI/orchestration code
  manifests/hd/             # logical IDs, hashes, licenses, reviews
  docs/RESEARCH-HD-ASSET-PIPELINE.md
```

These paths are proposed implementation locations, not a claim that they
already exist. Git may track only:

- Paired workflow UI/API JSON.
- CLI code and semantic TOML presets.
- Masks that cannot reconstruct original pixels.
- Logical manifests and approved-output hashes.
- Model/tool/version/license hashes.
- Provenance records and human reviews.

External sketch:

```text
~/AI/
  ComfyUI/                  # shared pinned application + isolated environment
  models/                   # reusable immutable model files addressed by hash
  workflows/                # personal non-project workflows
  output/                   # general ComfyUI output
  projects/bedlam/          # external Bedlam HD pack; never git-add
    inputs/                 # extracted originals
    work/                   # masks with pixels and intermediates
    generated/, approved/, rejected/
    provenance/, reviews/
    PACK-MANIFEST.toml      # hd-pack-manifest-v1 (§6)
    PACK-MANIFEST.sha256
```

The runtime consumes an external pack by stable logical asset ID and falls back
to the original asset when no approved replacement is present (the full seam
sketch is §8).

## 4. Workflow contract and headless execution

Every workflow has two committed representations:

- `*.ui.json`: editable graph with layout and authoring metadata.
- `*.api.json`: normalized graph submitted to the server.

Export both from the same accepted graph and hash both. The
[`comfy run` documentation](https://docs.comfy.org/comfy-cli/getting-started#comfy-run)
states that the CLI accepts UI- or API-format JSON and can convert UI format,
but committing the API form makes the exact submitted graph reviewable.

Do not make scripts patch anonymous node numbers directly. Each TOML preset
defines semantic slots such as:

```toml
[workflow]
id = "menu-outpaint-v1"

[slots]
source = "input.image"
mask = "input.mask"
seed = "sampler.seed"
output_prefix = "save.filename_prefix"
```

The wrapper resolves semantic slots against a checked workflow schema and
fails on missing or duplicate bindings. It must never silently use a node's UI
default.

Preferred local CLI sequence, following the official
[`comfy-cli` guide](https://docs.comfy.org/comfy-cli/getting-started)
(cited 2026-08-18; verb surface re-checked against the v1.17.0/v1.18.0 release
notes on 2026-08-28):

1. `comfy workflow validate <workflow.api.json>` (validation was re-homed here
   in v1.17.0; accept either verb layout in scripts, but never depend on the
   old one alone).
2. `comfy upload <source> <mask>`.
3. `comfy --json run --where local --workflow <workflow.api.json>`.
4. `comfy jobs status|wait <prompt_id>` or `comfy jobs watch <prompt_id>`.
5. `comfy download <prompt_id>` into a per-job temporary directory.

The wrapper validates required nodes and models before upload and rejects any
unexpected node class. It launches ComfyUI on loopback with
`--disable-api-nodes`; it never requests a wildcard/LAN listen address.

The direct API fallback uses the routes implemented by
[`server.py`](https://github.com/Comfy-Org/ComfyUI/blob/master/server.py):

- `POST /upload/image` for source and mask inputs.
- `POST /prompt` to queue the API graph.
- `/ws` for progress and completion events.
- `GET /history/{prompt_id}` to resolve outputs.
- `GET /view` to retrieve an output by server-returned metadata.

The official
[`websockets_api_example.py`](https://github.com/Comfy-Org/ComfyUI/blob/master/script_examples/websockets_api_example.py)
is the direct-client reference. Core image loading behavior is defined in
[`nodes.py`](https://github.com/Comfy-Org/ComfyUI/blob/master/nodes.py), and
upscale-model loading/execution in
[`nodes_upscale_model.py`](https://github.com/Comfy-Org/ComfyUI/blob/master/comfy_extras/nodes_upscale_model.py).

After download, decode with Pillow and verify mode, dimensions, and alpha. Use
the official [`Image`](https://pillow.readthedocs.io/en/stable/reference/Image.html)
and [`ImageChops`](https://pillow.readthedocs.io/en/stable/reference/ImageChops.html)
APIs for deterministic comparisons. Hash file and decoded pixels, run the
class-specific tests, write the external candidate, then write provenance to a
temporary file, `fsync`, and atomically rename it. Explicit human approval is
required before copying bytes to `approved/`.

A failed validation, interrupted job, malformed output, missing provenance
field, or failed invariant leaves no approved output.

## 5. The four workflow categories

The plan names exactly four workflow categories: (a) 4:3 → 16:9/16:10
background outpainting/generative fill, (b) alpha-aware sprite/sprite-sheet
upscale, (c) seamless tile/texture upscale, (d) portraits/UI art. Each
subsection below carries its candidate ComfyUI workflow preset shape and the
exact primary pins; the machine-checkable pin registry (fenced TOML at the end
of this section) is what the p6-hd-asset-research gate validates.

### 5.A (a) Background outpainting / generative fill (4:3 → 16:9/16:10)

Start with Comfy's official
[`FLUX.1 Fill outpaint workflow`](https://github.com/Comfy-Org/workflow_templates/blob/main/templates/flux_fill_outpaint_example.json)
(verified present in the templates listing via the GitHub contents API on
2026-08-28; a sibling `flux_fill_inpaint_example.json` exists) and the
[FLUX.1 Fill guide](https://docs.comfy.org/tutorials/flux/flux-1-fill-dev)
(cited 2026-08-18). Use it only if the gated model terms have been accepted and
a representative 12 GB VRAM smoke test passes.

The [FLUX.1 Fill model card](https://huggingface.co/black-forest-labs/FLUX.1-Fill-dev)
(re-fetched 2026-08-28) identifies a 12B BF16 model under
`flux-1-dev-non-commercial-license`, gated access, and — in the card's own
Limitations — possible slight color shifts in areas that are not filled in and
possible lines at the edges of the filled area. Read the actual
[`FLUX.1 dev` license](https://github.com/black-forest-labs/flux/blob/main/model_licenses/LICENSE-FLUX1-dev)
before downloading or publishing a pack.

Authoring contract:

- Canvas master is 16:10; the centered 16:9 rectangle is the safe region.
- Controls, text, symbols, and gameplay information are engine-rendered and
  remain inside the safe region.
- The diffusion mask covers only newly added canvas plus a narrow feather.
- Composite the original center pixels back over the generated result after
  inference. Do not trust an unmasked model region to remain byte-identical
  (the card's color-shift limitation applies to untouched areas).
- Use fixed seeds and recorded prompts, then require human review.
- Prompts must prohibit text, labels, controls, logos, icons, and invented
  gameplay symbols.

Fallback order if FLUX terms or memory profile are unacceptable:

1. [Stable Diffusion 2 Inpainting](https://huggingface.co/stabilityai/stable-diffusion-2-inpainting)
   — as of 2026-08-28 this model card is NOT retrievable unauthenticated (the
   fetch returns 401/login-gated). Its license and hash must be re-verified
   after access before it may serve as a fallback; it can never be a primary.
2. [SDXL base 1.0](https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0)
   with an independently pinned inpaint-compatible workflow/model — verified
   2026-08-28: license `openrail++` (CreativeML Open RAIL++-M), card states
   the model is intended for research purposes only; a base T2I model is not
   an inpainting solution by itself.

Do not imply that a fallback has equivalent quality. Evaluate identical
source/mask/prompt test cases and retain the reviewed bytes, not a model label,
as canon.

### 5.B (b) Alpha-aware sprite / sprite-sheet upscale

Two tiers, both whole-sheet:

Strict sprites are transformations, not generative restorations:

- Decode the complete sprite or sheet once.
- Scale RGB/index data with deterministic nearest-neighbor only.
- Preserve exact frame rectangles, anchors/hotspots, row/column ordering,
  padding, and sheet dimensions multiplied by the integer scale.
- Preserve the source palette/index semantics where the renderer expects an
  indexed result; reject newly introduced colors.
- Scale alpha separately with nearest-neighbor and require exact binary or
  source-level alpha values as appropriate.
- Recombine only after RGB/index and alpha tests pass.

This path has no diffusion, denoising, color correction, or per-frame crop.

Restored sprites are learned, alpha-aware, whole-sheet (the model never sees
transparent RGB garbage as opaque color). Compare official Real-ESRGAN
weights (URLs verified first-hand in the
[model zoo](https://github.com/xinntao/Real-ESRGAN/blob/master/docs/model_zoo.md)
on 2026-08-28):

- [RealESRGAN_x2plus](https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.1/RealESRGAN_x2plus.pth)
- [RealESRGAN_x4plus_anime_6B](https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.4/RealESRGAN_x4plus_anime_6B.pth)

The project is covered by its
[`BSD-3-Clause license`](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE),
but each downloaded weight still needs a recorded source revision and hash.
In ComfyUI both load through the core
[`UpscaleModelLoader`](https://github.com/Comfy-Org/ComfyUI/blob/master/comfy_extras/nodes_upscale_model.py)
(`models/upscale_models/`).

Rules:

- Process the whole sheet to avoid frame-by-frame style drift.
- Upscale RGB and alpha separately; never send transparent RGB garbage or
  alpha through the same learned pass.
- Restore exact output geometry after inference.
- Reject haloing, alpha growth, clipped frames, palette drift beyond the
  approved class policy, or changed contact points.
- Build nearest-neighbor and both learned candidates into one labeled contact
  sheet for human review.

### 5.C (c) Seamless tile / texture upscale

Diffusion is out of scope initially. Compare
[RealESRGAN_x2plus](https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.1/RealESRGAN_x2plus.pth)
with [SwinIR](https://github.com/JingyunLiang/SwinIR) on the same source set.
SwinIR is Apache-2.0 (verified from the repository README 2026-08-28) with
pretrained models on its [GitHub releases](https://github.com/JingyunLiang/SwinIR/releases);
the real-world SR checkpoint named by the README is
`003_realSR_BSRGAN_DFO_s64w8_SwinIR-M_x4_GAN.pth` (middle size; a SwinIR-L
variant exists). Resolve and record the exact release-asset URL and hash at
download time (the README pins names, not URLs).

For each candidate:

- Construct a 3x3 repeat before processing where the workflow supports it.
- Crop the center tile back to exact integer-scaled dimensions.
- Compare opposite edge rows and columns under the format's periodic contract.
- Render a 3x3 repetition and inspect both direct and diagonal junctions.
- Run half-width and half-height offset views so the seam crosses the center.
- Reject any candidate whose seam test is worse than deterministic nearest
  neighbor, regardless of single-tile sharpness.

### 5.D (d) Portraits / UI art

Portraits and UI art are a RESTRICTED sub-case of the sprite toolchain, not a
face-restoration pipeline:

- Deterministic baseline: the §5.B strict path (integer nearest-neighbor,
  exact palette/index discipline) is the default for portraits, panels,
  borders, and HUD chrome.
- Learned whole-image candidates: the same §5.B weights evaluated per-portrait
  (whole image, alpha-separated) — x2plus for 2x, anime_6B where its prior
  suits painted art.
- Face-prior restoration models are surveyed but NOT defaults:
  - [GFPGAN v1.4](https://github.com/TencentARC/GFPGAN/releases/download/v1.3.0/GFPGANv1.4.pth)
    (verified from the repository README 2026-08-28: code is Apache-2.0; the
    V1.3 entry documents a "slight change on identity" — identity drift is
    disqualifying for character portraits unless human review accepts it per
    asset). Its photographic StyleGAN2 prior repaints skin/shading detail onto
    indexed pixel art; ComfyUI carries no core GFPGAN node, so it is a
    phase-2 custom-node experiment at most.
  - [CodeFormer](https://github.com/sczhou/CodeFormer): NTU S-Lab License 1.0
    (non-commercial research), verified from the README 2026-08-28 — the
    license alone excludes it from any distributable HD pack; surveyed for
    completeness, deferred.
- UI art that carries meaning — glyphs, numbers, icons, cursors, button
  labels, click affordances — is ENGINE-RENDERED (§8); the pipeline never
  generates it, and the outpaint prompt rules (§5.A) prohibit text, labels,
  and symbols in generated backgrounds.

### 5.E Phase 2 restoration

After the core-only baseline is stable, evaluate
[SeedVR2](https://docs.comfy.org/tutorials/utility/seedvr2) (tutorial verified
2026-08-28: natively supported in ComfyUI core since PR #14424; both sizes
Apache-2.0) using the official
[Comfy-Org/SeedVR2](https://huggingface.co/Comfy-Org/SeedVR2) repackage —
exact variant filenames verified first-hand: `seedvr2_3b_fp8_e4m3fn.safetensors`,
`seedvr2_3b_int8_convrot.safetensors` (the 3B INT8/FP8 posture this doc
pinned originally), plus 7B / `7b_sharp` and MXFP4/NVFP4 variants and the
shared VAE `seedvr2_ema_vae_fp16.safetensors`. Original weights:
[ByteDance-Seed/SeedVR2-3B](https://huggingface.co/ByteDance-Seed/SeedVR2-3B)
/ [SeedVR2-7B](https://huggingface.co/ByteDance-Seed/SeedVR2-7B). Official
workflow template: `utility_seedvr2_3b_int8_upscale_image.json`. Re-profile
peak VRAM and quality on this 12 GB machine before making it a preset.

Defer SeedVR2 7B as a preset and [SUPIR](https://github.com/Fanghua-Yu/SUPIR)
entirely. SUPIR is a heavy, photorealism-oriented restoration path with
non-commercial restrictions; it is a poor initial fit for indexed game art and
this hardware envelope.

### Pin registry (machine-checkable, hd-asset-pins-v1)

The fenced block below is the machine-readable pin registry the
p6-hd-asset-research gate validates (tools/check-p6-hd-asset-research.py).
`kind = "model"` pins must carry a license; `role = "primary"` pins must carry
a verified license (never an "unverified" marker); deferred pins must carry a
note. The registry records retrieval provenance; runtime hashes are recorded
at download into the external pack, not here.

```toml
schema = "hd-asset-pins-v1"

[[pin]]
id = "comfyui"
kind = "tool"
role = "primary"
version = "v0.34.0"
revision = "12d5279438bfefc058a269eae805ceab6047777f"
url = "https://github.com/Comfy-Org/ComfyUI/releases/tag/v0.34.0"
license = "GPL-3.0"
retrieved = "2026-08-28"
note = "latest stable 2026-08-26; v0.32.0+ requires PyTorch >= 2.7"

[[pin]]
id = "comfyui-fallback"
kind = "tool"
role = "fallback"
version = "v0.33.1"
revision = "72865f4f27eaf5396f8f36370e0a2be3a9a090ee"
url = "https://github.com/Comfy-Org/ComfyUI/releases/tag/v0.33.1"
license = "GPL-3.0"
retrieved = "2026-08-28"

[[pin]]
id = "comfy-cli"
kind = "tool"
role = "primary"
version = "v1.18.0"
revision = "6a5e9d772453f27b0778b14e8a4b1e25ac5f949e"
url = "https://github.com/Comfy-Org/comfy-cli/releases/tag/v1.18.0"
retrieved = "2026-08-28"
note = "v1.17.0 re-homed API-workflow validation under comfy workflow validate"

[[pin]]
id = "comfy-cli-fallback"
kind = "tool"
role = "fallback"
version = "v1.16.0"
url = "https://github.com/Comfy-Org/comfy-cli/releases/tag/v1.16.0"
retrieved = "2026-08-28"

[[pin]]
id = "arctic-helper"
kind = "tool"
role = "bootstrap"
version = "v0.2.9"
revision = "7d595dbef1e92627623dc5c155fe2c6ce3ff192d"
url = "https://github.com/ArcticLatent/Arctic-Helper/releases/tag/v0.2.9"
license = "Apache-2.0"
sha256 = "91fbb27d4c35002c4c7a2d42de132e76bc25698ad3970a16b650761a4e61c1bd"
retrieved = "2026-08-28"
note = "package hash recorded 2026-08-18; re-verify against release checksums before use"

[[pin]]
id = "flux-fill-outpaint-template"
kind = "workflow-template"
role = "primary"
version = "main"
url = "https://github.com/Comfy-Org/workflow_templates/blob/main/templates/flux_fill_outpaint_example.json"
retrieved = "2026-08-28"
note = "template listing verified via the GitHub contents API; export UI+API JSON at accept time and pin both hashes"

[[pin]]
id = "seedvr2-3b-int8-upscale-template"
kind = "workflow-template"
role = "deferred"
version = "main"
url = "https://github.com/Comfy-Org/workflow_templates/blob/main/templates/utility_seedvr2_3b_int8_upscale_image.json"
retrieved = "2026-08-28"
note = "phase-2 reference per the official SeedVR2 tutorial"

[[pin]]
id = "flux-1-fill-dev"
kind = "model"
role = "primary"
categories = ["background-outpaint"]
version = "12B BF16"
url = "https://huggingface.co/black-forest-labs/FLUX.1-Fill-dev"
license = "FLUX.1 dev Non-Commercial License (gated access)"
retrieved = "2026-08-28"
note = "model card documents slight color shifts in untouched areas and lines at fill edges"

[[pin]]
id = "sd2-inpainting"
kind = "model"
role = "fallback"
categories = ["background-outpaint"]
version = "512-inpainting checkpoint"
url = "https://huggingface.co/stabilityai/stable-diffusion-2-inpainting"
license = "unverified (model card login-gated as of 2026-08-28); re-verify after access"
retrieved = "2026-08-28"
note = "fetch returns 401 unauthenticated; license + hash must be re-verified after access before ANY use"

[[pin]]
id = "sdxl-base-1.0"
kind = "model"
role = "fallback"
categories = ["background-outpaint"]
version = "base 1.0 (3B)"
url = "https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0"
license = "CreativeML Open RAIL++-M (openrail++); card states research purposes only"
retrieved = "2026-08-28"
note = "a base T2I model is not an inpainting solution by itself; needs an independently pinned inpaint workflow"

[[pin]]
id = "real-esrgan-x2plus"
kind = "model"
role = "primary"
categories = ["sprite-upscale", "tile-texture-upscale", "portrait-ui"]
version = "v0.2.1 release weights"
url = "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.1/RealESRGAN_x2plus.pth"
license = "BSD-3-Clause repository; per-weight terms recorded at download"
retrieved = "2026-08-28"

[[pin]]
id = "real-esrgan-x4plus-anime-6b"
kind = "model"
role = "primary"
categories = ["sprite-upscale", "portrait-ui"]
version = "v0.2.2.4 release weights"
url = "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.2.4/RealESRGAN_x4plus_anime_6B.pth"
license = "BSD-3-Clause repository; per-weight terms recorded at download"
retrieved = "2026-08-28"

[[pin]]
id = "swinir-real-sr-m-x4"
kind = "model"
role = "primary"
categories = ["tile-texture-upscale"]
version = "003_realSR_BSRGAN_DFO_s64w8_SwinIR-M_x4_GAN.pth"
url = "https://github.com/JingyunLiang/SwinIR/releases"
license = "Apache-2.0"
retrieved = "2026-08-28"
note = "checkpoint filename pinned by the README; resolve the exact release-asset URL + hash at download"

[[pin]]
id = "gfpgan-v1.4"
kind = "model"
role = "deferred"
categories = ["portrait-ui"]
version = "v1.4 weights (hosted under the v1.3.0 release)"
url = "https://github.com/TencentARC/GFPGAN/releases/download/v1.3.0/GFPGANv1.4.pth"
license = "Apache-2.0 code; verify weight terms before any distribution"
retrieved = "2026-08-28"
note = "documented identity drift + photographic prior unsuitable for indexed pixel art as default; no core ComfyUI node; phase-2 custom-node experiment only"

[[pin]]
id = "codeformer"
kind = "model"
role = "deferred"
categories = ["portrait-ui"]
version = "release tag v0.1.0 weights"
url = "https://github.com/sczhou/CodeFormer"
license = "NTU S-Lab License 1.0 (non-commercial research)"
retrieved = "2026-08-28"
note = "non-commercial license excludes it from any distributable HD pack; surveyed for completeness"

[[pin]]
id = "seedvr2-3b"
kind = "model"
role = "deferred"
categories = ["sprite-upscale", "tile-texture-upscale", "portrait-ui"]
version = "seedvr2_3b_fp8_e4m3fn / seedvr2_3b_int8_convrot (+ shared VAE seedvr2_ema_vae_fp16)"
url = "https://huggingface.co/Comfy-Org/SeedVR2"
license = "Apache-2.0"
retrieved = "2026-08-28"
note = "core-native since ComfyUI PR #14424; phase-2 after reproducibility + VRAM profiling; 7B/sharp variants exist"

[[pin]]
id = "supir"
kind = "model"
role = "deferred"
categories = ["sprite-upscale", "tile-texture-upscale", "portrait-ui"]
version = "n/a"
url = "https://github.com/Fanghua-Yu/SUPIR"
license = "repository non-commercial restrictions"
retrieved = "2026-08-18"
note = "heavy photorealism path, poor fit for indexed art; deferred (license note carried from the 2026-08-18 pass, not re-verified)"
```

## 6. Provenance + manifest schema

The external pack is addressed by a machine-checkable manifest,
`hd-pack-manifest-v1` (TOML, `PACK-MANIFEST.toml` + its `.sha256` at the pack
root). Every candidate and approval record contains:

| Group | Required fields |
|---|---|
| Source | logical asset ID; source file hash; decoded-pixel hash; dimensions; mode; palette hash/size when present |
| Recipe | workflow ID; UI/API workflow hashes; preset hash; fully resolved parameter hash; mask hash |
| Generation | seed; positive/negative prompts; target dimensions; 16:9 safe-region rectangle; feather geometry |
| Runtime | ComfyUI/CLI/Python/Torch versions; accelerator backend and runtime; GPU; VRAM; driver |
| Model | name; first-party source URL; immutable revision; file SHA-256; declared license and license-file hash |
| Output | candidate and approved byte hashes; decoded-pixel hash; dimensions; mode; external logical path |
| Review | invariant-test results; reviewer; UTC timestamp; disposition; notes/rejection reason |

Manifest shape (logical mirror only — the in-git `manifests/hd/` copy carries
IDs and hashes, never pixels):

```toml
schema = "hd-pack-manifest-v1"

[[asset]]
id = "zoneb.menu.background"        # stable logical asset ID (§8)
class = "background-outpaint"       # one of the four §5 categories
source_sha256 = "..."               # original file hash (value only, no bytes)
candidate_sha256 = "..."
approved_sha256 = "..."             # required for shipping
workflow_id = "menu-outpaint-v1"
workflow_ui_sha256 = "..."
workflow_api_sha256 = "..."
preset_sha256 = "..."
resolved_params_sha256 = "..."
mask_sha256 = "..."
seed = 0
prompt_positive = "..."
prompt_negative = "..."
target_width = 2048
target_height = 1280
safe_region = [224, 0, 1824, 1080]  # 16:9 inside the 16:10 master
runtime_comfyui = "v0.34.0"
runtime_torch = "..."
gpu = "..."
[model_records]                     # transitive: checkpoint, VAE, encoders, upscalers, LoRAs
[[review]]
reviewer = "..."
reviewed_utc = "..."
disposition = "approved"            # or rejected
invariants = "..."                  # pointer to the recorded gate results
notes = "..."
```

Include all transitive model components: checkpoint/diffusion model, VAE, text
encoders, upscaler, and any LoRA. A friendly model name is not provenance.

A fixed seed does **not** promise byte-identical output across Torch versions,
GPU architectures, drivers, CUDA runtimes, attention implementations, or
ComfyUI changes. It makes reruns inspectable. Once approved, the exact external
bytes and their hashes are canonical; reproducing visually similar bytes does
not silently replace them.

## 7. Automated gate criteria design

The PLAN §6 sentence names five gate families; each becomes a fail-closed
predicate over the pack (checked by the future hd gate tooling, designed here,
not implemented in this unit):

1. Provenance: every shipped output resolves to a complete `hd-pack-manifest-v1`
   record (all §6 groups present; every model record carries URL + revision +
   sha256 + license); the in-git logical manifest joins the external
   `PACK-MANIFEST.toml` on `approved_sha256`; and outputs without recorded
   provenance are excluded from shipping (the `approved/` directory is the
   ONLY shipping source and an unmanifested file there is an error, not a
   warning).
2. Dimensions: exact class geometry — sprites/sheets at exact integer-scaled
   frame rectangles and sheet dimensions; tiles at exact integer-scaled
   dimensions after the 3x3 crop-back; outpaints at the declared 16:10 master
   with the declared 16:9 safe region; any dimension mismatch rejects.
3. Alpha integrity: strict sprites keep exact binary/source-level alpha with
   zero halo growth (alpha erosion/dilation forbidden outside the declared
   feather); restored sprites' alpha is upscaled separately and compared
   channel-exactly against the source-derived expectation; palette-indexed
   outputs introduce no new palette entries.
4. Seam quality: tiles satisfy opposite-edge difference thresholds under the
   format's periodic contract, and the 3x3 repetition + half-offset views show
   no visible direct or diagonal junction worse than deterministic
   nearest-neighbor; outpaint composites keep the original center rectangle
   byte-for-byte with generated pixels confined to new canvas + feather.
5. Perceptual thresholds: learned candidates are compared against the
   deterministic nearest-neighbor baseline with recorded metrics (e.g. SSIM/
   PSNR floors and palette-drift bounds per class policy); a candidate that
   fails its class threshold cannot be approved regardless of reviewer
   enthusiasm; the labeled contact sheet (original / nearest / each candidate
   / alpha / high-contrast composite) is part of the evidence.

The gate is fail-closed and evidence-first: no recorded evidence, no shipping.
Human review approves exact bytes; the automated checks bound what review may
accept.

## 8. Runtime resolution seam sketch

The engine-side seam (future work, designed here — no engine change lands in
this unit):

- Replacements are resolved by STABLE LOGICAL ASSET ID (e.g.
  `zoneb.menu.background`), never by file name or path. The ID namespace is
  part of the asset contract and lands with the first real pack.
- At load time the engine consults the user-selected external HD pack
  (`PACK-MANIFEST.toml`): a present, hash-matching, approved entry replaces
  the original asset; on any miss — no pack, missing ID, hash mismatch,
  incomplete manifest record — the engine falls back to the original asset
  from the read-only game-data corpus. Fallback is silent, complete, and
  never an error: a missing or partial pack yields the fully original game.
- HD-pack selection is a platform presentation option OUT of ModeConfig (the
  D200 layering posture: it selects nothing in the sim, and both pacing arms
  accept it identically); it is authoring-pack metadata, not game state, and
  never enters a hash or the save format.
- The engine renders all text, controls, click targets and gameplay
  information; generated backgrounds never carry them. Click targets and
  layout live in the engine's responsive layout (the ENHANCED-mode 16:10
  master / 16:9 safe region), so a hallucinated button in a generated
  background is unreachable dead art, and prompts forbid it anyway (§5.A).
- Parity is untouched: the canonical 640x480 indexed frame + palette ride
  unchanged; the HD pack is an ENHANCED-mode surface only.

## 9. Isolated, hardware-profiled setup posture

- Isolated interpreter: uv-managed Python 3.12 dedicated to the authoring
  instance; never the system Python, never a shared site-packages. All
  package installs go through the isolated environment's lockfile.
- Pinned stack: ComfyUI v0.34.0 (fallback v0.33.1), comfy-cli v1.18.0
  (fallback v1.16.0), optional Arctic Helper v0.2.9 bootstrap — every pin
  verified against its primary source (§1, §2, §5); the resolved commit,
  Python, Torch, CUDA runtime, and package hashes are recorded after install.
- Network posture: loopback-only server binding, `--disable-api-nodes`, custom
  nodes and add-ons disabled; no hosted/partner (credit-spending) nodes ever
  run on this project's workflows.
- Hardware profile: the §1 table is recorded per acceptance run and re-verified
  whenever GPU/driver/VRAM changes; VRAM-class-sensitive steps (FLUX Fill,
  SeedVR2) require a measured smoke test on the actual profile before becoming
  presets. A profile that cannot run a category keeps that category's
  deterministic path only.
- Acceptance smoke uses non-corpus test pixels first; any original-art input
  stays inside the external pack and never enters git.

## 10. Licensing matrix and cautions

| Component | Upstream terms to record | Pipeline decision |
|---|---|---|
| ComfyUI | GPL-3.0 (LICENSE at the v0.34.0 tag, verified 2026-08-28) | Authoring tool only; pin commit and environment |
| comfy-cli | See pinned release/repository license | Authoring/orchestration tool only |
| Arctic Helper | Apache-2.0 (release notes, verified 2026-08-28) | Optional installer/manager; not runtime |
| FLUX.1 Fill dev weights | FLUX.1 dev non-commercial license; gated access (verified 2026-08-28) | Use only after terms review/acceptance; hash every component |
| SD2 inpainting | Model card login-gated as of 2026-08-28 — re-verify after access | Fallback; can never be primary until terms are recorded |
| SDXL base | openrail++ (verified 2026-08-28); card states research purposes only | Fallback component, not automatically an inpaint solution |
| Real-ESRGAN code/models | BSD-3-Clause repository; record each weight source | Initial learned sprite/tile/portrait comparison |
| SwinIR | Apache-2.0 repository (verified 2026-08-28); weight-specific source | Initial tile comparison; record the chosen checkpoint |
| SeedVR2 (Comfy-Org repackage + originals) | Apache-2.0 (verified 2026-08-28) | Phase 2, 3B INT8/FP8 only initially |
| GFPGAN v1.4 | Apache-2.0 code; documented identity drift | Deferred; phase-2 experiment, never a default |
| CodeFormer | NTU S-Lab License 1.0, non-commercial (verified 2026-08-28) | Deferred; excluded from distributable packs |
| SUPIR | Repository non-commercial restrictions (carried from 2026-08-18) | Deferred |
| Original Bedlam art | User-supplied copyrighted game data | Never redistribute originals or derivatives from this repo |

Tool-code licensing does not grant rights to model weights, input art, or
outputs. Model-card statements do not override the linked license text. Before
publishing any external HD pack, perform a separate review of original-game
rights, every model/weight license, attribution/notice duties, acceptable-use
terms, and whether generated derivatives may be distributed.

The official ComfyUI
[image upscale guide](https://docs.comfy.org/tutorials/utility/image-upscale)
documents the core upscale workflow, but it does not settle a downloaded
model's license.

## 11. Phased setup checklist
Phase 0 — policy:
- [ ] Confirm external pack root/budget, git exclusions, asset IDs, reviewers,
      and acceptance or rejection of each gated/non-commercial model.
Phase 1 — core baseline:
- [ ] Install isolated Python 3.12, ComfyUI v0.34.0, and comfy-cli v1.18.0;
      optionally use Arctic Helper v0.2.9 after verifying the pinned hash.
- [ ] Record Python/Torch/backend/GPU/driver versions and hashes; disable custom
      nodes, add-ons, and API nodes; verify loopback-only reachability.
- [ ] Smoke-test load, upscale, upload, queue, websocket/history, download/view,
      and atomic provenance using non-corpus test pixels.
Phase 2 — deterministic classes:
- [ ] Implement strict nearest-neighbor sheets plus geometry, alpha, palette,
      contact-sheet, 3x3, edge, and offset tests before learned comparisons.
Phase 3 — learned upscale:
- [ ] Pin/hash RealESRGAN x2plus, x4plus anime6B, and one SwinIR checkpoint.
- [ ] Compare whole-sheet RGB/alpha-separated sprites and periodic textures;
      approve exact bytes only after automated gates and human review.
Phase 4 — outpaint:
- [ ] Review FLUX terms and run a measured 12 GB smoke test.
- [ ] Save paired official-workflow UI/API JSON, bind TOML slots, enforce
      new-canvas masking and center recomposition, then review both target crops.
- [ ] If FLUX is rejected/unstable, benchmark pinned SD2 (after access +
      license re-verification), then SDXL alternatives.
Phase 5 — optional restoration:
- [ ] Evaluate SeedVR2 3B INT8/FP8 only after reproducibility passes; keep 7B
      presets and SUPIR deferred absent measured need and license/hardware
      approval; keep GFPGAN a per-asset experiment and CodeFormer out.
- [ ] Give every new custom node its own revision, hash, dependency lock,
      license record, and clean-instance acceptance test.
