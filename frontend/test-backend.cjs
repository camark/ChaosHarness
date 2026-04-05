const { spawn } = require('child_process');
const readline = require('readline');

console.log('Starting backend...');

// Start backend process
const backend = spawn('cargo', ['run', '--', '--stdio-backend'], {
    stdio: ['pipe', 'pipe', 'inherit'],
    cwd: 'C:/git/RustHarness'
});

console.log('Backend PID:', backend.pid);

// Read stdout
const reader = readline.createInterface({ input: backend.stdout });

reader.on('line', (line) => {
    console.log('Received:', line);

    if (line.startsWith('OHJSON:')) {
        const event = JSON.parse(line.slice(7));
        console.log('Event type:', event.type);

        if (event.type === 'ready') {
            console.log('Backend ready! Sending test message...');
            backend.stdin.write(JSON.stringify({ type: 'submit_line', line: 'hello test' }) + '\n');
        }
    }
});

backend.on('exit', (code) => {
    console.log('Backend exited with code:', code);
    process.exit(code);
});

backend.stdin.on('error', (err) => {
    console.error('stdin error:', err);
});

// Timeout
setTimeout(() => {
    console.log('Test timeout, killing backend...');
    backend.kill();
    process.exit(0);
}, 10000);
