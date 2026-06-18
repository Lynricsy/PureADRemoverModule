Future T25 packaging should place native artifacts under this directory:

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

T22 intentionally ships no native binaries. Lifecycle scripts report missing
binaries as `missing_binary` or `template_only` instead of treating this as a
successful daemon launch.
