use dioxus::dioxus_core;
use dioxus::document;
use dioxus::prelude::asset;
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::manganis;

#[dioxus::prelude::component]
pub fn App() -> dioxus::core::Element {
    /*
     * Initialize a global state, and hook it to a WebSocket.
     */
    dioxus::hooks::use_context_provider(crate::state::GlobalState::init);
    dioxus::prelude::use_future(crate::state::GlobalState::keep_connected);

    /*
     * Render stuff from the global state.
     */
    dioxus::prelude::rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        crate::layout::debug_viewer::DebugViewer {}
        PasskeyComponent {}
    }
}

use dioxus::prelude::dioxus_elements;
use dioxus::prelude::rsx;

#[dioxus::prelude::component]
pub fn PasskeyComponent() -> dioxus_core::Element {
    let handle_auth = move |_| {
        dioxus_core::spawn(async move {
            let window: web_sys::Window = web_sys::window().unwrap();
            let navigator: web_sys::Navigator = window.navigator();
            let credentials: web_sys::CredentialsContainer = navigator.credentials();

            let challenge: [u8; 32] = [0u8; 32];
            let challenge_js: js_sys::Uint8Array = js_sys::Uint8Array::from(&challenge[..]);

            let pk_options: web_sys::PublicKeyCredentialRequestOptions =
                web_sys::PublicKeyCredentialRequestOptions::new(challenge_js.as_ref());

            let options: web_sys::CredentialRequestOptions =
                web_sys::CredentialRequestOptions::new();
            options.set_public_key(&pk_options);

            let promise: js_sys::Promise = credentials.get_with_options(&options).unwrap();
            let future: wasm_bindgen_futures::JsFuture =
                wasm_bindgen_futures::JsFuture::from(promise);

            match future.await {
                Ok(creds) => web_sys::console::log_1(&creds),
                Err(err) => web_sys::console::log_1(&err),
            }
        });
    };

    rsx! {
        button { onclick: handle_auth, "Login with Passkey" }
    }
}
