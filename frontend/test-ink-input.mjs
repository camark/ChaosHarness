import React from 'react';
import {render, Text, Box, useInput} from 'ink';

function DebugApp() {
    const [events, setEvents] = React.useState([]);

    useInput((input, key) => {
        const event = {
            input: JSON.stringify(input),
            key: {
                backspace: key.backspace,
                return: key.return,
                ...key
            },
            time: Date.now()
        };

        React.startTransition(() => {
            setEvents(prev => [...prev.slice(-15), event]);
        });

        // Exit on Ctrl+C
        if (key.ctrl && input === 'c') {
            process.exit(0);
        }
    });

    return React.createElement(
        Box,
        { flexDirection: 'column', border: 'single', padding: 1 },
        React.createElement(Text, { bold: true }, '=== Ink Input Debug ==='),
        React.createElement(Text, null, 'Press any key to see events'),
        React.createElement(Text, null, 'Last 15 events:'),
        React.createElement(Text, null, '─'.repeat(50)),
        ...events.map((e, i) =>
            React.createElement(Text, { key: i },
                `input=${e.input.padEnd(10)} | backspace=${!!e.key.backspace} | return=${!!e.key.return}`
            )
        )
    );
}

const { waitUntilExit } = render(<DebugApp />, { exitOnCtrlC: false });
waitUntilExit();
