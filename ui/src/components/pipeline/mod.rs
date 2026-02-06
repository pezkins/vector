//! Pipeline Builder Components
//!
//! Components for building and editing Vector pipelines visually.
//! Features n8n-style drag-and-drop canvas with visual connections.

mod view;
mod canvas;
mod palette;
mod node;
mod config_panel;
pub mod data_view;

pub use view::PipelineView;
pub use canvas::PipelineCanvas;
pub use palette::ComponentPalette;
pub use node::PipelineNode;
pub use config_panel::ConfigPanel;
