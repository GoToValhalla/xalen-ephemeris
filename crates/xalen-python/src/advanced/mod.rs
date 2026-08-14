// Copyright 2024-2026 XALEN Technology Pvt Ltd
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[rustfmt::skip]
mod core;
mod product;
mod product_vedic_extended;
#[rustfmt::skip]
mod presentation;
#[rustfmt::skip]
mod vedic_extra;
#[path = "../western_enrichment.rs"]
mod western_enrichment;

use pyo3::prelude::*;

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    core::register(m)?;
    vedic_extra::register(m)?;
    presentation::register(m)?;
    western_enrichment::register(m)?;
    product::register(m)?;
    product_vedic_extended::register(m)?;
    Ok(())
}
