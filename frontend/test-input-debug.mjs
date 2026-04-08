import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Create a simple test script that sends keystrokes
const testApp = `
import React from 'react';
import {render, Text, Box, useInput} from 'ink';

const TestApp = () => {
    const [input, setInput] = React.useState('');
    const [log, setLog] = React.useState([]);

    useInput((inputChar, key) => {
        const entry = {
            input: JSON.stringify(inputChar),
            key: { ...key },
            time: new Date().toISOString()
        };

        setLog(prev => [...prev.slice(-10), entry]);

        if (key.return) {
            setInput(prev => prev + '\\n');
        } else if (key.backspace) {
            setInput(prev => prev.slice(0, -1));
        } else if (inputChar && !key.ctrl && !key.meta) {
            setInput(prev => prev + inputChar);
        }

        if (key.ctrl && inputChar === 'c') {
            process.exit(0);
        }
    });

    return React.createElement(
        Box,
        { flexDirection: 'column' },
        React.createElement(Text, null, 'Input: ' + input),
        React.createElement(Text, null, '---'),
        ...log.map((entry, i) =>
            React.createElement(Text, { key: i },
                'input=' + entry.input + ' | key=' + JSON.stringify(entry.key)
            )
        )
    );
};

render(<TestApp />, { exitOnCtrlC: false });
`;

console.log('=== Ink Input Debug Test ===');
console.log('This test will show what key events Ink receives');
console.log('');
console.log('Run this in a terminal:');
console.log('  cd ' + __dirname);
console.log('  node test-input-debug-runner.mjs');
console.log('');

// Write the test app
import { writeFileSync } from 'fs';
writeFileSync(join(__dirname, 'test-input-debug-app.jsx'), testApp);

// Now run it with script to simulate TTY
const child = spawn('script', ['-q', '-c', 'node test-input-debug-app.jsx'], {
    stdio: 'inherit',
    cwd: __dirname
});

child.on('exit', (code) => {
    console.log('Test exited with code:', code);
});
