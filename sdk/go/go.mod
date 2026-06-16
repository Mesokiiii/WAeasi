module waeasi.dev/sdk

go 1.23

// The Go SDK targets `tinygo build -target=wasip2` (TinyGo ≥ 0.34).
// We use no external runtime dependencies — the binding code below is
// hand-written against the wasi:http@0.2.0 + waeasi:obs WIT files,
// regenerated via `wit-bindgen-go` on release.
