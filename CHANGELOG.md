# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Expose `BoundPort`, `EdgeEnd`, `Port` and `Site` to public API
- Expose `PortDiff::boundary_iter`
- Implement `Debug` for `Owned` data structures

## [0.1.0] - 2024-08-26

Initial release

### Added
- Initial project structure and core functionality
- PortDiff data structure for efficient graph rewriting
- Graph extraction and manipulation utilities
- Portgraph integration and visualization
- Serialization and deserialization support
- Web viewer for PortDiff graphs using Next.js and React

[Unreleased]: https://github.com/lmondada/portdiff/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lmondada/portdiff/releases/tag/v0.1.0