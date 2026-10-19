#[repr(C)]
pub struct ModelResult {
    pub confidence: f32,
    pub inference_time_ms: i32,
    pub label_index: i32,
}

extern "C" {
    fn model_classify(features: *const f32, features_len: i32) -> ModelResult;
}

pub fn classify(features: &[f32]) -> ModelResult {
    unsafe { model_classify(features.as_ptr(), features.len() as i32) }
}