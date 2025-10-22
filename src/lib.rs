//! WebUI - A Rust HTML UI framework
//!
//! WebUI provides a way to build interactive user interfaces where the data layer is in Rust
//! and the UI is in HTML, connected by a JSON protocol over WebSocket.
//!
//! # Architecture
//!
//! - **Rust Layer**: Defines UI elements (Button, Text, Input) with id, text, and event handlers,
//!   but no geometry or styling. Manages state and business logic.
//! - **HTML Layer**: Custom HTML elements (`<ui-button>`, `<ui-text>`, `<ui-input>`) that can be
//!   placed and styled in HTML. Handles all presentation and layout.
//! - **JSON Protocol**: Bidirectional WebSocket communication. Client sends events (clicks, input),
//!   server sends UI updates.
//!
//! # HTML Elements
//!
//! The following custom HTML elements are available:
//!
//! ## `<ui-button>`
//!
//! Corresponds to [`UiElement::Button`]. Renders as a clickable button.
//!
//! **Rust Side:**
//! ```rust
//! # use webui::UiElement;
//! # use std::sync::Arc;
//! let button = UiElement::Button {
//!     id: "my-button".to_string(),
//!     text: "Click Me!".to_string(),
//!     on_click: Some(Arc::new(Box::new(|| {
//!         println!("Clicked!");
//!     }))),
//! };
//! ```
//!
//! **HTML Side:**
//! ```html
//! <ui-button id="my-button"></ui-button>
//! ```
//!
//! When clicked, sends a `click` event to the server with the button's ID.
//!
//! ## `<ui-text>`
//!
//! Corresponds to [`UiElement::Text`]. Renders as read-only text.
//!
//! **Rust Side:**
//! ```rust
//! # use webui::UiElement;
//! let text = UiElement::Text {
//!     id: "status".to_string(),
//!     text: "Ready".to_string(),
//! };
//! ```
//!
//! **HTML Side:**
//! ```html
//! <ui-text id="status"></ui-text>
//! ```
//!
//! ## `<ui-input>`
//!
//! Corresponds to [`UiElement::Input`]. Renders as a text input field.
//!
//! **Rust Side:**
//! ```rust
//! # use webui::UiElement;
//! # use std::sync::Arc;
//! let input = UiElement::Input {
//!     id: "name".to_string(),
//!     value: "".to_string(),
//!     on_input: Some(Arc::new(Box::new(|value| {
//!         println!("Input changed to: {}", value);
//!     }))),
//! };
//! ```
//!
//! **HTML Side:**
//! ```html
//! <ui-input id="name"></ui-input>
//! ```
//!
//! When the user types, sends an `input` event to the server with the input's ID and value.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use webui::{AppState, UiElement, RouterConfig, create_router};
//!
//! #[tokio::main]
//! async fn main() {
//!     let state = AppState::new();
//!
//!     // Add UI elements with handlers
//!     state.add_element(UiElement::Button {
//!         id: "btn1".to_string(),
//!         text: "Click Me!".to_string(),
//!         on_click: Some(Arc::new(Box::new(|| {
//!             println!("Button clicked!");
//!         }))),
//!     });
//!
//!     state.add_element(UiElement::Text {
//!         id: "status".to_string(),
//!         text: "Ready".to_string(),
//!     });
//!
//!     // Define HTML layout
//!     let html = r#"
//!         <div>
//!             <ui-button id="btn1"></ui-button>
//!             <ui-text id="status"></ui-text>
//!         </div>
//!     "#;
//!
//!     // Create and run server
//!     let config = RouterConfig::new(state, html);
//!     let app = create_router(config);
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
//!         .await
//!         .unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

/// JSON Protocol: Messages from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "click")]
    Click { id: String },
    #[serde(rename = "input")]
    Input { id: String, value: String },
    #[serde(rename = "change")]
    Change { id: String, value: serde_json::Value },
}

/// JSON Protocol: Messages from server to client
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "init")]
    Init { elements: Vec<UiElement> },
    #[serde(rename = "update")]
    Update { id: String, element: UiElement },
}

