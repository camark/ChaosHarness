import { spawn } from 'child_process';
import { setInterval } from 'timers';

const PROTOCOL_PREFIX = 'OHJSON:';

console.log('=== Testing Frontend Input Handling ===\n');

// Start backend process
const backend = spawn('cargo', ['run', '--', '--stdio-backend'], {
    stdio: ['pipe', 'pipe', 'inherit'],
    cwd: 'C:/git/RustHarness',
    env: process.env
});

console.log('Backend PID:', backend.pid);

let step = 0;
let isReady = false;

// Send test inputs
const sendTest = () => {
    if (!isReady) return;
    
    step++;
    const tests = [
        { input: 'hello test', desc: 'Test 1: Send text input' },
        { input: '\b', desc: 'Test 2: Send backspace' },
    ];
    
    if (step <= tests.length) {
        const test = tests[step - 1];
        console.log(test.desc);
        
        if (step === 1) {
            const msg = JSON.stringify({ type: 'submit_line', line: test.input });
            console.log('Sending:', msg);
            backend.stdin.write(msg + '\n');
        }
        
        setTimeout(sendTest, 1000);
    } else {
        console.log('\n=== Test Complete ===');
        backend.kill();
        process.exit(0);
    }
};

// Read stdout
backend.stdout.on('data', (data) => {
    const lines = data.toString().split('\n');
    for (const line of lines) {
        if (!line.trim()) continue;
        
        if (line.startsWith(PROTOCOL_PREFIX)) {
            const jsonStr = line.slice(PROTOCOL_PREFIX.length);
            try {
                const event = JSON.parse(jsonStr);
                console.log('Event:', event.type, event.item ? event.item : '');
                
                if (event.type === 'ready') {
                    console.log('Backend ready!');
                    isReady = true;
                    sendTest();
                }
                
                if (event.type === 'line_complete') {
                    console.log('Line complete - backend processed input successfully!\n');
                }
            } catch (e) {
                console.error('Parse error:', e);
            }
        }
    }
});

// Timeout
setTimeout(() => {
    console.log('\n=== Test TIMEOUT ===');
    backend.kill();
    process.exit(1);
}, 10000);
