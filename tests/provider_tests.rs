use document_parser::ml::ExecutionProviderKind;

#[test]
fn cpu_provider_available() {
    assert!(ExecutionProviderKind::Cpu.ensure_available().is_ok());
}

#[test]
fn cuda_provider_unavailable_handled() {
    let err = ExecutionProviderKind::Cuda
        .ensure_available()
        .unwrap_err()
        .to_string();
    assert!(err.contains("CUDA_PROVIDER_UNAVAILABLE"));
}

#[test]
fn tensorrt_provider_unavailable_handled() {
    let err = ExecutionProviderKind::TensorRt
        .ensure_available()
        .unwrap_err()
        .to_string();
    assert!(err.contains("TENSORRT_PROVIDER_UNAVAILABLE"));
}

#[test]
fn triton_provider_unavailable_handled() {
    let err = ExecutionProviderKind::Triton
        .ensure_available()
        .unwrap_err()
        .to_string();
    assert!(err.contains("TRITON_UNAVAILABLE"));
}
