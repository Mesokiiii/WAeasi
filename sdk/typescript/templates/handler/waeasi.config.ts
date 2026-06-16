import type { WaeasiConfig } from "@waeasi/sdk/build";

export default {
    entry:    "src/index.ts",
    outDir:   "dist",
    witPath:  "../../wit",         // adjust if you vendor wit/ locally
    keyEnv:   "WAEASI_SIGN_KEY",   // 32-byte hex seed
    manifest: {
        world:           "handler",
        rights:          ["CLOCK_MONO", "RANDOM_SEC"],
        cpuShares:       100,
        memoryPagesMax:  256,        // 16 MiB linear memory ceiling
        linearMemMax:    "16 MiB",
    },
} satisfies WaeasiConfig;
