//! Full-text search: normalization, stemming, block text, and the index written with
//! the site. `static/packages/search` reads the index in the browser.

pub mod index;
pub mod normalize;
pub mod stem;
pub mod text;