type ClickCallback = Option<Arc<Box<dyn Fn() + Send + Sync + 'static>>>;
type InputCallback = Option<Arc<Box<dyn Fn(&str) + Send + Sync + 'static>>>;
type BoolCallback = Option<Arc<Box<dyn Fn(bool) + Send + Sync + 'static>>>;
type NumberCallback = Option<Arc<Box<dyn Fn(f64) + Send + Sync + 'static>>>;

/// UI Element types that can be created in Rust and rendered in HTML.
///
/// Each element has an `id` for identification and element-specific properties.
/// Elements do not contain geometry or styling information - that is handled by HTML/CSS.
#[derive(Clone, Serialize)]
#[serde(tag = "kind")]
pub enum UiElement {
    /// A clickable button.
    ///
    /// # Fields
    /// - `id`: Unique identifier
    /// - `text`: Button label text
    /// - `on_click`: Optional click handler (not serialized)
    ///
    /// # HTML Element
    /// Renders as `<ui-button id="...">text</ui-button>`
    #[serde(rename = "button")]
    Button {
        id: String,
        text: String,
        #[serde(skip)]
        on_click: ClickCallback,
    },

    /// Read-only text display.
    ///
    /// # Fields
    /// - `id`: Unique identifier
    /// - `text`: Text content to display
    ///
    /// # HTML Element
    /// Renders as `<ui-text id="...">text</ui-text>`
    #[serde(rename = "text")]
    Text { id: String, text: String },

    /// Text input field.
    ///
    /// # Fields
    /// - `id`: Unique identifier
    /// - `value`: Current input value
    /// - `on_input`: Optional input change handler (not serialized)
    ///
    /// # HTML Element
    /// Renders as `<ui-input id="...">value</ui-input>`
    #[serde(rename = "input")]
    Input {
        id: String,
        value: String,
        #[serde(skip)]
        on_input: InputCallback,
    },

    /// Checkbox input.
    ///
    /// # Fields
    /// - `id`: Unique identifier
    /// - `checked`: Whether the checkbox is checked
    /// - `on_change`: Optional change handler (not serialized)
    ///
    /// # HTML Element
    /// Renders as `<ui-checkbox id="...">checked</ui-checkbox>`
    #[serde(rename = "checkbox")]
    Checkbox {
        id: String,
        checked: bool,
        #[serde(skip)]
        on_change: BoolCallback,
    },

    /// Slider input (range).
    ///
    /// # Fields
    /// - `id`: Unique identifier
    /// - `value`: Current slider value
    /// - `min`: Minimum value
    /// - `max`: Maximum value
    /// - `step`: Optional step increment
    /// - `on_change`: Optional change handler (not serialized)
    ///
    /// # HTML Element
    /// Renders as `<ui-slider id="...">value</ui-slider>`
    #[serde(rename = "slider")]
    Slider {
        id: String,
        value: f64,
        min: f64,
        max: f64,
        step: Option<f64>,
        #[serde(skip)]
        on_change: NumberCallback,
    },

    /// Radio button input.
    ///
    /// # Fields
    /// - `id`: Unique identifier
    /// - `name`: Group name (radio buttons with same name are mutually exclusive)
    /// - `value`: Value when selected
    /// - `checked`: Whether this radio is selected
    /// - `on_change`: Optional change handler (not serialized)
    ///
    /// # HTML Element
    /// Renders as `<ui-radio id="..." name="...">checked</ui-radio>`
    #[serde(rename = "radio")]
    Radio {
        id: String,
        name: String,
        value: String,
        checked: bool,
        #[serde(skip)]
        on_change: BoolCallback,
    },

    /// Number input field.
    ///
    /// # Fields
    /// - `id`: Unique identifier
    /// - `value`: Current numeric value
    /// - `min`: Optional minimum value
    /// - `max`: Optional maximum value
    /// - `step`: Optional step increment
    /// - `on_change`: Optional change handler (not serialized)
    ///
    /// # HTML Element
    /// Renders as `<ui-number id="...">value</ui-number>`
    #[serde(rename = "number")]
    NumberInput {
        id: String,
        value: f64,
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
        #[serde(skip)]
        on_change: NumberCallback,
    },
}

