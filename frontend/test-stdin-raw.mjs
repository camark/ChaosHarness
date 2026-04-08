#!/usr/bin/env node
// Debug script to see what stdin sends
console.log('=== Stdin Debug ===');
console.log('stdin.isTTY:', process.stdin.isTTY);
console.log('stdout.isTTY:', process.stdout.isTTY);
console.log('');

if (!process.stdin.isTTY) {
    console.log('WARNING: stdin is not a TTY. Raw mode will not work.');
    console.log('Run this script in a real terminal, not through pipes or redirection.');
    console.log('');
    console.log('For testing, you can use:');
    console.log('  script -q -c "node test-stdin-raw.mjs" /dev/null');
    console.log('');
    process.exit(1);
}

console.log('Enabling raw mode...');
process.stdin.setRawMode(true);
process.stdin.resume();
process.stdin.setEncoding('utf8');

console.log('');
console.log('Now type characters. Press Ctrl+C twice to exit.');
console.log('---');

let ctrlCHits = 0;

process.stdin.on('data', (chunk) => {
    const bytes = [...chunk].map(b => '0x' + b.charCodeAt(0).toString(16).padStart(2, '0'));
    const printable = chunk >= ' ' && chunk <= '~' ? chunk : '.';
    console.log(`\nReceived: "${chunk}" | bytes: [${bytes.join(', ')}] | printable: ${printable}`);

    if (chunk === '\x03') {
        ctrlCHits++;
        console.log(`Ctrl+C detected (${ctrlCHits}/2)`);
        if (ctrlCHits >= 2) {
            process.stdin.setRawMode(false);
            console.log('\nExiting...');
            process.exit(0);
        }
    }
});

process.on('exit', () => {
    process.stdin.setRawMode(false);
});
