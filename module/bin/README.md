Build packaging places native artifacts under this directory:

- `arm64-v8a/puread-daemon`
- `arm64-v8a/puread-cli`
- `armeabi-v7a/puread-daemon`
- `armeabi-v7a/puread-cli`
- `x86_64/puread-daemon`
- `x86_64/puread-cli`
- `x86/puread-daemon`
- `x86/puread-cli`
- `riscv64/puread-daemon`
- `riscv64/puread-cli`

Local host fixture artifacts can be used to validate zip layout when Android
cross-compilation is unavailable. Those host fixtures are not Android-device
validated binaries.
