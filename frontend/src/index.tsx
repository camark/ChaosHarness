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
let config: FrontendConfig = {backend_command: ['cargo', 'run', '--', '--stdio-backend']};
try {
	const configStr = process.env.OPENHARNESS_FRONTEND_CONFIG ?? '{}';
	const parsed = JSON.parse(configStr) as Partial<FrontendConfig>;
	config = {
		backend_command: parsed.backend_command ?? ['cargo', 'run', '--', '--stdio-backend'],
		initial_prompt: parsed.initial_prompt,
	};
} catch (e) {
	console.error('Failed to parse OPENHARNESS_FRONTEND_CONFIG:', e);
	console.error('Using default config:', config);
}

render(<App config={config} />, {
	patchConsole: true,
	exitOnCtrlC: false,
});
