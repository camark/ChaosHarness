import { spawn } from 'child_process';
import readline from 'readline';
import { createInterface } from 'readline';

const PROTOCOL_PREFIX = 'OHJSON:';

console.log('=== Frontend Debug Test ===');
console.log('Starting backend process...');

// Start backend process
const backend = spawn('cargo', ['run', '--', '--stdio-backend'], {
    stdio: ['pipe', 'pipe', 'inherit'],
    cwd: 'C:/git/RustHarness',
    env: process.env
});

console.log('Backend PID:', backend.pid);

let isReady = false;

// Read stdout with detailed logging
const reader = createInterface({
    input: backend.stdout,
    crlfDelay: Infinity
});

reader.on('line', (line) => {
    console.log('\n<<< RAW LINE:', line);

    if (!line.startsWith(PROTOCOL_PREFIX)) {
        console.log('<<< Non-OHJSON line, ignoring');
        return;
    }

    const jsonStr = line.slice(PROTOCOL_PREFIX.length);
    console.log('<<< JSON:', jsonStr);

    try {
        const event = JSON.parse(jsonStr);
        console.log('<<< EVENT TYPE:', event.type);

        if (event.type === 'ready') {
            console.log('<<< Backend ready! Sending test message...');
            isReady = true;
            const msg = JSON.stringify({ type: 'submit_line', line: 'hello from test' });
            console.log('>>> Sending:', msg);
            backend.stdin.write(msg + '\n', (err) => {
                if (err) {
                    console.error('>>> Write error:', err);
                } else {
                    console.log('>>> Write success!');
                }
            });
        } else if (event.type === 'transcript_item') {
            console.log('<<< Transcript:', event.item?.role, '-', event.item?.text);
        } else if (event.type === 'line_complete') {
            console.log('<<< Line complete - test PASSED!');
            console.log('\n=== Test PASSED ===');
            backend.kill();
            process.exit(0);
        }
    } catch (e) {
        console.error('<<< Parse error:', e);
    }
});

backend.stderr?.on('data', (data) => {
    console.log('STDERR:', data.toString());
});

backend.on('exit', (code) => {
    console.log('Backend exited with code:', code);
    if (!isReady) {
        console.log('=== Test FAILED - Backend exited before ready ===');
        process.exit(1);
    }
});

// Timeout after 15 seconds
setTimeout(() => {
    console.log('\n=== Test TIMEOUT ===');
    backend.kill();
    process.exit(1);
}, 15000);
