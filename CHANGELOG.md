# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2024-09-09

### Added
- `PortDiff`s now have a `value` field to associate an integer value with the diff.
- Add `PortDiffGraph::map_value` to set the `value` field of diffs. This creates new diffs with the same graph structure.


## [0.3.0] - 2024-09-04

### Added
- Add `PortDiff::resolve_boundary_index` to handle wire boundaries.
- Add `PortDiffGraph::from_sinks_while` to create a `PortDiffGraph` from a predicate.
- Add `PortDiffGraph::is_squashable` and `PortDiffGraph::try_squash` to check and squash compatible `PortDiffGraph`s.
- Add `PortDiff::all_parents` to get all parent `PortDiff`s.
- Viewer: show CX count delta in hierarchy node labels.

### Changed
- Redesigned boundary port abstraction. Use `BoundarySite::Wire` to represent a boundary not connected to a site.
- Rename `GraphView` to `PortDiffGraph` to better reflect its purpose.

### Fixed
- Crash on squashing several wires with identical IDs.
- Wires at boundary are squashed properly.

## [0.2.0] - 2024-09-03

### Added
- Expose `InvalidRewriteError`, `PortDiff::as_ptr`
- Viewer support for experimental `StaticSizeCircuit`s diffs from `tket2`.
- New `BoundaryPort` type. Boundaries can now be either sites or sentinel nodes, thus supporting rewrites to empty graphs.

### Changed
- Graph::link_sites no longer returns the newly created edge.
- `PortDiff::boundary_site` now returns `Option<BoundaryPort>`. Use `PortDiff::boundary_port` to get a `BoundaryPort` instead.

### Fixed
- `PortDiff::opposite_ports` was returning duplicate ports.
- Rewrite failed for boundaries outside of region area.
- `PortDiff::are_compatible` checks compatibility of all diffs in the graph view.

## [0.1.1] - 2024-08-27

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

[Unreleased]: https://github.com/lmondada/portdiff/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/lmondada/portdiff/releases/tag/v0.3.0
[0.2.0]: https://github.com/lmondada/portdiff/releases/tag/v0.2.0
[0.1.1]: https://github.com/lmondada/portdiff/releases/tag/v0.1.1
[0.1.0]: https://github.com/lmondada/portdiff/releases/tag/v0.1.0