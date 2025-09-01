use tinysearch::TinySearch;
use wasm_bindgen::prelude::*;

// Compile-time loaded index bytes
static TINYSEARCH_INDEX_BYTES: &[u8] = include_bytes!("index");

// One-time deserialized index
// static TINYSEARCH_INDEX: LazyLock<TinySearchIndex> = LazyLock::new(|| {
//     let bob = TinySearch::new()
//         .load_index_from_bytes(TINYSEARCH_INDEX_BYTES)
//         .expect("Failed to load TinySearch index");
// });

#[wasm_bindgen]
pub fn tinysearch_search(query: &str) -> JsValue {
    let index = TinySearch::new()
        .load_index_from_bytes(TINYSEARCH_INDEX_BYTES)
        .expect("Failed to load TinySearch index");

    let search = TinySearch::new().search(&index, query, 5);

    let results: Vec<(String, String)> = search
        .into_iter()
        .map(|(title, url, _)| (title.to_string(), url.to_string()))
        .collect();

    serde_wasm_bindgen::to_value(&results).unwrap()
}
