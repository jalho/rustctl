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

fn credential_to_json(credential: wasm_bindgen::JsValue) -> serde_json::Value {
    use wasm_bindgen::JsCast;
    let js_val = credential
        .dyn_into::<web_sys::PublicKeyCredential>()
        .unwrap();
    if let Ok(json_str) = js_sys::JSON::stringify(&js_val) {
        let s: String = json_str.into();
        return serde_json::from_str(&s).unwrap_or(serde_json::Value::Null);
    }
    serde_json::Value::Null
}

/// 1. Get params for creating a passkey.
///
///    ```sh
///    curl -k -X POST https://rustctl.internal:8080/auth/sign-up/challenge | jq
///    ```
///
///    Response looks like this (`shared::SignUpResponse`):
///
///    ```json
///    {
///      "id": "574db7a8-5030-417d-b10a-93f1264a3e2b",
///      "ccr": {
///        "publicKey": {
///          "attestation": "none",
///          "authenticatorSelection": {
///            "requireResidentKey": false,
///            "residentKey": "discouraged",
///            "userVerification": "required"
///          },
///          "challenge": "-sj-3MpCIpYOdzwQRf8nIk4E17sHnjkW3ijYJThiKgQ",
///          "extensions": {
///            "credProps": true,
///            "credentialProtectionPolicy": "userVerificationRequired",
///            "enforceCredentialProtectionPolicy": false,
///            "uvm": true
///          },
///          "pubKeyCredParams": [
///            {
///              "alg": -7,
///              "type": "public-key"
///            },
///            {
///              "alg": -257,
///              "type": "public-key"
///            }
///          ],
///          "rp": {
///            "id": "rustctl.internal",
///            "name": "rustctl.internal"
///          },
///          "timeout": 300000,
///          "user": {
///            "displayName": "PLACEHOLDER2",
///            "id": "V023qFAwQX2xCpPxJko-Kw",
///            "name": "PLACEHOLDER1"
///          }
///        }
///      }
///    }
///    ```
///
/// 2. Make a passkey.
///
///    Call whatever web browser & platform APIs necessary with the parameters.
///
/// 3. Submit the (public part of the) passkey.
///
///    ```
///    POST /auth/sign-up/submit/{challenge_id}
///    ```
///
///    The `challenge_id` is the `id` in the response of step #1.
pub async fn handle_sign_up() {
    // 1. Get params for creating a passkey
    let resp: shared::SignUpResponse = gloo_net::http::Request::post(shared::SIGN_UP_CHALLENGE)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let pk = &resp.ccr["publicKey"];

    // 2. Decode Base64URL challenge using browser's atob
    // Base64URL uses '-' and '_' which atob doesn't like, so we swap them
    let challenge_raw = pk["challenge"].as_str().unwrap();
    let challenge_base64 = challenge_raw.replace('-', "+").replace('_', "/");
    let window = web_sys::window().unwrap();
    let decoded_str = window.atob(&challenge_base64).unwrap();
    let challenge_js = js_sys::Uint8Array::new_with_length(decoded_str.len() as u32);
    for (i, byte) in decoded_str.bytes().enumerate() {
        challenge_js.set_index(i as u32, byte);
    }

    // 3. Prepare RP and User Entities
    let rp_id = pk["rp"]["id"].as_str().unwrap();
    let rp_name = pk["rp"]["name"].as_str().unwrap();
    let rp_entity = web_sys::PublicKeyCredentialRpEntity::new(rp_id);
    rp_entity.set_name(rp_name);

    let user_id_str = pk["user"]["id"].as_str().unwrap();
    let user_id_js = js_sys::Uint8Array::from(user_id_str.as_bytes());
    let user_name = pk["user"]["name"].as_str().unwrap();
    let user_display = pk["user"]["displayName"].as_str().unwrap();
    let user_entity =
        web_sys::PublicKeyCredentialUserEntity::new(user_name, user_display, &user_id_js);

    // 4. Map pubKeyCredParams from JSON
    let params_array = js_sys::Array::new();
    if let Some(params) = pk["pubKeyCredParams"].as_array() {
        for p in params {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"type".into(), &p["type"].as_str().unwrap().into())
                .unwrap();
            js_sys::Reflect::set(&obj, &"alg".into(), &p["alg"].as_i64().unwrap().into()).unwrap();
            params_array.push(&obj);
        }
    }

    // 5. Assemble Options using document-specified constraints
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

    // 6. Request Credential
    let credentials = window.navigator().credentials();
    let create_options = web_sys::CredentialCreationOptions::new();
    create_options.set_public_key(&options);

    if let Ok(promise) = credentials.create_with_options(&create_options)
        && let Ok(credential) = wasm_bindgen_futures::JsFuture::from(promise).await
    {
        let credential_json = credential_to_json(credential);

        // 7. Submit to the challenge-specific URL
        let submit_url = shared::SIGN_UP_SUBMIT.replace("{challenge_id}", &resp.id.to_string());

        let _ = gloo_net::http::Request::post(&submit_url)
            .json(&credential_json)
            .unwrap()
            .send()
            .await;
    }
}

pub async fn handle_sign_in() {
    let resp: serde_json::Value = gloo_net::http::Request::post(shared::SIGN_IN_CHALLENGE)
        .send()
        .await
        .map_err(|e| {
            web_sys::console::error_1(&format!("failed to fetch login options: {:?}", e).into());
            e
        })
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .map_err(|e| {
            web_sys::console::error_1(
                &format!("failed to parse login options JSON: {:?}", e).into(),
            );
            e
        })
        .unwrap();

    let challenge_hex: &str = resp["challenge"].as_str().unwrap_or_else(|| {
        web_sys::console::error_1(&"challenge missing in response".into());
        panic!();
    });
    let challenge_js: js_sys::Uint8Array =
        js_sys::Uint8Array::new_with_length((challenge_hex.len() / 2) as u32);
    for i in 0..(challenge_hex.len() / 2) {
        let byte = u8::from_str_radix(&challenge_hex[i * 2..i * 2 + 2], 16).unwrap_or_else(|e| {
            web_sys::console::error_1(&format!("invalid hex byte: {:?}", e).into());
            panic!();
        });
        challenge_js.set_index(i as u32, byte);
    }

    let options: web_sys::PublicKeyCredentialRequestOptions =
        web_sys::PublicKeyCredentialRequestOptions::new(&challenge_js);
    options.set_timeout(60000);

    if let Some(rp_id) = resp["rpId"].as_str() {
        options.set_rp_id(rp_id);
    }

    let window: web_sys::Window = web_sys::window().unwrap_or_else(|| {
        web_sys::console::error_1(&"no window found".into());
        panic!();
    });
    let credentials: web_sys::CredentialsContainer = window.navigator().credentials();

    let request_options: web_sys::CredentialRequestOptions =
        web_sys::CredentialRequestOptions::new();
    request_options.set_public_key(&options);

    let promise: js_sys::Promise = credentials
        .get_with_options(&request_options)
        .map_err(|e| {
            web_sys::console::error_1(&format!("failed to get credentials: {:?}", e).into());
            e
        })
        .unwrap();

    let credential: wasm_bindgen::JsValue = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| {
            web_sys::console::error_1(&format!("platform error: {:?}", e).into());
            e
        })
        .unwrap();

    let credential_json = credential_to_json(credential);
    let _ = gloo_net::http::Request::post(shared::SIGN_IN_SUBMIT)
        .json(&credential_json)
        .unwrap()
        .send()
        .await;
}
