import React from 'react';
import {render} from 'ink';

import {App} from './App.js';
import type {FrontendConfig} from './types.js';

const rawReturnSubmit = process.env.OPENHARNESS_FRONTEND_RAW_RETURN === '1';
const scriptedSteps = (() => {
	const raw = process.env.OPENHARNESS_FRONTEND_SCRIPT;
	if (!raw) {
		return [] as string[];
	}
	try {
		const parsed = JSON.parse(raw);
		return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === 'string') : [];
	} catch {
		return [];
	}
})();

// Parse config with error handling
const isWindows = process.platform === 'win32';
const binaryPath = isWindows ? '../target/debug/rust_harness.exe' : '../target/debug/rust_harness';
let config: FrontendConfig = {backend_command: [binaryPath, '--stdio-backend']};
try {
	const configStr = process.env.OPENHARNESS_FRONTEND_CONFIG ?? '{}';
	const parsed = JSON.parse(configStr) as Partial<FrontendConfig>;
	config = {
		backend_command: parsed.backend_command ?? [binaryPath, '--stdio-backend'],
		initial_prompt: parsed.initial_prompt,
	};
} catch (e) {
	console.error('Failed to parse OPENHARNESS_FRONTEND_CONFIG:', e);
	console.error('Using default config:', config);
}

// Check if stdin supports TTY/raw mode
const isTtySupported = process.stdin.isTTY === true;

if (!isTtySupported) {
	console.error('WARNING: stdin is not a TTY. Interactive input will not work.');
	console.error('This is expected when running inside Claude Code or with redirected stdin.');
	console.error('For full interactive support, run this in a standalone terminal window.');
	console.error('');
}

try {
	render(<App config={config} />, {
		patchConsole: true,
		exitOnCtrlC: false,
	});
} catch (error) {
	const errorMessage = error instanceof Error ? error.message : String(error);
	if (errorMessage.includes('Raw mode is not supported')) {
		console.error('');
		console.error('ERROR: Raw mode is not supported in this environment.');
		console.error('');
		console.error('To fix this, run the frontend in a standalone terminal:');
		console.error('');
		console.error('  1. Open a new terminal window (Windows Terminal, CMD, or PowerShell)');
		console.error('  2. Navigate to the frontend directory:');
		console.error('     cd C:\\git\\RustHarness\\frontend');
		console.error('  3. Run: npm start');
		console.error('');
		console.error('Alternatively, use the native TUI:');
		console.error('  cargo run -- --tui');
		console.error('');
		process.exit(1);
	}
	throw error;
}
