//! Comprehensive WebUI Example
//!
//! This example demonstrates all available UI elements:
//! - Button
//! - Text
//! - Input
//! - Checkbox
//! - Slider
//! - Radio Button
//! - Number Input
//!
//! Run with: cargo run --example all_elements
//! Then open http://127.0.0.1:3000 in your browser

use std::sync::Arc;
use webui::{AppState, UiElement, start_server};

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Create the application state
    let state = AppState::new();

    // Button elements
    let state_for_btn = state.clone();
    state.add_element(UiElement::Button {
        id: "submit-btn".to_string(),
        text: "Submit".to_string(),
        on_click: Some(Arc::new(Box::new(move || {
            println!("Submit button clicked!");
            state_for_btn.update_element(
                "status",
                UiElement::Text {
                    id: "status".to_string(),
                    text: "Form submitted!".to_string(),
                },
            );
        }))),
    });

    // Text elements
    state.add_element(UiElement::Text {
        id: "status".to_string(),
        text: "Ready".to_string(),
    });

    state.add_element(UiElement::Text {
        id: "echo".to_string(),
        text: "...".to_string(),
    });

    state.add_element(UiElement::Text {
        id: "slider-value".to_string(),
        text: "50".to_string(),
    });

    state.add_element(UiElement::Text {
        id: "quantity-display".to_string(),
        text: "1".to_string(),
    });

    // Text input
    let state_for_input = state.clone();
    state.add_element(UiElement::Input {
        id: "name-input".to_string(),
        value: "".to_string(),
        on_input: Some(Arc::new(Box::new(move |value| {
            println!("Name input: {}", value);
            state_for_input.update_element(
                "echo",
                UiElement::Text {
                    id: "echo".to_string(),
                    text: format!("Hello, {}!", value),
                },
            );
        }))),
    });

    // Checkbox
    state.add_element(UiElement::Checkbox {
        id: "terms-checkbox".to_string(),
        checked: false,
        on_change: Some(Arc::new(Box::new(|checked| {
            println!("Terms accepted: {}", checked);
        }))),
    });

    // Slider
    let state_for_slider = state.clone();
    state.add_element(UiElement::Slider {
        id: "volume-slider".to_string(),
        value: 50.0,
        min: 0.0,
        max: 100.0,
        step: Some(1.0),
        on_change: Some(Arc::new(Box::new(move |value| {
            println!("Volume: {}", value);
            state_for_slider.update_element(
                "slider-value",
                UiElement::Text {
                    id: "slider-value".to_string(),
                    text: format!("{:.0}", value),
                },
            );
        }))),
    });

    // Radio buttons
    state.add_element(UiElement::Radio {
        id: "option-a".to_string(),
        name: "choice".to_string(),
        value: "a".to_string(),
        checked: true,
        on_change: Some(Arc::new(Box::new(|checked| {
            if checked {
                println!("Selected: Option A");
            }
        }))),
    });

    state.add_element(UiElement::Radio {
        id: "option-b".to_string(),
        name: "choice".to_string(),
        value: "b".to_string(),
        checked: false,
        on_change: Some(Arc::new(Box::new(|checked| {
            if checked {
                println!("Selected: Option B");
            }
        }))),
    });

    state.add_element(UiElement::Radio {
        id: "option-c".to_string(),
        name: "choice".to_string(),
        value: "c".to_string(),
        checked: false,
        on_change: Some(Arc::new(Box::new(|checked| {
            if checked {
                println!("Selected: Option C");
            }
        }))),
    });

    // Number input
    let state_for_number = state.clone();
    state.add_element(UiElement::NumberInput {
        id: "quantity-input".to_string(),
        value: 1.0,
        min: Some(1.0),
        max: Some(100.0),
        step: Some(1.0),
        on_change: Some(Arc::new(Box::new(move |value| {
            println!("Quantity: {}", value);
            state_for_number.update_element(
                "quantity-display",
                UiElement::Text {
                    id: "quantity-display".to_string(),
                    text: format!("{:.0} items", value),
                },
            );
        }))),
    });

    // Define the UI layout in HTML
    let html = include_str!("all_elements.html");

    // Start the server using the convenient helper function
    start_server(state, html, "WebUI - All Elements", "127.0.0.1:3000")
        .await
        .unwrap();
}
