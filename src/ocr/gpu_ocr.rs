use crate::ml::ExecutionProviderKind;

#[derive(Debug, Clone, Copy, Default)]
pub struct GpuOcrSupport;

impl GpuOcrSupport {
    pub fn validate_provider(provider: ExecutionProviderKind) -> anyhow::Result<()> {
        provider.ensure_available()
    }
}
