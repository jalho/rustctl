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
            let resp: serde_json::Value = gloo_net::http::Request::post("/auth/init")
                .send()
                .await
                .expect("Failed to fetch auth options")
                .json::<serde_json::Value>()
                .await
                .expect("Failed to parse JSON");

            // decode hex
            let challenge_hex: &str = resp["challenge"].as_str().unwrap();
            let challenge_js: js_sys::Uint8Array =
                js_sys::Uint8Array::new_with_length((challenge_hex.len() / 2) as u32);
            for i in 0..(challenge_hex.len() / 2) {
                let byte = u8::from_str_radix(&challenge_hex[i * 2..i * 2 + 2], 16).unwrap();
                challenge_js.set_index(i as u32, byte);
            }

            // RP entity
            let rp_entity: web_sys::PublicKeyCredentialRpEntity =
                web_sys::PublicKeyCredentialRpEntity::new(resp["rp"]["id"].as_str().unwrap());
            rp_entity.set_name(resp["rp"]["name"].as_str().unwrap());

            // user entity
            let user_id_str: &str = resp["user"]["id"].as_str().unwrap();
            let user_id_js: js_sys::Uint8Array = js_sys::Uint8Array::from(user_id_str.as_bytes());
            let user_entity: web_sys::PublicKeyCredentialUserEntity =
                web_sys::PublicKeyCredentialUserEntity::new(
                    resp["user"]["name"].as_str().unwrap(),
                    resp["user"]["displayName"].as_str().unwrap(),
                    &user_id_js,
                );

            // parameters
            let param: js_sys::Object = js_sys::Object::new();
            js_sys::Reflect::set(&param, &"type".into(), &"public-key".into()).unwrap();
            js_sys::Reflect::set(&param, &"alg".into(), &(-7).into()).unwrap();
            let params_array: js_sys::Array = js_sys::Array::of1(&param);

            // creation options
            let options: web_sys::PublicKeyCredentialCreationOptions =
                web_sys::PublicKeyCredentialCreationOptions::new(
                    &challenge_js,
                    &params_array,
                    &rp_entity,
                    &user_entity,
                );
            options.set_timeout(60000); // TODO: Use timeout value specified in the response payload?

            // trigger browser API
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

            web_sys::console::log_1(&credential);
        });
    };

    dioxus::prelude::rsx! {
        button { onclick: handle_register, "Create Passkey" }
    }
}
