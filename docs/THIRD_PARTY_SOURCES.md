# Third-party sources and provenance

This file records source provenance for capabilities added or adapted for the GoToValhalla Tarot project. It supplements the repository's Apache-2.0 `LICENSE`/`NOTICE` files; it does not replace upstream license obligations.

## Base engine

- Repository: https://github.com/vedika-io/xalen-ephemeris
- Fork: https://github.com/GoToValhalla/xalen-ephemeris
- Validated base SHA: `cc6edbec1f748ebdc4950ae6198f575c5ada73fa`
- License: Apache-2.0

## Python advanced bindings

The Tarot project exposes existing XALEN Rust capabilities through thin PyO3 wrappers. The mathematical implementations remain in their original XALEN crates; they are not reimplemented in Python.

### Vedic enrichment

Target wrapper: `crates/xalen-python/src/advanced/vedic_extra.rs`

Existing XALEN Rust sources used directly:

- `crates/xalen-vedic/src/ashtakavarga.rs` — Bhinna/Sarva Ashtakavarga
- `crates/xalen-vedic/src/ashtottari.rs` — Ashtottari Dasha
- `crates/xalen-vedic/src/yogini.rs` — Yogini Dasha
- `crates/xalen-vedic/src/compatibility.rs` — Ashtakoota / Guna Milan
- `crates/xalen-vedic/src/shadbala.rs` — full six-fold Shadbala
- `crates/xalen-vedic/src/dosha.rs` — Mangal, Kaal Sarpa/Amrit, Pitra Dosha
- `crates/xalen-vedic/src/yoga.rs` — Pancha Mahapurusha, Gajakesari, Budhaditya, Vipreeta Raja, Kemadruma

External validation/reference projects for later cross-validation (no code from these projects is copied by this package):

- PyJHora: https://github.com/naturalstupid/PyJHora
- VedAstro: https://github.com/VedAstro/VedAstro

For any future direct adaptation from those repositories, record the exact source SHA, source file/function, license, target file, and modifications before merging.

### Fixed stars

Target wrapper: `crates/xalen-python/src/advanced/presentation.rs`

Existing XALEN source:

- `crates/xalen-stars/src/lib.rs`

Important data-license boundary:

- The curated astrologically significant catalog is part of the normal XALEN source surface.
- The expanded Hipparcos-derived catalog is behind the `hip-catalog` feature and is explicitly treated by XALEN as non-commercial data.
- Commercial-clean builds must use `--no-default-features` unless the expanded catalog's data rights are separately cleared.
- The Python API exposes `fixed_star_catalog_info()` so callers can inspect which mode was built.

Kerykeion is retained as a behavioral/rendering/fixed-star reference only for this package: https://github.com/g-battaglia/kerykeion

No Kerykeion code is copied by this package.

### Chart rendering

Target wrapper: `crates/xalen-python/src/advanced/presentation.rs`

Existing XALEN rendering source used directly:

- `crates/xalen-chart/src/lib.rs`
- `render_western_wheel`
- `render_north_indian`
- `render_south_indian`

No external renderer code is copied in this package. Kerykeion/Stellium remain visual/behavioral references for later comparison.

### Advanced Western enrichment

Target wrapper: `crates/xalen-python/src/western_enrichment.rs`.

The following capabilities already existed in the Apache-2.0 XALEN Rust core and are exposed directly rather than rewritten from donor repositories:

- `crates/xalen-western/src/hellenistic.rs` — annual profections and zodiacal releasing;
- `crates/xalen-western/src/progressions.rs` — solar-arc directions and profection support;
- `crates/xalen-western/src/declination.rs` — declination, parallel and contraparallel aspects;
- `crates/xalen-western/src/dignity.rs` — domicile, exaltation, detriment, fall, triplicity, terms, faces and aggregate essential-dignity score;
- `crates/xalen-western/src/lots.rs` — Arabic Lots / Parts;
- `crates/xalen-western/src/electional.rs` — Moon quality and void-of-course state;
- `crates/xalen-ephem/src/event_search.rs` — sign ingress and station root searches;
- `crates/xalen-ephem/src/eclipse.rs` — solar and lunar eclipse searches/classification.

New GoToValhalla implementation:

- `crates/xalen-western/src/relationship.rs` — shortest-arc midpoint composite primitives and Davison time/geographic midpoint, with Rust unit tests. This implementation was written for the fork and does not copy an external donor implementation.

Reference-only projects retained for later cross-validation, not copied in this package:

- Stellium: https://github.com/katelouie/stellium
- Kerykeion: https://github.com/g-battaglia/kerykeion
- LibEphemeris: https://github.com/g-battaglia/libephemeris

The Celestine aspect-engine package is intentionally separate and must keep its own exact source SHA/files/tests provenance when merged.

## Tarot canonical data

Tarot data lives in the separate `GoToValhalla/tarot` repository and maintains its own field-level provenance manifest under `data/tarot/provenance.json`.
