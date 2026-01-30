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

#[dioxus::prelude::component]
pub fn PasskeyComponent() -> dioxus::core::Element {
    let handle_register = move |_| {
        dioxus::prelude::spawn(async move {
            // 1. Fetch options from server
            let resp: serde_json::Value = gloo_net::http::Request::get("/auth/init")
                .send()
                .await
                .expect("Failed to fetch auth options")
                .json::<serde_json::Value>()
                .await
                .expect("Failed to parse JSON");

            // 2. Decode Challenge (Base64 -> Uint8Array)
            let challenge_str: &str = resp["challenge"].as_str().unwrap();
            let challenge_bytes: Vec<u8> =
                <base64::engine::GeneralPurpose as base64::Engine>::decode(
                    &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                    challenge_str,
                )
                .expect("Invalid base64 challenge");
            let challenge_js: js_sys::Uint8Array = js_sys::Uint8Array::from(&challenge_bytes[..]);

            // 3. Prepare RP Entity
            let rp_entity: web_sys::PublicKeyCredentialRpEntity =
                web_sys::PublicKeyCredentialRpEntity::new(resp["rp"]["id"].as_str().unwrap());
            rp_entity.set_name(resp["rp"]["name"].as_str().unwrap());

            // 4. Prepare User Entity
            let user_id_str: &str = resp["user"]["id"].as_str().unwrap();
            let user_id_js: js_sys::Uint8Array = js_sys::Uint8Array::from(user_id_str.as_bytes());
            let user_entity: web_sys::PublicKeyCredentialUserEntity =
                web_sys::PublicKeyCredentialUserEntity::new(
                    resp["user"]["name"].as_str().unwrap(),
                    resp["user"]["displayName"].as_str().unwrap(),
                    &user_id_js,
                );

            // 5. Prepare Parameters (Algorithm: ES256)
            let param: js_sys::Object = js_sys::Object::new();
            js_sys::Reflect::set(&param, &"type".into(), &"public-key".into()).unwrap();
            js_sys::Reflect::set(&param, &"alg".into(), &(-7).into()).unwrap();
            let params_array: js_sys::Array = js_sys::Array::of1(&param);

            // 6. Create Creation Options
            let options: web_sys::PublicKeyCredentialCreationOptions =
                web_sys::PublicKeyCredentialCreationOptions::new(
                    challenge_js.as_ref(),
                    params_array.as_ref(),
                    &rp_entity,
                    &user_entity,
                );
            options.set_timeout(60000); // TODO: Use the value received from the server?

            // 7. Trigger Browser Hardware API
            let window: web_sys::Window = web_sys::window().expect("No window");
            let credentials: web_sys::CredentialsContainer = window.navigator().credentials();

            let create_options: web_sys::CredentialCreationOptions =
                web_sys::CredentialCreationOptions::new();
            create_options.set_public_key(&options);

            let promise: js_sys::Promise = credentials
                .create_with_options(&create_options)
                .expect("Failed to create promise");
            let credential: wasm_bindgen::JsValue = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .expect("Hardware/User Error");

            // 8. Log success
            web_sys::console::log_1(&credential);
        });
    };

    dioxus::prelude::rsx! {
        button {
            onclick: handle_register,
            "Create Passkey"
        }
    }
}
