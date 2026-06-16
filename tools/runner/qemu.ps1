# PowerShell wrapper around the runner so cargo's `[target] runner = ...`
# directive can launch QEMU on Windows.
param([string]$Kernel)
cargo run -p runner --release -- $Kernel