impl std::fmt::Debug for UiElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiElement::Button { id, text, .. } => f
                .debug_struct("Button")
                .field("id", id)
                .field("text", text)
                .field("on_click", &"<handler>")
                .finish(),
            UiElement::Text { id, text } => f
                .debug_struct("Text")
                .field("id", id)
                .field("text", text)
                .finish(),
            UiElement::Input { id, value, .. } => f
                .debug_struct("Input")
                .field("id", id)
                .field("value", value)
                .field("on_input", &"<handler>")
                .finish(),
            UiElement::Checkbox { id, checked, .. } => f
                .debug_struct("Checkbox")
                .field("id", id)
                .field("checked", checked)
                .field("on_change", &"<handler>")
                .finish(),
            UiElement::Slider { id, value, min, max, step, .. } => f
                .debug_struct("Slider")
                .field("id", id)
                .field("value", value)
                .field("min", min)
                .field("max", max)
                .field("step", step)
                .field("on_change", &"<handler>")
                .finish(),
            UiElement::Radio { id, name, value, checked, .. } => f
                .debug_struct("Radio")
                .field("id", id)
                .field("name", name)
                .field("value", value)
                .field("checked", checked)
                .field("on_change", &"<handler>")
                .finish(),
            UiElement::NumberInput { id, value, min, max, step, .. } => f
                .debug_struct("NumberInput")
                .field("id", id)
                .field("value", value)
                .field("min", min)
                .field("max", max)
                .field("step", step)
                .field("on_change", &"<handler>")
                .finish(),
        }
    }
}

/// Application state managing UI elements and event handlers.
///
/// The `AppState` is the core of the WebUI framework. It:
/// - Stores all UI elements by ID (buttons and inputs include their handlers)
/// - Broadcasts updates to all connected WebSocket clients
///
/// # Thread Safety
/// `AppState` is designed to be shared across multiple async tasks and cloned freely.
/// All mutations are protected by internal locks.
#[derive(Clone)]
pub struct AppState {
    elements: Arc<Mutex<HashMap<String, UiElement>>>,
    update_tx: broadcast::Sender<ServerMessage>,
}

