use dioxus::dioxus_core;
use dioxus::document;
use dioxus::prelude::asset;
use dioxus::prelude::dioxus_elements;
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::manganis;

#[dioxus::prelude::component]
pub fn App() -> dioxus::core::Element {
    let state = dioxus::hooks::use_context_provider(crate::state::GlobalState::init);

    dioxus::prelude::use_future(move || async move {
        crate::state::GlobalState::connect(state).await;
    });

    dioxus::prelude::rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        crate::layout::debug_viewer::DebugViewer {}
        PasskeyComponent {}
    }
}

#[dioxus::prelude::component]
pub fn PasskeyComponent() -> dioxus::core::Element {
    dioxus::prelude::rsx! {
        div { class: "flex gap-2",
            button {
                onclick: |_| {
                    dioxus::prelude::spawn(handle_sign_up());
                },
                "Sign Up: Create Passkey"
            }
            button {
                onclick: |_| {
                    dioxus::prelude::spawn(handle_sign_in());
                },
                "Sign In: Use Existing Passkey"
            }
        }
    }
}

fn log_error_and_panic(msg: &str) -> ! {
    web_sys::console::error_1(&msg.into());
    panic!("{}", msg);
}

fn credential_to_json(credential: wasm_bindgen::JsValue) -> serde_json::Value {
    use wasm_bindgen::JsCast;
    let js_val = credential
        .dyn_into::<web_sys::PublicKeyCredential>()
        .unwrap_or_else(|_| log_error_and_panic("Failed to cast JsValue to PublicKeyCredential"));

    let json_str: String = js_sys::JSON::stringify(&js_val)
        .unwrap_or_else(|_| log_error_and_panic("js_sys::JSON::stringify failed"))
        .into();

    serde_json::from_str(&json_str).unwrap_or_else(|e| {
        log_error_and_panic(&format!("Failed to parse credential JSON: {:?}", e))
    })
}

pub async fn handle_sign_up() {
    let resp: shared::SignUpResponse = gloo_net::http::Request::post(shared::SIGN_UP_CHALLENGE)
        .send()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("POST challenge failed: {:?}", e)))
        .json()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("Parse SignUpResponse failed: {:?}", e)));

    let pk = &resp.ccr["publicKey"];

    // Decode Challenge
    let challenge_raw = pk["challenge"]
        .as_str()
        .unwrap_or_else(|| log_error_and_panic("Challenge missing"));
    let challenge_base64 = challenge_raw.replace('-', "+").replace('_', "/");
    let window = web_sys::window().unwrap_or_else(|| log_error_and_panic("No window"));
    let decoded_str = window
        .atob(&challenge_base64)
        .unwrap_or_else(|_| log_error_and_panic("atob failed"));
    let challenge_js = js_sys::Uint8Array::new_with_length(decoded_str.len() as u32);
    for (i, byte) in decoded_str.bytes().enumerate() {
        challenge_js.set_index(i as u32, byte);
    }

    // Entities
    let rp_entity = web_sys::PublicKeyCredentialRpEntity::new(
        pk["rp"]["id"]
            .as_str()
            .unwrap_or_else(|| log_error_and_panic("rp.id missing")),
    );
    rp_entity.set_name(
        pk["rp"]["name"]
            .as_str()
            .unwrap_or_else(|| log_error_and_panic("rp.name missing")),
    );

    let user_id_js = js_sys::Uint8Array::from(
        pk["user"]["id"]
            .as_str()
            .unwrap_or_else(|| log_error_and_panic("user.id missing"))
            .as_bytes(),
    );
    let user_entity = web_sys::PublicKeyCredentialUserEntity::new(
        pk["user"]["name"]
            .as_str()
            .unwrap_or_else(|| log_error_and_panic("user.name missing")),
        pk["user"]["displayName"]
            .as_str()
            .unwrap_or_else(|| log_error_and_panic("user.displayName missing")),
        &user_id_js,
    );

    // Params: Fix for "Cannot convert a BigInt value to a number"
    let params_array = js_sys::Array::new();
    let params_json = pk["pubKeyCredParams"]
        .as_array()
        .unwrap_or_else(|| log_error_and_panic("pubKeyCredParams missing"));
    for p in params_json {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"type".into(), &p["type"].as_str().unwrap().into()).unwrap();
        // FIX: Cast i64 to i32 before converting to JsValue to avoid BigInt issues
        let alg_i32 = p["alg"].as_i64().unwrap() as i32;
        js_sys::Reflect::set(&obj, &"alg".into(), &alg_i32.into()).unwrap();
        params_array.push(&obj);
    }

    let options = web_sys::PublicKeyCredentialCreationOptions::new(
        &challenge_js,
        &params_array,
        &rp_entity,
        &user_entity,
    );
    let auth_selection = web_sys::AuthenticatorSelectionCriteria::new();
    auth_selection.set_require_resident_key(
        pk["authenticatorSelection"]["requireResidentKey"]
            .as_bool()
            .unwrap(),
    );
    auth_selection.set_user_verification(web_sys::UserVerificationRequirement::Required);
    options.set_authenticator_selection(&auth_selection);
    options.set_timeout(pk["timeout"].as_f64().unwrap() as u32);

    let create_options = web_sys::CredentialCreationOptions::new();
    create_options.set_public_key(&options);

    let promise = window
        .navigator()
        .credentials()
        .create_with_options(&create_options)
        .unwrap_or_else(|e| log_error_and_panic(&format!("Credentials.create failed: {:?}", e)));

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("Platform error/User cancel: {:?}", e)));

    let submit_url = shared::SIGN_UP_SUBMIT.replace("{challenge_id}", &resp.id.to_string());
    gloo_net::http::Request::post(&submit_url)
        .json(&credential_to_json(result))
        .unwrap_or_else(|e| log_error_and_panic(&format!("Submit serialization failed: {:?}", e)))
        .send()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("Submit POST failed: {:?}", e)));
}

