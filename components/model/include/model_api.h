#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    float confidence;
    int inference_time_ms;
    int label_index;
} model_result_t;

model_result_t model_classify(const float *features, int features_len);

#ifdef __cplusplus
}
#endif