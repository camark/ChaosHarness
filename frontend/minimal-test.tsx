import React, {useState} from 'react';
import {render, Text, Box, useInput} from 'ink';

function MinimalTest() {
    const [input, setInput] = useState('');
    const [events, setEvents] = useState<string[]>([]);

    useInput((inputChar, key) => {
        const event = `input=${JSON.stringify(inputChar)}, return=${key.return}, backspace=${key.backspace}`;
        setEvents(prev => [...prev.slice(-5), event]);

        if (key.return) {
            setInput(prev => prev + '[ENTER]');
        } else if (key.backspace) {
            setInput(prev => prev.slice(0, -1) + '[BS]');
        } else if (inputChar) {
            setInput(prev => prev + inputChar);
        }

        if (key.ctrl && inputChar === 'c') {
            process.exit(0);
        }
    });

    return (
        <Box flexDirection="column">
            <Text bold>=== Minimal Input Test ===</Text>
            <Text>Current input: "{input}"</Text>
            <Text>Recent events:</Text>
            {events.map((e, i) => (
                <Text key={i}>{e}</Text>
            ))}
            <Text dimColor>Press Ctrl+C to exit</Text>
        </Box>
    );
}

render(<MinimalTest />, {exitOnCtrlC: false});