pub async fn handle_sign_in() {
    let resp: serde_json::Value = gloo_net::http::Request::post(shared::SIGN_IN_CHALLENGE)
        .send()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("SignIn challenge failed: {:?}", e)))
        .json()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("SignIn JSON parse failed: {:?}", e)));

    // Handle Hex decoding (keeping your logic from app.rs)
    let challenge_hex = resp["challenge"]
        .as_str()
        .unwrap_or_else(|| log_error_and_panic("SignIn challenge missing"));
    let challenge_js = js_sys::Uint8Array::new_with_length((challenge_hex.len() / 2) as u32);
    for i in 0..(challenge_hex.len() / 2) {
        let byte = u8::from_str_radix(&challenge_hex[i * 2..i * 2 + 2], 16)
            .unwrap_or_else(|_| log_error_and_panic("Invalid hex in challenge"));
        challenge_js.set_index(i as u32, byte);
    }

    let options = web_sys::PublicKeyCredentialRequestOptions::new(&challenge_js);
    options.set_timeout(60000);
    if let Some(rp_id) = resp["rpId"].as_str() {
        options.set_rp_id(rp_id);
    }

    let window = web_sys::window().unwrap_or_else(|| log_error_and_panic("No window"));
    let req_options = web_sys::CredentialRequestOptions::new();
    req_options.set_public_key(&options);

    let promise = window
        .navigator()
        .credentials()
        .get_with_options(&req_options)
        .unwrap_or_else(|e| log_error_and_panic(&format!("Credentials.get failed: {:?}", e)));

    let credential = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("SignIn platform error: {:?}", e)));

    gloo_net::http::Request::post(shared::SIGN_IN_SUBMIT)
        .json(&credential_to_json(credential))
        .unwrap_or_else(|e| log_error_and_panic(&format!("Submit serialization failed: {:?}", e)))
        .send()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("Submit POST failed: {:?}", e)));
}
