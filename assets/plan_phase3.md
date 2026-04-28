# Assets Module — Implementation Plan

## Module Overview

`assets/` stores non-code assets: fonts, icons, images, audio, 3D models, scene resources, and licensed sample learning materials. Assets are organized by type and usage, with clear provenance records.

**Core principle:** Every asset must have a known source, a known license, and a documented purpose. Assets are replaceable and independent from business logic.

## Phase Scope

| Phase | Deliverables | Status |
|-------|-------------|--------|
| Phase 1 | Basic UI icons, fonts for readable learning content | Planned |
| Phase 2 | No specific additions | — |
| Phase 2.5 | No specific additions | — |
| Phase 3 | Scene images, sprites, audio, 3D models, licensed learning materials | Planned |

## Phase 1 Detailed Plan

### File Structure

```
assets/
├── AGENTS.md
├── plan_phase3.md
├── fonts/
│   ├── README.md              # Font inventory with sources and licenses
│   ├── inter-v4/
│   │   ├── Inter-Regular.woff2
│   │   ├── Inter-Bold.woff2
│   │   └── LICENSE.txt        # SIL Open Font License 1.1
│   └── jetbrains-mono-v2/
│       ├── JetBrainsMono-Regular.woff2
│       ├── JetBrainsMono-Bold.woff2
│       └── LICENSE.txt
├── icons/
│   ├── README.md              # Icon inventory
│   ├── ui/
│   │   ├── chat-24x24.svg
│   │   ├── curriculum-24x24.svg
│   │   ├── chapter-24x24.svg
│   │   ├── complete-24x24.svg
│   │   ├── error-24x24.svg
│   │   ├── loading-24x24.svg
│   │   ├── send-24x24.svg
│   │   └── user-24x24.svg
│   └── LICENSE.txt
├── images/
│   └── README.md
├── audio/
│   └── README.md
├── models/
│   └── README.md
└── sample-materials/
    ├── README.md              # Licensed sample learning materials
    └── LICENSE.txt
```

### Phase 1 Assets

#### Fonts

| Font | Files | License | Purpose |
|------|-------|---------|---------|
| **Inter** | Regular (400), Bold (700) | SIL OFL 1.1 | Primary UI font; excellent readability at small sizes |
| **JetBrains Mono** | Regular (400), Bold (700) | SIL OFL 1.1 | Code blocks in learning content; ligatures for code readability |

Both fonts are:
- Available as `.woff2` for web performance.
- Self-hosted (no Google Fonts dependency — privacy requirement).
- Subset to Latin character set to minimize file size.

**Font loading strategy:**
```css
/* In apps/web-ui */
@font-face {
  font-family: 'Inter';
  src: url('/assets/fonts/inter-v4/Inter-Regular.woff2') format('woff2');
  font-weight: 400;
  font-display: swap;
}
```

#### Icons

Phase 1 uses SVG icons (not an icon font) for:
- Chat interface: send, attach, microphone (future).
- Navigation: curriculum, next chapter, previous chapter.
- Status: complete, in-progress, error, loading.
- User: profile, settings.

**Icon design constraints:**
- 24x24 base size with 2px stroke width.
- Monochrome with `currentColor` for theme compatibility.
- Accessible: include `<title>` elements for screen readers.
- License: MIT or CC0 (must be committable without attribution burden).

**Recommended source:** Create simple geometric icons in-house, or use Lucide icons (MIT licensed, clean style, 24x24 with `currentColor` support).

#### Asset Inventory (`fonts/README.md` format)

```markdown
# Font Inventory

| File | Font | Weight | Format | Size (KB) | Source | License |
|------|------|--------|--------|-----------|--------|---------|
| Inter-Regular.woff2 | Inter | 400 | woff2 | 42 | https://rsms.me/inter/ | SIL OFL 1.1 |
| Inter-Bold.woff2 | Inter | 700 | woff2 | 43 | https://rsms.me/inter/ | SIL OFL 1.1 |
| JetBrainsMono-Regular.woff2 | JetBrains Mono | 400 | woff2 | 48 | https://www.jetbrains.com/lp/mono/ | SIL OFL 1.1 |
| JetBrainsMono-Bold.woff2 | JetBrains Mono | 700 | woff2 | 50 | https://www.jetbrains.com/lp/mono/ | SIL OFL 1.1 |
```

