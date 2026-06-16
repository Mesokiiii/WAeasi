// @waeasi/sdk — Sign stage
//
// Produces an Ed25519 detached signature over the canonical bundle
// hash so the kernel's `crypto/verify.rs` allowlist gate accepts the
// component.
//
// Bundle hash = SHA-256(
//     LE32(version)
//   ‖ LE32(len(engineDigest))  ‖ engineDigest (raw 32B)
//   ‖ LE32(len(userDigest))    ‖ userDigest   (raw 32B)
//   ‖ LE32(len(manifestBytes)) ‖ manifestBytes
// )
//
// The version prefix prevents cross-version replay; engine + user
// digests bind both modules; manifest binds capabilities + resource
// limits.  Re-signing requires touching at least one of those four.
//
// Keys: Ed25519 seed (32B raw) loaded from a PEM-wrapped file
// (`-----BEGIN WAEASI ED25519 SEED-----`) or from `WAEASI_SIGN_KEY`
// environment variable.

import { readFile, writeFile, mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, resolve } from "node:path";
import { sign as ed25519Sign } from "node:crypto";

const MAGIC = Buffer.from("WAEASIv1", "ascii");
const VERSION = 1;

export interface SignInput {
    /** Output dir (where signature.ed25519 is written). */
    outDir: string;
    engineDigest: string | null;
    userDigest: string;
    manifestPath: string;
    /** Either inline 32-byte seed or path to a PEM seed file. */
    keySource:
        | { kind: "raw"; seed: Uint8Array }
        | { kind: "file"; path: string }
        | { kind: "env"; var: string };
}

export interface SignResult {
    signaturePath: string;
    bundleDigestHex: string;
    publicKeyHex: string;
    durationMs: number;
}

export async function sign(input: SignInput): Promise<SignResult> {
    const start = Date.now();
    const { seed, publicKey } = await loadSigningKey(input.keySource);

    const manifestBytes = await readFile(input.manifestPath);
    const bundleHash = computeBundleDigest(
        VERSION,
        input.engineDigest,
        input.userDigest,
        manifestBytes,
    );

    // Node's sign() accepts a raw seed via `KeyObject` in newer versions,
    // but to keep the dependency tree tight we use the createPrivateKey
    // path with a synthetic PKCS#8 wrapper.
    const { createPrivateKey } = await import("node:crypto");
    const pkcs8 = ed25519SeedToPkcs8(seed);
    const key = createPrivateKey({ key: pkcs8, format: "der", type: "pkcs8" });
    const sigBytes = ed25519Sign(null, bundleHash, key);

    const sigBlob = Buffer.concat([
        MAGIC,
        Buffer.from([VERSION]),
        Buffer.from([sigBytes.length]),
        sigBytes,
        Buffer.from([publicKey.length]),
        publicKey,
        bundleHash,
    ]);

    await mkdir(input.outDir, { recursive: true });
    const sigPath = resolve(input.outDir, "signature.ed25519");
    await writeFile(sigPath, sigBlob);

    return {
        signaturePath: sigPath,
        bundleDigestHex: bundleHash.toString("hex"),
        publicKeyHex: Buffer.from(publicKey).toString("hex"),
        durationMs: Date.now() - start,
    };
}

function computeBundleDigest(
    version: number,
    engineDigest: string | null,
    userDigest: string,
    manifestBytes: Buffer,
): Buffer {
    const h = createHash("sha256");
    h.update(le32(version));
    if (engineDigest) {
        const eb = Buffer.from(engineDigest, "hex");
        h.update(le32(eb.byteLength));
        h.update(eb);
    } else {
        h.update(le32(0));
    }
    const ub = Buffer.from(userDigest, "hex");
    h.update(le32(ub.byteLength));
    h.update(ub);
    h.update(le32(manifestBytes.byteLength));
    h.update(manifestBytes);
    return h.digest();
}

function le32(n: number): Buffer {
    const b = Buffer.alloc(4);
    b.writeUInt32LE(n, 0);
    return b;
}

async function loadSigningKey(
    src: SignInput["keySource"],
): Promise<{ seed: Uint8Array; publicKey: Uint8Array }> {
    let seed: Uint8Array;
    if (src.kind === "raw") seed = src.seed;
    else if (src.kind === "file") seed = await loadSeedFile(src.path);
    else seed = parseHexEnv(src.var);
    if (seed.byteLength !== 32) {
        throw new Error(`expected 32-byte ed25519 seed, got ${seed.byteLength}`);
    }
    const publicKey = await derivePublic(seed);
    return { seed, publicKey };
}

async function loadSeedFile(path: string): Promise<Uint8Array> {
    const text = await readFile(path, "utf8");
    const m = text.match(/-----BEGIN WAEASI ED25519 SEED-----\s+([A-Za-z0-9+/=\s]+?)-----END/);
    if (!m) throw new Error(`malformed key file: ${path}`);
    return new Uint8Array(Buffer.from(m[1].replace(/\s/g, ""), "base64"));
}

function parseHexEnv(name: string): Uint8Array {
    const v = process.env[name];
    if (!v) throw new Error(`env var ${name} unset`);
    return new Uint8Array(Buffer.from(v, "hex"));
}

async function derivePublic(seed: Uint8Array): Promise<Uint8Array> {
    const { createPrivateKey, createPublicKey } = await import("node:crypto");
    const pkcs8 = ed25519SeedToPkcs8(seed);
    const priv = createPrivateKey({ key: pkcs8, format: "der", type: "pkcs8" });
    const pub = createPublicKey(priv).export({ format: "der", type: "spki" });
    // SPKI: last 32 bytes are the raw public key.
    return new Uint8Array(pub.subarray(pub.byteLength - 32));
}

function ed25519SeedToPkcs8(seed: Uint8Array): Buffer {
    // PKCS#8 ASN.1 wrapper for Ed25519 raw seeds (RFC 8410).
    const prefix = Buffer.from([
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b,
        0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
    ]);
    return Buffer.concat([prefix, Buffer.from(seed)]);
}
