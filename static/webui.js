/**
 * WebUI - JavaScript client library
 *
 * Provides custom HTML elements and WebSocket communication for WebUI applications.
 *
 * Usage:
 * 1. Include this script in your HTML: <script src="/static/webui.js"></script>
 * 2. Add UI elements to your HTML: <ui-button id="btn1"></ui-button>
 * 3. The WebUI client will automatically connect and synchronize with the server
 */

// Helper function to get the scope path for an element
function getElementScopePath(element) {
    const scopes = [];
    let current = element.parentElement;

    while (current) {
        if (current.tagName && current.tagName.toLowerCase() === 'ui-scope') {
            const scopeName = current.getAttribute('name');
            if (scopeName) {
                scopes.unshift(scopeName);
            }
        }
        current = current.parentElement;
    }

    return scopes.join('.');
}

// Helper function to build full scoped ID
function buildScopedId(element, localId) {
    const scopePath = getElementScopePath(element);
    if (scopePath) {
        return `${scopePath}.${localId}`;
    }
    return localId;
}

// Helper function to auto-rewrite an element's ID based on scope
// Call this at the start of connectedCallback for all UI elements
function autoRewriteId(element) {
    if (element.id) {
        element._originalId = element.id;
        const scopedId = buildScopedId(element, element.id);
        if (scopedId !== element.id) {
            element.id = scopedId;
        }
    }
}

// Custom UI Elements

/**
 * <ui-scope> - Container for namespacing UI elements
 *
 * Attributes:
 *   name - The scope name (required)
 *
 * Elements inside a scope have their IDs automatically prefixed with the scope path.
 */
class UiScope extends HTMLElement {
    constructor() {
        super();
    }

    connectedCallback() {
        // Scope is just a container, no special rendering needed
        // The magic happens in the child elements
    }
}

/**
 * <ui-button> - A clickable button
 *
 * Attributes:
 *   id - Unique identifier (required, will be auto-scoped)
 *
 * The button text is set by the server via the JSON protocol.
 */
class UiButton extends HTMLElement {
    constructor() {
        super();
        this._button = document.createElement('button');
    }

    connectedCallback() {
        autoRewriteId(this);

        this.appendChild(this._button);
        this._button.addEventListener('click', () => {
            this.dispatchEvent(new CustomEvent('ui-click', {
                bubbles: true,
                detail: { id: this.id }
            }));
        });
    }

    setText(text) {
        this._button.textContent = text;
    }
}

/**
 * <ui-text> - Read-only text display
 *
 * Attributes:
 *   id - Unique identifier (required, will be auto-scoped)
 *
 * The text content is set by the server via the JSON protocol.
 */
class UiText extends HTMLElement {
    constructor() {
        super();
        this._span = document.createElement('span');
    }

    connectedCallback() {
        autoRewriteId(this);
        this.appendChild(this._span);
    }

    setText(text) {
        this._span.textContent = text;
    }
}

/**
 * <ui-input> - Text input field
 *
 * Attributes:
 *   id - Unique identifier (required, will be auto-scoped)
 *
 * The input value is synchronized between client and server.
 */
class UiInput extends HTMLElement {
    constructor() {
        super();
        this._label = document.createElement('label');
        this._input = document.createElement('input');
        this._input.type = 'text';
        this._label.appendChild(this._input);
    }

    connectedCallback() {
        autoRewriteId(this);

        this.appendChild(this._label);
        this._input.addEventListener('input', () => {
            this.dispatchEvent(new CustomEvent('ui-input', {
                bubbles: true,
                detail: { id: this.id, value: this._input.value }
            }));
        });
    }

    setValue(value) {
        this._input.value = value;
    }

    setLabel(label) {
        const labelText = document.createTextNode(label + ': ');
        this._label.insertBefore(labelText, this._input);
    }
}

