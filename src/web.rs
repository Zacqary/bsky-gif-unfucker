use wasm_bindgen::prelude::*;

use crate::gif_unfuck;

#[wasm_bindgen]
pub struct Processed(gif_unfuck::Processed);

#[wasm_bindgen]
impl Processed {
    #[wasm_bindgen(getter, js_name = originalSecs)]
    pub fn original_secs(&self) -> f64 {
        self.0.original_secs
    }

    #[wasm_bindgen(getter, js_name = unfuckedSecs)]
    pub fn unfucked_secs(&self) -> Option<f64> {
        self.0.unfucked.as_ref().map(|unfucked| unfucked.secs)
    }

    #[wasm_bindgen(getter, js_name = unfuckedBytes)]
    pub fn unfucked_bytes(&self) -> Option<Vec<u8>> {
        self.0
            .unfucked
            .as_ref()
            .map(|unfucked| unfucked.bytes.clone())
    }
}

#[wasm_bindgen]
pub fn process(input: &[u8]) -> Result<Processed, JsError> {
    gif_unfuck::process(input)
        .map(Processed)
        .map_err(|error| JsError::new(&error.to_string()))
}
