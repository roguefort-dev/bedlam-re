# RESEARCH — reproducible HD asset pipeline (2026-08-18)

Status: recommended implementation baseline for PLAN P6/D21. This pipeline is
optional, non-parity, and authoring-only. It must not become a game runtime
dependency.

## 1. Constraints and recommendation

Local hardware profile recorded on 2026-08-18:

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

Pin the first usable baseline to:

- [ComfyUI v0.33.1](https://github.com/Comfy-Org/ComfyUI/releases/tag/v0.33.1).
- Official [comfy-cli v1.16.0](https://github.com/Comfy-Org/comfy-cli/releases/tag/v1.16.0).
- Python 3.12 installed and isolated with `uv`.
- Core ComfyUI nodes only for the first acceptance pass.
- Shared authoring root `~/AI/ComfyUI/`, outside this repository and not Bedlam-specific.
- Reusable models/workflows under `~/AI/`; Bedlam-specific extracted inputs and generated outputs under an external `~/AI/projects/bedlam/` HD pack (or another user-selected pack root).
- Loopback-only server binding and `--disable-api-nodes`.

The upstream [ComfyUI README](https://github.com/Comfy-Org/ComfyUI/blob/master/README.md)
and [manual installation guide](https://docs.comfy.org/installation/manual_install)
are the installation authorities. Do not replace the pins above with a moving
branch. Record the resolved ComfyUI commit, Python, Torch, CUDA runtime, and
every package hash after installation.

This split keeps large, mutable tool state and prohibited asset pixels out of
the engine-only repository (`README.md`, PLAN P6, D21). Loopback binding reduces
API exposure, while disabling API nodes prevents accidental hosted-service use;
local core workflows are sufficient for the baseline.

## 2. Optional preferred bootstrap: Arctic Helper

[Arctic ComfyUI Helper](https://github.com/ArcticLatent/Arctic-Helper) is the
preferred optional bootstrap/manager path because it provides uv-managed
Python, GPU-aware Torch recommendations, ComfyUI/model placement, and verified
updates using signed manifests and SHA-256 checksums. It is Apache-2.0 licensed.

Pin [Arctic Helper v0.2.9](https://github.com/ArcticLatent/Arctic-Helper/releases/tag/v0.2.9):

```text
URL: https://github.com/ArcticLatent/Arctic-Helper/releases/download/v0.2.9/arctic-comfyui-helper-0.2.9-1-x86_64.pkg.tar.zst
sha256: 91fbb27d4c35002c4c7a2d42de132e76bc25698ad3970a16b650761a4e61c1bd
```

v0.2.9 fixed the Arch package's binary-packaging defect: the previous package
contained a NixOS dynamic loader and Nix-store paths and could not start on a
normal Arch installation. The v0.2.9 release also added checks for unexpected
ELF interpreters and leaked build paths.

Use Arctic Helper to install or manage the pinned authoring instance at the
runtime root. It is not linked, bundled, or launched by the game. Select the
RTX 5070 Ti, review the proposed Torch stack, target `~/AI/ComfyUI`, and pin
ComfyUI v0.33.1. Keep every optional add-on/custom node disabled, including
SageAttention, FlashAttention, InsightFace, Nunchaku, Trellis, and ComfyUI
Manager. Independently capture versions, sources, revisions, licenses, and
SHA-256 hashes; a manager's catalog is not the provenance record.

The manual `uv` + official `comfy-cli` path remains supported and is the
fallback if Arctic Helper cannot reproduce these pins.

## 3. Repository and external-pack boundaries

Tracked sketch:

```text
bedlam-re/
  workflows/hd/
    menu-outpaint.{ui,api}.json
    sprite-restore.{ui,api}.json
    tile-restore.{ui,api}.json
  presets/hd/
    menu-outpaint.toml, sprite-strict.toml
    sprite-restore.toml, tile-restore.toml
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
    PACK-MANIFEST.sha256
```

No original pixels, extracted derivatives, intermediate images, generated
images, or approved replacements belong in git. The runtime consumes an
external pack by stable logical asset ID and falls back to the original asset
when no approved replacement is present.

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
[`comfy-cli` guide](https://docs.comfy.org/comfy-cli/getting-started):

1. `comfy validate --workflow <workflow.api.json>`.
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

## 5. Asset-class workflows

### 5.1 Menu/background outpaint

Start with Comfy's official
[`FLUX.1 Fill outpaint workflow`](https://github.com/Comfy-Org/workflow_templates/blob/main/templates/flux_fill_outpaint_example.json)
and [guide](https://docs.comfy.org/tutorials/flux/flux-1-fill-dev). Use it only
if the gated model terms have been accepted and a representative 12 GB VRAM
smoke test passes.

The [FLUX.1 Fill model card](https://huggingface.co/black-forest-labs/FLUX.1-Fill-dev)
identifies a 12B BF16 model, gated access, a non-commercial weight license,
possible color shifts in untouched areas, and possible seams around filled
regions. Read the actual
[`FLUX.1 dev` license](https://github.com/black-forest-labs/flux/blob/main/model_licenses/LICENSE-FLUX1-dev)
before downloading or publishing a pack.

Authoring contract:

- Canvas master is 16:10; the centered 16:9 rectangle is the safe region.
- Controls, text, symbols, and gameplay information are engine-rendered and
  remain inside the safe region.
- The diffusion mask covers only newly added canvas plus a narrow feather.
- Composite the original center pixels back over the generated result after
  inference. Do not trust an unmasked model region to remain byte-identical.
- Use fixed seeds and recorded prompts, then require human review.
- Prompts must prohibit text, labels, controls, logos, icons, and invented
  gameplay symbols.

Fallback order if FLUX terms or memory profile are unacceptable:

1. [Stable Diffusion 2 Inpainting](https://huggingface.co/stabilityai/stable-diffusion-2-inpainting).
2. [SDXL base 1.0](https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0)
   with an independently pinned inpaint-compatible workflow/model.

Do not imply that the fallback has equivalent quality. Evaluate identical
source/mask/prompt test cases and retain the reviewed bytes, not a model label,
as canon.

### 5.2 Strict sprites

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

### 5.3 Restored sprites and sprite sheets

Compare official Real-ESRGAN
[`RealESRGAN_x2plus` and `RealESRGAN_x4plus_anime_6B`](https://github.com/xinntao/Real-ESRGAN/blob/master/docs/model_zoo.md)
on representative sheets. The project is covered by its
[`BSD-3-Clause license`](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE),
but each downloaded weight still needs a recorded source revision and hash.

Rules:

- Process the whole sheet to avoid frame-by-frame style drift.
- Upscale RGB and alpha separately; never send transparent RGB garbage or
  alpha through the same learned pass.
- Restore exact output geometry after inference.
- Reject haloing, alpha growth, clipped frames, palette drift beyond the
  approved class policy, or changed contact points.
- Build nearest-neighbor and both learned candidates into one labeled contact
  sheet for human review.

### 5.4 Periodic tiles and textures

Diffusion is out of scope initially. Compare
[`RealESRGAN_x2plus`](https://github.com/xinntao/Real-ESRGAN/blob/master/docs/model_zoo.md)
with [SwinIR](https://github.com/JingyunLiang/SwinIR) on the same source set.

For each candidate:

- Construct a 3x3 repeat before processing where the workflow supports it.
- Crop the center tile back to exact integer-scaled dimensions.
- Compare opposite edge rows and columns under the format's periodic contract.
- Render a 3x3 repetition and inspect both direct and diagonal junctions.
- Run half-width and half-height offset views so the seam crosses the center.
- Reject any candidate whose seam test is worse than deterministic nearest
  neighbor, regardless of single-tile sharpness.

### 5.5 Phase 2 restoration

After the core-only baseline is stable, evaluate
[`SeedVR2`](https://docs.comfy.org/tutorials/utility/seedvr2) using the official
[`Comfy-Org/SeedVR2`](https://huggingface.co/Comfy-Org/SeedVR2) 3B INT8/FP8
variants. Re-profile peak VRAM and quality on this 12 GB machine before making
it a preset.

Defer SeedVR2 7B and [SUPIR](https://github.com/Fanghua-Yu/SUPIR). SUPIR is a
heavy, photorealism-oriented restoration path with non-commercial restrictions;
it is a poor initial fit for indexed game art and this hardware envelope.

## 6. Provenance schema

Every candidate and approval record contains:

| Group | Required fields |
|---|---|
| Source | logical asset ID; source file hash; decoded-pixel hash; dimensions; mode; palette hash/size when present |
| Recipe | workflow ID; UI/API workflow hashes; preset hash; fully resolved parameter hash; mask hash |
| Generation | seed; positive/negative prompts; target dimensions; 16:9 safe-region rectangle; feather geometry |
| Runtime | ComfyUI/CLI/Python/Torch versions; accelerator backend and runtime; GPU; VRAM; driver |
| Model | name; first-party source URL; immutable revision; file SHA-256; declared license and license-file hash |
| Output | candidate and approved byte hashes; decoded-pixel hash; dimensions; mode; external logical path |
| Review | invariant-test results; reviewer; UTC timestamp; disposition; notes/rejection reason |

Include all transitive model components: checkpoint/diffusion model, VAE, text
encoders, upscaler, and any LoRA. A friendly model name is not provenance.

A fixed seed does **not** promise byte-identical output across Torch versions,
GPU architectures, drivers, CUDA runtimes, attention implementations, or
ComfyUI changes. It makes reruns inspectable. Once approved, the exact external
bytes and their hashes are canonical; reproducing visually similar bytes does
not silently replace them.

## 7. Acceptance tests

Sprite gates:

- Output and every frame rectangle have exact expected integer-scaled geometry.
- Anchors/contact points scale exactly; no frame content crosses its cell.
- Alpha values obey the selected strict/restored policy; transparent borders
  and RGB-under-alpha are checked explicitly.
- Strict mode introduces no palette entries and no unexpected indices.
- Labeled contact sheets show original, nearest, each model candidate, alpha,
  and a high-contrast background composite.

Tile gates:

- Opposite edges satisfy exact or policy-defined difference thresholds.
- 3x3 repetition has no visible horizontal, vertical, or diagonal junction.
- Half-tile X/Y/XY offsets expose no center seam.
- Geometry and alpha remain exact.

Outpaint gates:

- Original center rectangle is byte-for-byte unchanged after compositing.
- The 16:9 safe region contains all required engine-rendered UI bounds.
- Generated pixels are confined to the new canvas/feather allowance.
- No invented text, controls, logos, mission markers, arrows, or symbols.
- 16:9 and 16:10 crops receive separate human approval.

All classes also require successful decoding, expected dimensions/mode, model
and workflow hash matches, complete provenance, and an explicit review state.

## 8. Licensing matrix and cautions

| Component | Upstream terms to record | Pipeline decision |
|---|---|---|
| ComfyUI | See pinned release/repository license | Authoring tool only; pin commit and environment |
| comfy-cli | See pinned release/repository license | Authoring/orchestration tool only |
| Arctic Helper | Apache-2.0 | Optional installer/manager; not runtime |
| FLUX.1 Fill dev weights | FLUX.1 dev non-commercial license; gated access | Use only after terms review/acceptance; hash every component |
| SD2 inpainting | Model-card license/usage restrictions | Fallback; preserve card and license revision |
| SDXL base | Model-card OpenRAIL terms/restrictions | Fallback component, not automatically an inpaint solution |
| Real-ESRGAN code/models | BSD-3-Clause repository; record each weight source | Initial learned sprite/tile comparison |
| SwinIR | Repository license and weight-specific source | Initial tile comparison; verify the selected checkpoint |
| SeedVR2 | Model-card and per-file terms | Phase 2, 3B INT8/FP8 only initially |
| SUPIR | Repository non-commercial restrictions | Deferred |
| Original Bedlam art | User-supplied copyrighted game data | Never redistribute originals or derivatives from this repo |

Tool-code licensing does not grant rights to model weights, input art, or
outputs. Model-card statements do not override the linked license text. Before
publishing any external HD pack, perform a separate review of original-game
rights, every model/weight license, attribution/notice duties, acceptable-use
terms, and whether generated derivatives may be distributed.

The official ComfyUI
[`image upscale guide`](https://docs.comfy.org/tutorials/utility/image-upscale)
documents the core upscale workflow, but it does not settle a downloaded
model's license.

## 9. Phased setup checklist
Phase 0 — policy:
- [ ] Confirm external pack root/budget, git exclusions, asset IDs, reviewers,
      and acceptance or rejection of each gated/non-commercial model.
Phase 1 — core baseline:
- [ ] Install isolated Python 3.12, ComfyUI v0.33.1, and comfy-cli v1.16.0;
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
- [ ] If FLUX is rejected/unstable, benchmark pinned SD2, then SDXL alternatives.
Phase 5 — optional restoration:
- [ ] Evaluate SeedVR2 3B INT8/FP8 only after reproducibility passes; keep 7B
      and SUPIR deferred absent measured need and license/hardware approval.
- [ ] Give every new custom node its own revision, hash, dependency lock,
      license record, and clean-instance acceptance test.
