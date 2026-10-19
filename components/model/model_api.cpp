#include "model_api.h"

extern "C" model_result_t model_classify(const float *features, int features_len) {
    model_result_t result;
    // STUB SEMENTARA — nanti diganti pemanggilan run_classifier() dari SDK Edge Impulse asli
    result.confidence = 88.0f;
    result.inference_time_ms = 12;
    result.label_index = 0;
    return result;
}