### Phase 3 Assets

Phase 3 adds interactive scene assets for the Bevy renderer. These are planned here but not created in Phase 1.

#### Scene Images and Sprites

```
assets/
├── images/
│   ├── ui/                    # UI-specific images (backgrounds, illustrations)
│   └── scenes/
│       ├── molecules/         # Chemistry visualization sprites
│       ├── diagrams/          # Physics diagram components
│       ├── maps/              # Geography/history map tiles
│       └── characters/        # Learning companion character sprites
```

#### Audio

```
assets/
├── audio/
│   ├── music/                 # Background music (ambient learning focus tracks)
│   └── sfx/                   # Sound effects (correct answer, achievement, notification)
```

Audio must be:
- Compressed (`.ogg` or `.mp3` for web; `.wav` only for source files).
- Licensed for commercial use or in-house created.
- Optional — the app must function fully without audio.

#### 3D Models

```
assets/
├── models/
│   ├── characters/            # Animated learning companion
│   ├── objects/               # 3D manipulatives (molecules, geometric shapes, etc.)
│   └── environments/          # Scene environments (lab, classroom, outdoor)
```

3D models should use `.gltf` or `.glb` format for Bevy compatibility.

#### Sample Learning Materials

Licensed sample materials for testing and demonstration:

```
assets/
└── sample-materials/
    ├── math/
    │   └── sample-problems.md         # CC-BY licensed math problems
    ├── programming/
    │   └── sample-code-snippets.py    # MIT licensed code examples
    └── science/
        └── sample-diagram-data.json   # CC0 licensed diagram specifications
```

### Naming Rules

| Rule | Example |
|------|---------|
| Lowercase with hyphens | `button-primary.svg` |
| Include dimensions when useful | `icon-home-24x24.png` |
| Include state when useful | `button-hover.png` |
| Include variant when useful | `icon-send-disabled.svg` |
| Include resolution for raster | `bg-hero-1920x1080.webp` |

### Asset Processing Pipeline (Phase 3)

The `tools/asset-optimizer` tool processes source assets into optimized formats:

```
source/character.blend (source, not committed)
  → tools/asset-optimizer
  → assets/models/characters/character.glb (optimized, committed)
```

**Optimizer rules:**
- Logs source path, output path, optimizer version, size before/after, and errors.
- Never overwrites source assets — outputs to a separate directory.
- Reproducible: same input + same optimizer version = same output.
- Optimized assets are committed; source assets may be stored externally if large.

#### Font Subsetting

Fonts are subset to include only characters actually used in the application, reducing file size by 70-90%:

```bash
# tools/scripts/subset-font.sh
#!/bin/bash
# Subset fonts to Blup's required character set
# Usage: ./subset-font.sh <input-font> <output-dir>

INPUT="$1"
OUTPUT="$2"
BASENAME=$(basename "$INPUT" | sed 's/\.[^.]*$//')

# Characters needed:
# - Latin basic + extended (English, European languages)
# - Mathematical operators and symbols (for KaTeX math rendering)
# - Code punctuation
# - Common UI glyphs (arrows, bullets, etc.)
CHARSET="$(cat <<'EOF'
ABCDEFGHIJKLMNOPQRSTUVWXYZ
abcdefghijklmnopqrstuvwxyz
0123456789
αβγδεζηθικλμνξπρστυφχψω
ΓΔΘΛΞΠΣΦΨΩ
∂∇∫∏∑
←↑→↓↔⇒⇐⇑⇓
±×÷√∞≈≠≤≥
⏎␣▸▪●○◉
EOF
)"

# Remove whitespace, keep unique
CHARSET=$(echo "$CHARSET" | tr -d '[:space:]' | fold -w1 | sort -u | tr -d '\n')

# Subset using fonttools (Python)
pip install fonttools brotli 2>/dev/null

pyftsubset "$INPUT" \
  --text="$CHARSET" \
  --output-file="$OUTPUT/$BASENAME.woff2" \
  --flavor=woff2 \
  --layout-features='*' \
  --no-subset-tables+=COLR,CPAL \
  --desubroutinize \
  --no-hinting

echo "Subset: $INPUT → $OUTPUT/$BASENAME.woff2"
echo "Before: $(du -h "$INPUT" | cut -f1)"
echo "After:  $(du -h "$OUTPUT/$BASENAME.woff2" | cut -f1)"
```

