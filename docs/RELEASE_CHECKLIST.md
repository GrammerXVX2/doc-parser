# Release Checklist

1. `cargo check`
2. `cargo test`
3. `scripts/run_regression.sh`
4. `scripts/run_benchmarks.sh`
5. `scripts/smoke_test.sh`
6. Docker build (`docker/Dockerfile`)
7. Security hardening tests
8. Актуализировать документацию
9. `cargo run -- doctor`

При падении любого пункта релиз блокируется.
