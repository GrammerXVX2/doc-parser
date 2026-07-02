pub mod backends;
pub mod books;
pub mod config;
pub mod domain;
pub mod layout;
pub mod legal;
pub mod ocr;
pub mod router;
pub mod slow_path;
pub mod structured;
pub mod tables;

pub use books::{BookExtraction, detect_historical_orthography, extract_book_mvp};
pub use config::{
    ModelBackendConfig, ModelProfileConfig, ModelStackConfig, ModelStackRoot,
    load_model_stack_config, load_model_stack_config_or_default,
};
pub use domain::{DocumentDomain, DomainProfile, detect_document_domain};
pub use legal::{LegalExtraction, extract_legal_mvp, legal_required_fields_present};
pub use router::{ModelRoutingDecision, parse_domain_override, route_models};
pub use slow_path::{SlowPathDecision, decide_slow_path};
pub use layout::{DoclingLayoutHttpBackend, SuryaLayoutHttpBackend};
pub use ocr::{PaddleOcrV6HttpBackend, SuryaOcrHttpBackend};
pub use structured::DoclingStructuredParseHttpBackend;
pub use tables::{DoclingTableFormerHttpBackend, SuryaTableHttpBackend};
