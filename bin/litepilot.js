#!/usr/bin/env node
const { execFileSync } = require('child_process');
const path = require('path');

const platform = process.platform;
const arch = process.arch;

const platformMap = {
  'darwin-arm64': 'litepilot-darwin-arm64',
  'darwin-x64': 'litepilot-darwin-x64',
  'linux-x64': 'litepilot-linux-x64',
  'linux-arm64': 'litepilot-linux-arm64',
};

const binaryName = platformMap[`${platform}-${arch}`];
if (!binaryName) {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  console.error('Supported: darwin-arm64, darwin-x64, linux-x64, linux-arm64');
  process.exit(1);
}

const binaryPath = path.join(__dirname, binaryName);
const args = process.argv.slice(2);

try {
  execFileSync(binaryPath, args, { stdio: 'inherit' });
} catch (e) {
  process.exit(e.status || 1);
}