/**
 * <ui-checkbox> - Checkbox input
 *
 * Attributes:
 *   id - Unique identifier (required, will be auto-scoped)
 *
 * The checked state is synchronized between client and server.
 */
class UiCheckbox extends HTMLElement {
    constructor() {
        super();
        this._label = document.createElement('label');
        this._input = document.createElement('input');
        this._input.type = 'checkbox';
        this._label.appendChild(this._input);
    }

    connectedCallback() {
        autoRewriteId(this);

        this.appendChild(this._label);
        this._input.addEventListener('change', () => {
            this.dispatchEvent(new CustomEvent('ui-change', {
                bubbles: true,
                detail: { id: this.id, value: this._input.checked }
            }));
        });
    }

    setChecked(checked) {
        this._input.checked = checked;
    }
}

/**
 * <ui-slider> - Slider (range) input
 *
 * Attributes:
 *   id - Unique identifier (required, will be auto-scoped)
 *
 * The slider value is synchronized between client and server.
 */
class UiSlider extends HTMLElement {
    constructor() {
        super();
        this._input = document.createElement('input');
        this._input.type = 'range';
    }

    connectedCallback() {
        autoRewriteId(this);

        this.appendChild(this._input);
        this._input.addEventListener('change', () => {
            this.dispatchEvent(new CustomEvent('ui-change', {
                bubbles: true,
                detail: { id: this.id, value: parseFloat(this._input.value) }
            }));
        });
    }

    setValue(value, min, max, step) {
        this._input.value = value;
        this._input.min = min;
        this._input.max = max;
        if (step !== null && step !== undefined) {
            this._input.step = step;
        }
    }
}

/**
 * <ui-radio> - Radio button input
 *
 * Attributes:
 *   id - Unique identifier (required, will be auto-scoped)
 *   name - Group name for mutual exclusion (required)
 *
 * The checked state is synchronized between client and server.
 */
class UiRadio extends HTMLElement {
    constructor() {
        super();
        this._label = document.createElement('label');
        this._input = document.createElement('input');
        this._input.type = 'radio';
        this._label.appendChild(this._input);
    }

    connectedCallback() {
        autoRewriteId(this);

        this.appendChild(this._label);
        this._input.addEventListener('change', () => {
            this.dispatchEvent(new CustomEvent('ui-change', {
                bubbles: true,
                detail: { id: this.id, value: this._input.checked }
            }));
        });
    }

    setChecked(checked, name, value) {
        this._input.checked = checked;
        this._input.name = name;
        this._input.value = value;
    }
}

/**
 * <ui-number> - Number input field
 *
 * Attributes:
 *   id - Unique identifier (required, will be auto-scoped)
 *
 * The numeric value is synchronized between client and server.
 */
class UiNumber extends HTMLElement {
    constructor() {
        super();
        this._input = document.createElement('input');
        this._input.type = 'number';
    }

    connectedCallback() {
        autoRewriteId(this);

        this.appendChild(this._input);
        this._input.addEventListener('change', () => {
            this.dispatchEvent(new CustomEvent('ui-change', {
                bubbles: true,
                detail: { id: this.id, value: parseFloat(this._input.value) }
            }));
        });
    }

    setValue(value, min, max, step) {
        this._input.value = value;
        if (min !== null && min !== undefined) {
            this._input.min = min;
        }
        if (max !== null && max !== undefined) {
            this._input.max = max;
        }
        if (step !== null && step !== undefined) {
            this._input.step = step;
        }
    }
}

// Register custom elements
customElements.define('ui-scope', UiScope);
customElements.define('ui-button', UiButton);
customElements.define('ui-text', UiText);
customElements.define('ui-input', UiInput);
customElements.define('ui-checkbox', UiCheckbox);
customElements.define('ui-slider', UiSlider);
customElements.define('ui-radio', UiRadio);
customElements.define('ui-number', UiNumber);

/**
 * WebUIClient - Manages WebSocket connection and UI synchronization
 *
 * Automatically connects to the server, handles reconnection,
 * and synchronizes UI elements.
 */
