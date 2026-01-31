---
name: build
description: Build the olx-tracker project. Use when compiling the project or when the user asks to build.
argument-hint: [--release for optimized build]
allowed-tools: Bash(cargo *)
---

# Build Project

Compile the olx-tracker project.

## Commands

```bash
# Debug build (faster compile, larger binary, includes debug symbols)
cargo build

# Release build (optimized, smaller binary with LTO enabled)
cargo build --release
```

## Default Behavior

When $ARGUMENTS contains "--release" or "release", build in release mode. Otherwise, build in debug mode.

## Output Locations

- Debug: `target/debug/olx-tracker`
- Release: `target/release/olx-tracker`

## Release Optimizations

The release profile includes:
- Link-time optimization (LTO)
- Single codegen unit for maximum optimization
- Symbol stripping for smaller binaries
