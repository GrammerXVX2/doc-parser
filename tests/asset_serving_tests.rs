use std::fs;
use std::path::PathBuf;

use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use tower::ServiceExt;

#[tokio::test]
async fn asset_retrieval_and_missing_asset_behavior() {
    let mut profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).expect("profile");
    profile.storage.output_dir = "target/asset_serving_tests_output".to_string();

    let document_id = "doc_asset_test";
    let output_dir = PathBuf::from(&profile.storage.output_dir).join(document_id);
    fs::create_dir_all(output_dir.join("assets/images")).unwrap();
    fs::write(output_dir.join("assets/images/file.bin"), b"asset").unwrap();

    let model = serde_json::json!({
        "assets": [
            {
                "asset_id": "asset_1",
                "type": "embedded_image",
                "path": "assets/images/file.bin",
                "mime_type": "application/octet-stream"
            }
        ]
    });
    fs::write(output_dir.join("model.json"), serde_json::to_vec_pretty(&model).unwrap()).unwrap();

    let app = build_app(build_state(profile));

    let ok = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/v1/documents/{}/assets/asset_1", document_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), axum::http::StatusCode::OK);

    let missing = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/v1/documents/{}/assets/asset_404", document_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), axum::http::StatusCode::NOT_FOUND);
}