class WebUIClient {
    constructor() {
        this.ws = null;
        this.elements = new Map();
        this.connect();
    }

    connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        this.ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

        this.ws.onopen = () => {
            console.log('WebUI: Connected');
            this.updateConnectionStatus(true);
        };

        this.ws.onclose = () => {
            console.log('WebUI: Disconnected');
            this.updateConnectionStatus(false);
            setTimeout(() => this.connect(), 2000);
        };

        this.ws.onerror = (error) => {
            console.error('WebUI: Error', error);
        };

        this.ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            this.handleMessage(msg);
        };
    }

    updateConnectionStatus(connected) {
        // Update connection indicator if present
        const statusEl = document.querySelector('.webui-connection-status');
        if (statusEl) {
            if (connected) {
                statusEl.className = 'webui-connection-status connected';
                statusEl.textContent = 'Connected';
            } else {
                statusEl.className = 'webui-connection-status disconnected';
                statusEl.textContent = 'Disconnected';
            }
        }

        // Dispatch event for custom handling
        document.dispatchEvent(new CustomEvent('webui-connection', {
            detail: { connected }
        }));
    }

    handleMessage(msg) {
        console.log('WebUI: Received', msg);

        switch (msg.type) {
            case 'init':
                this.initializeUI(msg.elements);
                break;
            case 'update':
                this.updateElement(msg.id, msg.element);
                break;
        }
    }

    initializeUI(elements) {
        elements.forEach(element => {
            this.updateElement(element.id, element);
        });
    }

    updateElement(id, data) {
        // Find the element in the DOM
        let el = document.getElementById(id);

        if (!el) {
            console.warn(`WebUI: Element with id="${id}" not found in DOM`);
            return;
        }

        // Store reference
        this.elements.set(id, el);

        // Update the element based on its type
        switch (data.kind) {
            case 'button':
                if (el.tagName.toLowerCase() === 'ui-button') {
                    el.setText(data.text);
                }
                break;
            case 'text':
                if (el.tagName.toLowerCase() === 'ui-text') {
                    el.setText(data.text);
                }
                break;
            case 'input':
                if (el.tagName.toLowerCase() === 'ui-input') {
                    el.setValue(data.value);
                }
                break;
            case 'checkbox':
                if (el.tagName.toLowerCase() === 'ui-checkbox') {
                    el.setChecked(data.checked);
                }
                break;
            case 'slider':
                if (el.tagName.toLowerCase() === 'ui-slider') {
                    el.setValue(data.value, data.min, data.max, data.step);
                }
                break;
            case 'radio':
                if (el.tagName.toLowerCase() === 'ui-radio') {
                    el.setChecked(data.checked, data.name, data.value);
                }
                break;
            case 'number':
                if (el.tagName.toLowerCase() === 'ui-number') {
                    el.setValue(data.value, data.min, data.max, data.step);
                }
                break;
        }
    }

    sendClick(id) {
        this.send({
            type: 'click',
            id: id
        });
    }

    sendInput(id, value) {
        this.send({
            type: 'input',
            id: id,
            value: value
        });
    }

    sendChange(id, value) {
        this.send({
            type: 'change',
            id: id,
            value: value
        });
    }

    send(msg) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(msg));
        }
    }
}

// Auto-initialize when DOM is ready
let webuiClient;

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initWebUI);
} else {
    initWebUI();
}

function initWebUI() {
    // Initialize client
    webuiClient = new WebUIClient();

    // Handle UI events
    document.addEventListener('ui-click', (e) => {
        webuiClient.sendClick(e.detail.id);
    });

    document.addEventListener('ui-input', (e) => {
        webuiClient.sendInput(e.detail.id, e.detail.value);
    });

    document.addEventListener('ui-change', (e) => {
        webuiClient.sendChange(e.detail.id, e.detail.value);
    });
}
