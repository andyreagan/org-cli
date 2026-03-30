//! Public HTML API — re-exports from the implementation modules.
//!
//! External code (tests, commands) imports from here; the actual logic lives in:
//!   - `crate::render`  — inline + body rendering, page layout, CSS
//!   - `crate::site`    — ID index, site export, file I/O

pub use crate::render::{render_html, render_html_opts, RenderOptions};
pub use crate::site::{
    build_id_index, export_file_with_map, export_site, generate_site_index,
};
