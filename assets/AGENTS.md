# AGENTS.md

## Purpose

`assets/` stores non-code assets such as fonts, icons, images, audio, 3D models, scene resources, and licensed sample learning materials.

## Scope

### Phase 1 deliverables

- Basic UI icons if required.
- Fonts if required for readable learning content.

### Future deliverables

- Scene images and sprites for Bevy scenes.
- Audio, sound effects, 3D models, and richer interactive-learning assets.
- Sample learning material fixtures with explicit licensing.

## Module Responsibilities

- Keep assets organized by type and usage.
- Record asset source, license, and intended use.
- Keep render assets replaceable and independent from business logic.
- Separate source assets from generated or optimized artifacts.

## Recommended Structure

```text
assets/
├── fonts/
├── icons/
├── images/
│   ├── ui/
│   └── scenes/
├── audio/
│   ├── music/
│   └── sfx/
└── models/
    ├── characters/
    └── objects/
```

## Naming Rules

- Use lowercase names with hyphens, for example `button-primary.svg`.
- Include dimensions when useful, for example `icon-home-24x24.png`.
- Include state names when useful, for example `button-hover.png`.

## Testing and Quality Gates

- New assets must have a known source and license.
- Large binary assets must have a documented purpose.
- Generated assets should be reproducible or excluded from source control.

## Logging and Observability

Asset processing tools should log source path, output path, optimizer version, size changes, and errors without exposing private user files.

## Security and Privacy Rules

- Do not commit assets with unclear licensing.
- Do not commit user-private imported documents into `assets/`.
- Do not store secrets, generated caches, or private paths in asset metadata.

## Do Not

- Do not bind assets directly to Agent business logic.
- Do not add large binaries without justification.
- Do not use filenames that contain sensitive information.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../apps/AGENTS.md`](../apps/AGENTS.md)