#### Image Optimization Pipeline

```python
# tools/scripts/optimize-images.py
"""Optimize images for web: PNG → WebP + AVIF, with provenance tracking."""
import subprocess
import json
import hashlib
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional

@dataclass
class ImageOptimization:
    source: str
    source_size_bytes: int
    source_checksum: str
    outputs: list[dict]  # List of {path, format, size_bytes, compression_ratio, quality}

def optimize_image(source: Path, output_dir: Path, quality: int = 85) -> ImageOptimization:
    """Optimize a single image to WebP and AVIF."""
    source_size = source.stat().st_size
    source_hash = hashlib.sha256(source.read_bytes()).hexdigest()

    basename = source.stem
    outputs = []

    # ── WebP (lossy) ──
    webp_path = output_dir / f"{basename}.webp"
    subprocess.run([
        "cwebp", "-q", str(quality),
        "-o", str(webp_path), str(source)
    ], check=True, capture_output=True)
    webp_size = webp_path.stat().st_size
    outputs.append({
        "path": str(webp_path.relative_to(output_dir.parent)),
        "format": "webp",
        "size_bytes": webp_size,
        "compression_ratio": round(source_size / webp_size, 2),
        "quality": quality,
    })

    # ── AVIF (lossy, better compression) ──
    avif_path = output_dir / f"{basename}.avif"
    subprocess.run([
        "avifenc", "--min", str(quality - 10), "--max", str(quality),
        "--speed", "6",
        str(source), str(avif_path)
    ], check=True, capture_output=True)
    avif_size = avif_path.stat().st_size
    outputs.append({
        "path": str(avif_path.relative_to(output_dir.parent)),
        "format": "avif",
        "size_bytes": avif_size,
        "compression_ratio": round(source_size / avif_size, 2),
        "quality": quality,
    })

    # ── Compressed PNG (lossless, for icons) ──
    if source.suffix.lower() in ('.png', '.svg'):
        png_path = output_dir / f"{basename}.png"
        subprocess.run([
            "oxipng", "--opt", "max", "--strip", "all",
            "--out", str(png_path), str(source)
        ], check=True, capture_output=True)
        png_size = png_path.stat().st_size
        outputs.append({
            "path": str(png_path.relative_to(output_dir.parent)),
            "format": "png",
            "size_bytes": png_size,
            "compression_ratio": round(source_size / max(png_size, 1), 2),
            "quality": 100,  # Lossless
        })

    return ImageOptimization(
        source=str(source),
        source_size_bytes=source_size,
        source_checksum=f"sha256:{source_hash}",
        outputs=outputs,
    )

def optimize_directory(source_dir: Path, output_dir: Path, quality: int = 85) -> dict:
    """Optimize all images in a directory tree."""
    results = []
    source_dir = Path(source_dir)
    output_dir = Path(output_dir)

    for ext in ('*.png', '*.jpg', '*.jpeg', '*.webp'):
        for source in source_dir.rglob(ext):
            # Skip already-optimized files
            if '/optimized/' in str(source):
                continue

            rel_path = source.relative_to(source_dir)
            out_subdir = output_dir / rel_path.parent
            out_subdir.mkdir(parents=True, exist_ok=True)

            result = optimize_image(source, out_subdir, quality)
            results.append(asdict(result))

            print(f"  {source.name}: {result.source_size_bytes:,}B → "
                  f"{result.outputs[0]['size_bytes']:,}B "
                  f"(×{result.outputs[0]['compression_ratio']})")

    # Write provenance manifest
    manifest = {
        "optimizer_version": "0.1.0",
        "optimized_at": datetime.utcnow().isoformat(),
        "quality": quality,
        "total_source_size_bytes": sum(r["source_size_bytes"] for r in results),
        "total_optimized_size_bytes": sum(
            r["outputs"][0]["size_bytes"] for r in results
        ),
        "records": results,
    }

    manifest_path = output_dir / "provenance.json"
    manifest_path.write_text(json.dumps(manifest, indent=2))
    print(f"\nProvenance: {manifest_path}")
    print(f"Total: {manifest['total_source_size_bytes']:,}B → "
          f"{manifest['total_optimized_size_bytes']:,}B "
          f"(×{round(manifest['total_source_size_bytes'] / max(manifest['total_optimized_size_bytes'], 1), 1)})")

    return manifest
```

