# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic
Versioning].

<!-- #release:next-header -->

## [Unreleased] <!-- #release:date -->

## [0.2.0] - 2023-03-07

* Uniformly derive `Serialize` and `Deserialize` on all API types, even if the
  type is not serialized or deserialized by `Client`. The idea is to allow
  downstream users to serialize and deserialize these types for their own
  purposes (e.g., to store them on disk).

## 0.1.0 - 2023-01-08

Initial release.

<!-- #release:next-url -->
[Unreleased]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.1.0...v0.2.0

[Keep a Changelog]: https://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
