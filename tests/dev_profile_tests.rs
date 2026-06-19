use std::path::Path;

use document_parser::config::profiles::ServiceProfile;

#[test]
fn dev_team_profile_loads_and_matches_constraints() {
    let profile_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/profiles/dev_team.jsonc");
    let profile = ServiceProfile::from_path(&profile_path)
        .expect("dev_team profile should load");

    assert_eq!(profile.storage.metadata_backend, "local_json");
    assert!(profile.security.max_file_size_mb <= 100);
    assert_eq!(profile.security.max_pages_per_document, 300);
    assert!(!profile.security.allow_external_converters);
    assert_eq!(profile.service.locale, "ru");
}
