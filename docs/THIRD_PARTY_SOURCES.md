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

The Celestine aspect-engine package is intentionally separate and must keep its own exact source SHA/files/tests provenance when merged. See "Western aspect engine enrichment" below.

## Western aspect engine enrichment

**What was added:** the 3 Kepler aspects (septile, novile, decile) completing
XALEN's aspect catalogue to 14 types; a configurable per-aspect-type orb
model (`AspectConfig`); a normalized 0-100 aspect strength; out-of-sign
(dissociate) aspect detection; and named-body implementations of all 7
aspect patterns (T-Square, Grand Trine, Grand Cross, Yod, Kite, Mystic
Rectangle, Stellium).

**Source repository:** https://github.com/Anonyfox/celestine

**Source commit:** `954d63315ec00d29ba4becaef3f6a101497946b7`

**Source license:** MIT License (`Copyright (c) 2025 Anonyfox`; see
celestine's own `LICENSE` file at that commit for the full text).

**Source files used (behavioral reference for the Rust logic):**
- `src/aspects/types.ts` — `Aspect`, `AspectConfig`, `AspectBody`,
  `AspectPattern`/`PatternType` shape (mapped onto XALEN's existing
  `Aspect`/`AspectType`/`AspectDirection` naming; see "Naming decisions"
  below).
- `src/aspects/constants.ts` — the 14 aspect angles, default orbs, and the
  `ASPECT_SIGN_SEPARATIONS` table used for out-of-sign detection.
- `src/aspects/angular-separation.ts` — `normalizeAngle`, `angularSeparation`,
  `getSignIndex`, `signSeparation` (ported as `sign_index`/`sign_separation`
  in `xalen-western::aspects`; `angularSeparation` maps onto XALEN's
  pre-existing `angular_distance`, which was already equivalent and is
  unchanged).
- `src/aspects/orbs.ts` — `getOrb`, `calculateStrength` (the linear
  exact-to-orb-boundary decay model), `findMatchingAspect` (best-match-by-
  smallest-deviation rule).
- `src/aspects/aspect-detection.ts` — `isOutOfSign`, `detectAspect`,
  `calculateIsApplying` (the "compare today's deviation to tomorrow's,
  1-day linear extrapolation" applying/separating rule), `findAllAspects`.
- `src/aspects/patterns.ts` — all 7 pattern detectors
  (`detectTSquare`, `detectGrandTrine`, `detectGrandCross`, `detectYod`,
  `detectKite`, `detectMysticRectangle`, `detectStellium`) and the
  `findPatterns` dispatch order (rarity-first: Grand Cross, Kite, Mystic
  Rectangle, Grand Trine, T-Square, Yod, Stellium).

**Source test files used (behavioral reference for XALEN's own new tests):**
- `src/aspects/constants.test.ts`
- `src/aspects/angular-separation.test.ts`
- `src/aspects/orbs.test.ts`
- `src/aspects/aspect-detection.test.ts` — including its "Real J2000.0
  planetary aspects (JPL Horizons data)" section, whose fixture longitudes
  (Sun 280.3689092°, Moon 223.323786°, Mercury 271.8892699°, etc.) are
  reused directly in XALEN's own tests as a shared cross-reference point.
- `src/aspects/patterns.test.ts` — its `PERFECT_TSQUARE`/`PERFECT_GRAND_TRINE`/
  `PERFECT_GRAND_CROSS`/`PERFECT_YOD`/`PERFECT_KITE`/
  `PERFECT_MYSTIC_RECTANGLE`/`PERFECT_STELLIUM` mathematically-exact fixture
  longitudes, and its JPL J2000.0 body set.

**XALEN files with per-file provenance comments citing this entry:**
- `crates/xalen-western/src/aspects.rs`
- `crates/xalen-western/src/patterns.rs`
- `crates/xalen-python/src/aspect_engine.rs`

### Naming decisions (adapting to existing XALEN concepts, not duplicating)

Per the porting brief for this package, Celestine's names were mapped onto
XALEN's pre-existing equivalents wherever one already existed, rather than
introducing parallel/duplicate types:

| Celestine (TypeScript) | XALEN (Rust) | Relationship |
|---|---|---|
| `AspectType` | `xalen_western::aspects::AspectType` | XALEN's enum already existed (11 variants); this package added the 3 missing Kepler variants (Septile, Novile, Decile) to it directly — not a new enum. |
| `isApplying: boolean \| null` | `AspectPhase` (= `AspectDirection`, pre-existing) | XALEN already had `AspectDirection { Applying, Separating, Exact }`. `AspectPhase` is a `pub type` alias of it, not a second enum. |
| `Aspect` (rich, with strength/out-of-sign) | `AspectResult` | XALEN's pre-existing `Aspect` struct has a narrower shape (used by the pre-existing `find_aspect`/`find_all_aspects`, unchanged). `AspectResult` is an additive, richer sibling struct — not a replacement — returned by the new `detect_aspect`/`find_all_aspects_ex`. |
| `AspectConfig` | `AspectConfig` | New in XALEN (no prior equivalent); same name, same purpose (a growable config struct, not a flat parameter list, so future body-specific/luminary orb modifiers can be added without an API break). |
| `PatternType` | `AspectPatternType` (= `AspectPattern`, pre-existing) | XALEN already had a `patterns::AspectPattern` enum with all 7 variants (used by the pre-existing index-based `detect_patterns`, unchanged). `AspectPatternType` is a `pub type` alias of it. |
| `AspectPattern` (rich, named-body result) | `AspectPatternMatch` | XALEN's existing `AspectPattern` name was already taken by the enum (see above), so the new named-body rich result struct is `AspectPatternMatch` to avoid a naming collision while still being obviously paired with it. |

### Intentional differences from Celestine (not bugs, not "fixed to match")

- **Deterministic tie-break ordering.** Celestine's `findAllAspects` sorts by
  strength only, which leaves same-strength ties in whatever order
  `Array.prototype.sort` produces from insertion order. XALEN's
  `find_all_aspects_ex` sorts by strength descending, then by
  `(body1, body2)` ascending as an explicit tie-break, giving fully
  deterministic output regardless of input body ordering.
- **Default orbs for the 11 pre-existing `AspectType` variants are UNCHANGED
  from XALEN's own prior values**, not updated to match Celestine's. The two
  differ slightly (e.g. XALEN's pre-existing sextile default is 5°, Celestine
  documents 6°; XALEN's Quintile/BiQuintile default is 1.5°, Celestine's is
  2°). XALEN's existing values were kept for backward compatibility (existing
  callers of `find_all_aspects`/`find_aspect` must not see any behavior
  change from this package). Only the 3 newly-added Kepler variants (which
  had no prior XALEN value) use Celestine's documented default (1° for all
  three).
- **No behavioral bugs were found in Celestine's aspect/pattern logic during
  this port.** The above two entries are the only known divergences, and
  both are deliberate. Direct cross-validation (running the same fixture
  longitudes through Celestine's real TypeScript via `tsx` and through
  XALEN's Rust code) confirmed all 7 pattern-detection fixtures and 7
  pairwise-aspect-detection fixtures match on every field except the two
  documented differences above.

## Tarot canonical data

Tarot data lives in the separate `GoToValhalla/tarot` repository and maintains its own field-level provenance manifest under `data/tarot/provenance.json`.
