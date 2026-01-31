const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const isRelease = process.env.RELEASE === "true";
const root = path.resolve(__dirname, "../../");
const serverDir = path.join(root, "crates", "server");
const extensionBinDir = path.join(__dirname, "../server/bin");

console.log("🔨 Building Rust server...");

execSync(
    isRelease ? "cargo build --release" : "cargo build", {
    cwd: serverDir,
    stdio: "inherit",
});

const binaryName = process.platform === "win32"
    ? "tine_server.exe"
    : "tine_server";

const builtBinary = path.join(
    root,
    "target",
    isRelease ? "release" : "debug",
    binaryName
);

// Ensure destination folder exists
fs.mkdirSync(extensionBinDir, { recursive: true });

// Copy binary
const destBinary = path.join(extensionBinDir, binaryName);
fs.copyFileSync(builtBinary, destBinary);

console.log("✅ Server built and copied to extension.");
