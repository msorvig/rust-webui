//! Basic WebUI Example
//!
//! This example demonstrates the core features of WebUI:
//! - Creating UI elements (buttons, text, input)
//! - Specifying UI layout in HTML
//! - Handling click events
//! - Handling input events
//! - Updating UI elements dynamically
//!
//! Run with: cargo run --example basic
//! Then open http://127.0.0.1:3000 in your browser

use std::sync::Arc;
use webui::{AppState, UiElement, RouterConfig, create_router};

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Create the application state
    let state = AppState::new();

    // Add UI elements
    let state_for_btn1 = state.clone();
    state.add_element(UiElement::Button {
        id: "btn1".to_string(),
        text: "Click Me!".to_string(),
        on_click: Some(Arc::new(Box::new(move || {
            println!("Button 1 clicked!");
            state_for_btn1.update_element(
                "status",
                UiElement::Text {
                    id: "status".to_string(),
                    text: "Button 1 clicked!".to_string(),
                },
            );
        }))),
    });

    let state_for_btn2 = state.clone();
    state.add_element(UiElement::Button {
        id: "btn2".to_string(),
        text: "Or Click Me!".to_string(),
        on_click: Some(Arc::new(Box::new(move || {
            println!("Button 2 clicked!");
            state_for_btn2.update_element(
                "status",
                UiElement::Text {
                    id: "status".to_string(),
                    text: "Button 2 clicked!".to_string(),
                },
            );
        }))),
    });

    state.add_element(UiElement::Text {
        id: "status".to_string(),
        text: "Ready".to_string(),
    });

    state.add_element(UiElement::Text {
        id: "echo".to_string(),
        text: "Type something above...".to_string(),
    });

    let state_for_input = state.clone();
    state.add_element(UiElement::Input {
        id: "name".to_string(),
        value: "".to_string(),
        on_input: Some(Arc::new(Box::new(move |value| {
            println!("Input changed: {}", value);
            state_for_input.update_element(
                "echo",
                UiElement::Text {
                    id: "echo".to_string(),
                    text: format!("You typed: {}", value),
                },
            );
        }))),
    });

    // Define the UI layout in HTML
    // Load from external file for better editing experience
    let html = include_str!("basic.html");

    // Create the router with HTML layout
    let config = RouterConfig::new(state.clone(), html)
        .title("WebUI Basic Example");

    let app = create_router(config);

    // Bind to localhost:3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Server running on http://127.0.0.1:3000");

    // Run the server
    axum::serve(listener, app).await.unwrap();
}
