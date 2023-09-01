#[cfg(target_arch = "wasm32")]
use futures::stream::TryStreamExt;
#[cfg(target_arch = "wasm32")]
use kcl_language_server::server::{get_completions_from_stdlib, Backend};
#[cfg(target_arch = "wasm32")]
use tower_lsp::{LspService, Server};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::*, JsCast};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct ServerConfig {
    into_server: js_sys::AsyncIterator,
    from_server: web_sys::WritableStream,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl ServerConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(into_server: js_sys::AsyncIterator, from_server: web_sys::WritableStream) -> Self {
        Self {
            into_server,
            from_server,
        }
    }
}

/// Run the `kcl` lsp server.
//
// NOTE: we don't use web_sys::ReadableStream for input here because on the
// browser side we need to use a ReadableByteStreamController to construct it
// and so far only Chromium-based browsers support that functionality.

// NOTE: input needs to be an AsyncIterator<Uint8Array, never, void> specifically
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn run(config: ServerConfig) -> Result<(), JsValue> {
    let ServerConfig {
        into_server,
        from_server,
    } = config;

    let stdlib = kcl_lib::std::StdLib::new();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        stdlib_completions: get_completions_from_stdlib(&stdlib),
        token_map: Default::default(),
        ast_map: Default::default(),
    });

    let input = wasm_bindgen_futures::stream::JsStream::from(into_server);
    let input = input
        .map_ok(|value| {
            value
                .dyn_into::<js_sys::Uint8Array>()
                .expect("could not cast stream item to Uint8Array")
                .to_vec()
        })
        .map_err(|_err| std::io::Error::from(std::io::ErrorKind::Other))
        .into_async_read();

    let output = JsCast::unchecked_into::<wasm_streams::writable::sys::WritableStream>(from_server);
    let output = wasm_streams::WritableStream::from_raw(output);
    let output = output.try_into_async_write().map_err(|err| err.0)?;

    Server::new(input, output, socket).serve(service).await;

    Ok(())
}