### License Audit Tooling

A script validates license compliance before commit:

```bash
# tools/scripts/audit-assets.sh
#!/bin/bash
# Audit all assets for license compliance
# Exit 1 if any asset lacks a documented license

ERRORS=0
ASSETS_DIR="${1:-assets}"

echo "=== Asset License Audit ==="

# Check fonts
for dir in "$ASSETS_DIR"/fonts/*/; do
    [ -d "$dir" ] || continue
    font_name=$(basename "$dir")
    if [ ! -f "$dir/LICENSE.txt" ] && [ ! -f "$dir/LICENSE" ] && [ ! -f "$dir/OFL.txt" ]; then
        echo "FAIL: $font_name — No license file found"
        ERRORS=$((ERRORS + 1))
    else
        echo "  OK: $font_name"
    fi
done

# Check icons
for svg in "$ASSETS_DIR"/icons/ui/*.svg; do
    [ -f "$svg" ] || continue
    basename=$(basename "$svg")
    # Check for <title> accessibility
    if ! grep -q '<title>' "$svg"; then
        echo "WARN: $basename — Missing <title> element (accessibility)"
    fi
done

if [ -f "$ASSETS_DIR/icons/LICENSE.txt" ]; then
    echo "  OK: icons/ (license present)"
else
    echo "FAIL: icons/ — No LICENSE.txt"
    ERRORS=$((ERRORS + 1))
fi

# Check images
find "$ASSETS_DIR/images" -type f \( -name "*.png" -o -name "*.svg" -o -name "*.webp" \) 2>/dev/null | while read img; do
    # Check if listed in README inventory
    if ! grep -q "$(basename "$img")" "$ASSETS_DIR/images/README.md" 2>/dev/null; then
        echo "WARN: $(basename "$img") — Not listed in images/README.md inventory"
    fi
done

echo ""
if [ "$ERRORS" -gt 0 ]; then
    echo "$ERRORS license violation(s) found. Fix before committing."
    exit 1
else
    echo "All assets license-compliant."
fi
```

### Asset Inventory Format

Each asset directory has a `README.md` inventory:

```markdown
# Images Inventory

| File | Source | License | Dimensions | Size | Purpose |
|------|--------|---------|------------|------|---------|
| hero-bg.webp | In-house | MIT | 1920×1080 | 45KB | Landing page background |
| molecule-h2o.glb | In-house (Blender) | CC-BY 4.0 | — | 120KB | Water molecule 3D model |
| icon-chat.svg | Lucide Icons | MIT | 24×24 | 1.2KB | Chat navigation icon |
| diagram-parabola.svg | In-house | MIT | 800×600 | 8KB | Parabola math diagram |

Last updated: 2025-06-01
Audited by: CI (audit-assets.sh)
```

### Testing and Quality Gates

- [ ] Every font file has a corresponding license file or entry in `fonts/README.md`.
- [ ] Every icon has an accessible `<title>` element.
- [ ] Fonts are subset to required character ranges (no unused glyphs).
- [ ] SVG icons validate as well-formed XML.
- [ ] Asset inventory is complete and accurate.
- [ ] No large binary assets without documented purpose.
- [ ] Generated/optimized assets are reproducible or excluded from source control.

### Security and Privacy Rules

- Never commit assets with unclear licensing.
- Never commit user-private imported documents into `assets/`.
- Never store secrets, generated caches, or private paths in asset metadata.
- Asset file names must not contain sensitive information (no `user-123-data.png`).
- Phase 3 3D models and scenes must not contain embedded scripts or executable content.

### Do Not

- Do not bind assets directly to Agent business logic (assets are presentation-layer only).
- Do not add large binaries (over 1MB) without justification in the inventory.
- Do not use filenames that contain sensitive information.
- Do not include assets that are only needed for one specific demo or experiment.
- Do not depend on external CDN-hosted assets (self-host everything for privacy).
