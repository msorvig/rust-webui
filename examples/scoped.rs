//! Scoped Elements Example
//!
//! This example demonstrates how to use scoped states to avoid ID conflicts.
//! Multiple UI sections can use the same local IDs without interfering with each other.
//!
//! Run with: cargo run --example scoped
//! Then open http://127.0.0.1:3000 in your browser

use std::sync::Arc;
use webui::{AppState, UiElement, start_server};

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Create the root application state
    let state = AppState::new();

    // Create scoped states for different UI sections
    let form_state = state.scope("form");
    let modal_state = state.scope("modal");

    // Both sections can use the same local IDs without conflict
    // Form section
    let form_state_for_btn = form_state.clone();
    form_state.add_element(UiElement::Button {
        id: "submit".to_string(),
        text: "Submit Form".to_string(),
        on_click: Some(Arc::new(Box::new(move || {
            println!("Form submitted!");
            form_state_for_btn.update_element(
                "status",
                UiElement::Text {
                    id: "status".to_string(),
                    text: "Form submitted successfully!".to_string(),
                },
            );
        }))),
    });

    form_state.add_element(UiElement::Text {
        id: "status".to_string(),
        text: "Ready to submit".to_string(),
    });

    form_state.add_element(UiElement::Input {
        id: "name".to_string(),
        value: "".to_string(),
        on_input: None,
    });

    // Modal section - uses same local IDs!
    let modal_state_for_btn = modal_state.clone();
    modal_state.add_element(UiElement::Button {
        id: "submit".to_string(),  // Same local ID as form's submit button
        text: "Close Modal".to_string(),
        on_click: Some(Arc::new(Box::new(move || {
            println!("Modal closed!");
            modal_state_for_btn.update_element(
                "status",  // Same local ID as form's status text
                UiElement::Text {
                    id: "status".to_string(),
                    text: "Modal closed!".to_string(),
                },
            );
        }))),
    });

    modal_state.add_element(UiElement::Text {
        id: "status".to_string(),  // Same local ID as form's status text
        text: "Modal is open".to_string(),
    });

    // Define the UI layout in HTML
    // Note: Both sections use identical local IDs (submit, status)
    // The <ui-scope> containers namespace them automatically
    // Load from external file for better editing experience
    let html = include_str!("scoped.html");

    // Start the server
    start_server(state, html, "WebUI Scoped Example", "127.0.0.1:3000")
        .await
        .unwrap();
}
