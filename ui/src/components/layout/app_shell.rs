//! App Shell Component
//!
//! Main layout container that combines:
//! - Top navigation tabs
//! - Optional sidebar (for pipeline pages)
//! - Main content area
//! - Optional resizable bottom panel (for pipeline/observability pages)
//! - Status bar

use leptos::*;

use super::{MainTabs, PipelineSidebar, BottomPanel, DataPreviewPanel, StatusBar};
use crate::state::{AppState, Theme, BottomPanelTab};

/// Main application shell layout
#[component]
pub fn AppShell(
    /// Main content (routes)
    children: Children,
    /// Whether to show the sidebar (for pipeline pages)
    #[prop(default = false)] show_sidebar: bool,
    /// Whether to show the bottom panel (for pipeline/observability pages)
    #[prop(default = false)] show_bottom_panel: bool,
) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    // Apply theme class to html element
    create_effect(move |_| {
        let theme = app_state.theme.get();
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            if let Some(html) = document.document_element() {
                // Remove existing theme classes
                let _ = html.class_list().remove_1("light");
                let _ = html.class_list().remove_1("dark");
                
                // Apply new theme
                match theme {
                    Theme::Light => {
                        let _ = html.class_list().add_1("light");
                    }
                    Theme::Dark => {
                        // Dark is default, no class needed
                    }
                    Theme::System => {
                        // For System, we'd need to check prefers-color-scheme
                        // For now, default to dark mode
                    }
                }
            }
        }
    });
    
    view! {
        <div class="h-screen flex flex-col bg-theme-bg text-theme overflow-hidden">
            // Top navigation tabs
            <MainTabs />
            
            // Main content area (with optional sidebar)
            <div class="flex-1 flex min-h-0 overflow-hidden">
                // Pipeline sidebar (optional)
                <Show when=move || show_sidebar>
                    <PipelineSidebar />
                </Show>
                
                // Content area
                <main class="flex-1 overflow-hidden flex flex-col min-w-0">
                    // Page content
                    <div class="flex-1 overflow-auto">
                        {children()}
                    </div>
                    
                    // Bottom panel (optional - for pipeline/observability pages)
                    <Show when=move || show_bottom_panel && (app_state.bottom_panel_height.get() > 0.0)>
                        <BottomPanel>
                            {move || {
                                match app_state.bottom_panel_tab.get() {
                                    BottomPanelTab::DataPreview => {
                                        view! {
                                            <div class="h-full overflow-hidden">
                                                <DataPreviewPanel />
                                            </div>
                                        }.into_view()
                                    }
                                    BottomPanelTab::Logs => {
                                        view! {
                                            <div class="h-full p-4 overflow-auto custom-scrollbar">
                                                <LogsPlaceholder />
                                            </div>
                                        }.into_view()
                                    }
                                    BottomPanelTab::TestResults => {
                                        view! {
                                            <div class="h-full p-4 overflow-auto custom-scrollbar">
                                                <TestResultsPlaceholder />
                                            </div>
                                        }.into_view()
                                    }
                                }
                            }}
                        </BottomPanel>
                    </Show>
                </main>
            </div>
            
            // Status bar
            <StatusBar />
        </div>
    }
}

/// Placeholder for logs tab
#[component]
fn LogsPlaceholder() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-full text-theme-muted">
            <div class="text-center">
                <svg class="w-12 h-12 mx-auto mb-3 opacity-50" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                    <polyline points="14 2 14 8 20 8" />
                    <line x1="16" y1="13" x2="8" y2="13" />
                    <line x1="16" y1="17" x2="8" y2="17" />
                </svg>
                <p class="text-sm">"No logs to display"</p>
            </div>
        </div>
    }
}

/// Placeholder for test results tab
#[component]
fn TestResultsPlaceholder() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-full text-theme-muted">
            <div class="text-center">
                <svg class="w-12 h-12 mx-auto mb-3 opacity-50" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
                </svg>
                <p class="text-sm">"Run a functional test to see results"</p>
            </div>
        </div>
    }
}
