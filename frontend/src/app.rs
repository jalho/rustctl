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

    let challenge_raw = pk["challenge"]
        .as_str()
        .unwrap_or_else(|| log_error_and_panic("Challenge missing"));

    let challenge_bytes =
        base64::Engine::decode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, challenge_raw)
            .unwrap_or_else(|e| {
                log_error_and_panic(&format!("Base64URL challenge decode failed: {:?}", e))
            });

    let challenge_js = js_sys::Uint8Array::from(&challenge_bytes[..]);

    let user_id_raw = pk["user"]["id"]
        .as_str()
        .unwrap_or_else(|| log_error_and_panic("User ID missing"));

    let user_id_bytes =
        base64::Engine::decode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, user_id_raw)
            .unwrap_or_else(|_| user_id_raw.as_bytes().to_vec());

    let user_id_js = js_sys::Uint8Array::from(&user_id_bytes[..]);

    let rp_entity = web_sys::PublicKeyCredentialRpEntity::new(pk["rp"]["id"].as_str().unwrap());
    rp_entity.set_name(pk["rp"]["name"].as_str().unwrap());

    let user_entity = web_sys::PublicKeyCredentialUserEntity::new(
        pk["user"]["name"].as_str().unwrap(),
        pk["user"]["displayName"].as_str().unwrap(),
        &user_id_js,
    );

    let params_array = js_sys::Array::new();
    if let Some(params_json) = pk["pubKeyCredParams"].as_array() {
        for p in params_json {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"type".into(), &p["type"].as_str().unwrap().into())
                .unwrap();
            let alg = p["alg"].as_i64().unwrap() as i32;
            js_sys::Reflect::set(&obj, &"alg".into(), &alg.into()).unwrap();
            params_array.push(&obj);
        }
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
            .unwrap_or(false),
    );
    auth_selection.set_user_verification(web_sys::UserVerificationRequirement::Required);
    options.set_authenticator_selection(&auth_selection);

    if let Some(timeout) = pk["timeout"].as_f64() {
        options.set_timeout(timeout as u32);
    }

    let create_options = web_sys::CredentialCreationOptions::new();
    create_options.set_public_key(&options);

    let window = web_sys::window().unwrap();
    let promise = window
        .navigator()
        .credentials()
        .create_with_options(&create_options)
        .unwrap();
    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("Passkey creation failed: {:?}", e)));

    let submit_url = shared::SIGN_UP_SUBMIT.replace("{challenge_id}", &resp.id.to_string());
    gloo_net::http::Request::post(&submit_url)
        .json(&credential_to_json(result))
        .unwrap()
        .send()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("Submission failed: {:?}", e)));
}

pub async fn handle_sign_in() {
    let resp: serde_json::Value = gloo_net::http::Request::post(shared::SIGN_IN_CHALLENGE)
        .send()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("POST challenge failed: {:?}", e)))
        .json()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("Parse SignInResponse failed: {:?}", e)));

    let challenge_raw = resp["publicKey"]["challenge"]
        .as_str()
        .unwrap_or_else(|| log_error_and_panic("SignIn challenge missing"));

    let challenge_bytes =
        base64::Engine::decode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, challenge_raw)
            .unwrap_or_else(|e| {
                log_error_and_panic(&format!("Base64URL challenge decode failed: {:?}", e))
            });

    let challenge_js = js_sys::Uint8Array::from(&challenge_bytes[..]);

    let options = web_sys::PublicKeyCredentialRequestOptions::new(&challenge_js);

    if let Some(timeout) = resp["publicKey"]["timeout"].as_f64() {
        options.set_timeout(timeout as u32);
    }

    if let Some(rp_id) = resp["publicKey"]["rpId"].as_str() {
        options.set_rp_id(rp_id);
    }

    if let Some(allow_credentials) = resp["publicKey"]["allowCredentials"].as_array() {
        let allow_creds_array = js_sys::Array::new();
        for cred in allow_credentials {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"type".into(), &cred["type"].as_str().unwrap().into())
                .unwrap();

            let id_raw = cred["id"].as_str().unwrap();
            let id_bytes = base64::Engine::decode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, id_raw)
                .unwrap_or_else(|_| id_raw.as_bytes().to_vec());
            let id_js = js_sys::Uint8Array::from(&id_bytes[..]);
            js_sys::Reflect::set(&obj, &"id".into(), &id_js.into()).unwrap();

            allow_creds_array.push(&obj);
        }
        options.set_allow_credentials(&allow_creds_array);
    }

    let window = web_sys::window().unwrap();
    let req_options = web_sys::CredentialRequestOptions::new();
    req_options.set_public_key(&options);

    let promise = window
        .navigator()
        .credentials()
        .get_with_options(&req_options)
        .unwrap();

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("SignIn platform error: {:?}", e)));

    gloo_net::http::Request::post(shared::SIGN_IN_SUBMIT)
        .json(&credential_to_json(result))
        .unwrap()
        .send()
        .await
        .unwrap_or_else(|e| log_error_and_panic(&format!("SignIn submission failed: {:?}", e)));
}
