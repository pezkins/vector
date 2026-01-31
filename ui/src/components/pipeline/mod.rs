//! Pipeline Builder Components
//!
//! Components for building and editing Vector pipelines visually.

mod view;
mod canvas;
mod palette;
mod node;

pub use view::PipelineView;
pub use canvas::PipelineCanvas;
pub use palette::ComponentPalette;
pub use node::PipelineNode;