impl AppState {
    /// Creates a new `AppState` instance.
    ///
    /// # Example
    /// ```
    /// use webui::AppState;
    ///
    /// let state = AppState::new();
    /// ```
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            elements: Arc::new(Mutex::new(HashMap::new())),
            update_tx: tx,
        }
    }

    /// Adds a UI element to the application.
    ///
    /// If an element with the same ID already exists, it will be replaced.
    ///
    /// # Example
    /// ```
    /// use webui::{AppState, UiElement};
    ///
    /// let state = AppState::new();
    /// state.add_element(UiElement::Button {
    ///     id: "btn1".to_string(),
    ///     text: "Click Me!".to_string(),
    ///     on_click: None,
    /// });
    /// ```
    pub fn add_element(&self, element: UiElement) {
        let id = match &element {
            UiElement::Button { id, .. } => id.clone(),
            UiElement::Text { id, .. } => id.clone(),
            UiElement::Input { id, .. } => id.clone(),
            UiElement::Checkbox { id, .. } => id.clone(),
            UiElement::Slider { id, .. } => id.clone(),
            UiElement::Radio { id, .. } => id.clone(),
            UiElement::NumberInput { id, .. } => id.clone(),
        };
        self.elements.lock().unwrap().insert(id, element);
    }

    /// Updates an existing element and broadcasts the change to all connected clients.
    ///
    /// # Example
    /// ```
    /// # use webui::{AppState, UiElement};
    /// # let state = AppState::new();
    /// state.update_element(
    ///     "status",
    ///     UiElement::Text {
    ///         id: "status".to_string(),
    ///         text: "Updated!".to_string(),
    ///     },
    /// );
    /// ```
    pub fn update_element(&self, id: &str, element: UiElement) {
        self.elements.lock().unwrap().insert(id.to_string(), element.clone());
        let _ = self.update_tx.send(ServerMessage::Update {
            id: id.to_string(),
            element,
        });
    }

    /// Gets all UI elements.
    ///
    /// Returns a vector of cloned elements. Used internally when initializing new clients.
    pub fn get_all_elements(&self) -> Vec<UiElement> {
        self.elements.lock().unwrap().values().cloned().collect()
    }

    fn handle_click(&self, id: &str) {
        let handler = {
            let elements = self.elements.lock().unwrap();
            if let Some(UiElement::Button { on_click: Some(handler), .. }) = elements.get(id) {
                Some(handler.clone())
            } else {
                None
            }
        };
        if let Some(handler) = handler {
            handler();
        }
    }

    fn handle_input(&self, id: &str, value: &str) {
        let handler = {
            let elements = self.elements.lock().unwrap();
            if let Some(UiElement::Input { on_input: Some(handler), .. }) = elements.get(id) {
                Some(handler.clone())
            } else {
                None
            }
        };
        if let Some(handler) = handler {
            handler(value);
        }
    }

    fn handle_change(&self, id: &str, value: serde_json::Value) {
        enum HandlerCall {
            Bool(Arc<Box<dyn Fn(bool) + Send + Sync + 'static>>, bool),
            Number(Arc<Box<dyn Fn(f64) + Send + Sync + 'static>>, f64),
        }

        let handler_call = {
            let elements = self.elements.lock().unwrap();
            if let Some(element) = elements.get(id) {
                match element {
                    UiElement::Checkbox { on_change: Some(handler), .. } => {
                        value.as_bool().map(|checked| HandlerCall::Bool(handler.clone(), checked))
                    }
                    UiElement::Slider { on_change: Some(handler), .. } => {
                        value.as_f64().map(|num| HandlerCall::Number(handler.clone(), num))
                    }
                    UiElement::Radio { on_change: Some(handler), .. } => {
                        value.as_bool().map(|checked| HandlerCall::Bool(handler.clone(), checked))
                    }
                    UiElement::NumberInput { on_change: Some(handler), .. } => {
                        value.as_f64().map(|num| HandlerCall::Number(handler.clone(), num))
                    }
                    _ => None
                }
            } else {
                None
            }
        };

        if let Some(handler_call) = handler_call {
            match handler_call {
                HandlerCall::Bool(handler, value) => handler(value),
                HandlerCall::Number(handler, value) => handler(value),
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: AppState) {
    use futures_util::sink::SinkExt;
    use futures_util::stream::StreamExt;

    let (mut sender, mut receiver) = stream.split();

    // Send initial UI state
    let init_msg = ServerMessage::Init {
        elements: state.get_all_elements(),
    };
    let json = serde_json::to_string(&init_msg).unwrap();
    if sender.send(Message::Text(json)).await.is_err() {
        return;
    }

    // Subscribe to updates from the app state
    let mut update_rx = state.update_tx.subscribe();

    // Spawn task to forward updates to this client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = update_rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg
                && let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Click { id } => {
                        state_clone.handle_click(&id);
                    }
                    ClientMessage::Input { id, value } => {
                        state_clone.handle_input(&id, &value);
                    }
                    ClientMessage::Change { id, value } => {
                        state_clone.handle_change(&id, value);
                    }
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

// Default HTML template - wraps user content
fn generate_html(title: &str, body_content: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="stylesheet" href="/static/webui.css">
</head>
<body>
{body_content}
    <script src="/static/webui.js"></script>
</body>
</html>"#, title = title, body_content = body_content)
}

/// Configuration for creating a WebUI router
pub struct RouterConfig {
    /// Application state
    pub state: AppState,
    /// Path to static files directory
    pub static_dir: String,
    /// HTML page title
    pub title: String,
    /// HTML body content (the UI layout)
    pub body_html: String,
}

impl RouterConfig {
    /// Creates a new router configuration
    pub fn new(state: AppState, body_html: impl Into<String>) -> Self {
        Self {
            state,
            static_dir: "static".to_string(),
            title: "WebUI App".to_string(),
            body_html: body_html.into(),
        }
    }

    /// Sets the page title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the static files directory
    pub fn static_dir(mut self, dir: impl Into<String>) -> Self {
        self.static_dir = dir.into();
        self
    }
}

/// Creates an Axum router configured for WebUI.
///
/// The router includes:
/// - `/` - Serves the main HTML page with your custom UI layout
/// - `/ws` - WebSocket endpoint for UI communication
/// - `/static` - Serves static files (webui.js, webui.css, etc.)
///
/// # Arguments
/// - `config`: Router configuration with state and HTML content
///
/// # Example
/// ```no_run
/// use webui::{AppState, RouterConfig, create_router};
///
/// #[tokio::main]
/// async fn main() {
///     let state = AppState::new();
///
///     let html = r#"
///         <div class="container">
///             <h1>My App</h1>
///             <ui-button id="btn1"></ui-button>
///             <ui-text id="status"></ui-text>
///         </div>
///     "#;
///
///     let config = RouterConfig::new(state.clone(), html)
///         .title("My WebUI App");
///
///     let app = create_router(config);
///
///     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
///         .await
///         .unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
pub fn create_router(config: RouterConfig) -> Router {
    let html_content = generate_html(&config.title, &config.body_html);
    let state = config.state.clone();

    Router::new()
        .route("/", get(move || async move {
            Html(html_content)
        }))
        .route("/ws", get(websocket_handler))
        .nest_service("/static", ServeDir::new(config.static_dir))
        .with_state(state)
}

/// Convenience function to start a WebUI server.
///
/// This is a helper that combines creating a router and starting the server.
///
/// # Arguments
/// - `state`: The application state
/// - `html`: HTML body content for the UI layout
/// - `title`: Page title (optional, defaults to "WebUI App")
/// - `addr`: Address to bind to (e.g., "127.0.0.1:3000")
///
/// # Example
/// ```no_run
/// use webui::{AppState, UiElement, start_server};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let state = AppState::new();
///
///     state.add_element(UiElement::Button {
///         id: "btn1".to_string(),
///         text: "Click Me!".to_string(),
///         on_click: Some(Arc::new(Box::new(|| {
///             println!("Clicked!");
///         }))),
///     });
///
///     let html = r#"<ui-button id="btn1"></ui-button>"#;
///
///     start_server(state, html, "My App", "127.0.0.1:3000").await.unwrap();
/// }
/// ```
pub async fn start_server(
    state: AppState,
    html: impl Into<String>,
    title: impl Into<String>,
    addr: impl AsRef<str>,
) -> Result<(), std::io::Error> {
    let config = RouterConfig::new(state, html).title(title);
    let app = create_router(config);

    let listener = tokio::net::TcpListener::bind(addr.as_ref()).await?;
    println!("Server running on http://{}", addr.as_ref());

    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use headless_chrome::{Browser, Tab};
    use std::sync::Arc;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        state.add_element(UiElement::Button {
            id: "btn1".to_string(),
            text: "Test".to_string(),
            on_click: None,
        });

        let elements = state.get_all_elements();
        assert_eq!(elements.len(), 1);
    }

    // Test helper: Start a web server on a random port and wait for it to be ready
    async fn start_test_server(state: AppState, html: &str, title: &str) -> u16 {
        let config = RouterConfig::new(state, html).title(title);
        let app = create_router(config);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind");
        let port = listener.local_addr().expect("Failed to get address").port();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Wait for server to be ready by polling HTTP endpoint
        let url = format!("http://127.0.0.1:{}", port);
        let client = reqwest::Client::new();
        for _ in 0..10 {
            if client.get(&url).send().await.is_ok() {
                return port;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        panic!("Server failed to start");
    }

    // Test helper: Create browser and navigate to URL in blocking thread
    async fn create_browser_and_navigate(url: &str) -> (Arc<Browser>, Arc<Tab>) {
        let url = url.to_string();
        tokio::task::spawn_blocking(move || {
            let browser = Browser::default().expect("Failed to launch browser");
            let tab = browser.new_tab().expect("Failed to create tab");
            tab.navigate_to(&url).expect("Failed to navigate");
            tab.wait_for_element("body").expect("Failed to find body");
            (Arc::new(browser), tab)
        })
        .await
        .expect("Browser task panicked")
    }

    #[tokio::test]
    async fn test_button_click_e2e() {
        let state = AppState::new();

        // Track whether button was clicked
        let clicked = Arc::new(Mutex::new(false));
        let clicked_clone = clicked.clone();

        state.add_element(UiElement::Button {
            id: "test-btn".to_string(),
            text: "Test Button".to_string(),
            on_click: Some(Arc::new(Box::new(move || {
                *clicked_clone.lock().unwrap() = true;
            }))),
        });

        let html = r#"<ui-button id="test-btn"></ui-button>"#;
        let port = start_test_server(state, html, "Button Test").await;
        let url = format!("http://127.0.0.1:{}", port);

        let (_browser, tab) = create_browser_and_navigate(&url).await;

        // Click button in blocking thread
        tokio::task::spawn_blocking(move || {
            let button = tab.wait_for_element("ui-button#test-btn").expect("Failed to find button");
            button.click().expect("Failed to click button");
        })
        .await
        .expect("Click task panicked");

        // Wait for click event to propagate through WebSocket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the button was clicked
        assert!(*clicked.lock().unwrap(), "Button click handler was not called");
    }

    #[tokio::test]
    async fn test_input_e2e() {
        let state = AppState::new();

        // Track input value
        let input_value = Arc::new(Mutex::new(String::new()));
        let input_value_clone = input_value.clone();

        state.add_element(UiElement::Input {
            id: "test-input".to_string(),
            value: "".to_string(),
            on_input: Some(Arc::new(Box::new(move |value| {
                *input_value_clone.lock().unwrap() = value.to_string();
            }))),
        });

        let html = r#"<ui-input id="test-input"></ui-input>"#;
        let port = start_test_server(state, html, "Input Test").await;
        let url = format!("http://127.0.0.1:{}", port);

        let (_browser, tab) = create_browser_and_navigate(&url).await;

        // Type into input in blocking thread
        tokio::task::spawn_blocking(move || {
            let input = tab.wait_for_element("ui-input#test-input input").expect("Failed to find input");
            input.type_into("Hello World").expect("Failed to type into input");
        })
        .await
        .expect("Input task panicked");

        // Wait for input event to propagate through WebSocket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the input handler was called with the correct value
        let final_value = input_value.lock().unwrap();
        assert!(!final_value.is_empty(), "Input handler was not called");
        assert!(final_value.contains("Hello World"),
            "Input handler received incorrect value: expected 'Hello World', got '{}'",
            final_value);
    }

    #[tokio::test]
    async fn test_checkbox_e2e() {
        let state = AppState::new();

        // Track checkbox state
        let checked_state = Arc::new(Mutex::new(false));
        let checked_state_clone = checked_state.clone();

        state.add_element(UiElement::Checkbox {
            id: "test-checkbox".to_string(),
            checked: false,
            on_change: Some(Arc::new(Box::new(move |checked| {
                *checked_state_clone.lock().unwrap() = checked;
            }))),
        });

        let html = r#"<ui-checkbox id="test-checkbox"></ui-checkbox>"#;
        let port = start_test_server(state, html, "Checkbox Test").await;
        let url = format!("http://127.0.0.1:{}", port);

        let (_browser, tab) = create_browser_and_navigate(&url).await;

        // Click checkbox in blocking thread
        tokio::task::spawn_blocking(move || {
            let checkbox = tab.wait_for_element("ui-checkbox#test-checkbox input").expect("Failed to find checkbox");
            checkbox.click().expect("Failed to click checkbox");
        })
        .await
        .expect("Checkbox task panicked");

        // Wait for change event to propagate through WebSocket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the checkbox was toggled
        assert!(*checked_state.lock().unwrap(), "Checkbox change handler was not called or incorrect value");
    }

    #[tokio::test]
    async fn test_slider_e2e() {
        let state = AppState::new();

        // Track slider value
        let slider_value = Arc::new(Mutex::new(0.0));
        let slider_value_clone = slider_value.clone();

        state.add_element(UiElement::Slider {
            id: "test-slider".to_string(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: Some(1.0),
            on_change: Some(Arc::new(Box::new(move |value| {
                *slider_value_clone.lock().unwrap() = value;
            }))),
        });

        let html = r#"<ui-slider id="test-slider"></ui-slider>"#;
        let port = start_test_server(state, html, "Slider Test").await;
        let url = format!("http://127.0.0.1:{}", port);

        let (_browser, tab) = create_browser_and_navigate(&url).await;

        // Set slider value in blocking thread
        tokio::task::spawn_blocking(move || {
            tab.wait_for_element("ui-slider#test-slider input[type='range']").expect("Failed to find slider");
            // Set value and trigger change event
            tab.evaluate("const slider = document.querySelector('ui-slider#test-slider input'); slider.value = 75; slider.dispatchEvent(new Event('change', { bubbles: true }));", false)
                .expect("Failed to set slider value and dispatch event");
        })
        .await
        .expect("Slider task panicked");

        // Wait for change event to propagate through WebSocket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the slider value was changed
        let final_value = *slider_value.lock().unwrap();
        assert!(final_value > 0.0, "Slider change handler was not called");
    }

    #[tokio::test]
    async fn test_radio_e2e() {
        let state = AppState::new();

        // Track radio state
        let radio_checked = Arc::new(Mutex::new(false));
        let radio_checked_clone = radio_checked.clone();

        state.add_element(UiElement::Radio {
            id: "test-radio".to_string(),
            name: "test-group".to_string(),
            value: "option1".to_string(),
            checked: false,
            on_change: Some(Arc::new(Box::new(move |checked| {
                *radio_checked_clone.lock().unwrap() = checked;
            }))),
        });

        let html = r#"<ui-radio id="test-radio" name="test-group"></ui-radio>"#;
        let port = start_test_server(state, html, "Radio Test").await;
        let url = format!("http://127.0.0.1:{}", port);

        let (_browser, tab) = create_browser_and_navigate(&url).await;

        // Click radio button in blocking thread
        tokio::task::spawn_blocking(move || {
            let radio = tab.wait_for_element("ui-radio#test-radio input").expect("Failed to find radio");
            radio.click().expect("Failed to click radio");
        })
        .await
        .expect("Radio task panicked");

        // Wait for change event to propagate through WebSocket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the radio was selected
        assert!(*radio_checked.lock().unwrap(), "Radio change handler was not called or incorrect value");
    }

    #[tokio::test]
    async fn test_number_input_e2e() {
        let state = AppState::new();

        // Track number value
        let number_value = Arc::new(Mutex::new(0.0));
        let number_value_clone = number_value.clone();

        state.add_element(UiElement::NumberInput {
            id: "test-number".to_string(),
            value: 0.0,
            min: Some(0.0),
            max: Some(100.0),
            step: Some(1.0),
            on_change: Some(Arc::new(Box::new(move |value| {
                *number_value_clone.lock().unwrap() = value;
            }))),
        });

        let html = r#"<ui-number id="test-number"></ui-number>"#;
        let port = start_test_server(state, html, "Number Test").await;
        let url = format!("http://127.0.0.1:{}", port);

        let (_browser, tab) = create_browser_and_navigate(&url).await;

        // Set number value in blocking thread
        tokio::task::spawn_blocking(move || {
            tab.wait_for_element("ui-number#test-number input[type='number']").expect("Failed to find number input");
            // Set value and trigger change event
            tab.evaluate("const number = document.querySelector('ui-number#test-number input'); number.value = 42; number.dispatchEvent(new Event('change', { bubbles: true }));", false)
                .expect("Failed to set number value and dispatch event");
        })
        .await
        .expect("Number input task panicked");

        // Wait for change event to propagate through WebSocket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the number value was changed
        let final_value = *number_value.lock().unwrap();
        assert!(final_value > 0.0, "Number input change handler was not called");
        assert!((final_value - 42.0).abs() < 0.01, "Number input received incorrect value: expected 42, got {}", final_value);
    }
}
