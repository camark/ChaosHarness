import React from 'react';
import {Box, Text} from 'ink';

export function PromptInput({
	busy,
	input,
	toolName,
}: {
	busy: boolean;
	input: string;
	toolName?: string;
}): React.JSX.Element {
	if (busy) {
		return (
			<Box flexDirection="column">
				<Box>
					<Text color="cyan" bold>{'> '}</Text>
					<Text>{input}</Text>
				</Box>
				<Box>
					<Text color="yellow">Waiting for response...</Text>
				</Box>
			</Box>
		);
	}

	return (
		<Box>
			<Text color="cyan" bold>{'> '}</Text>
			<Text color="green">{input}</Text>
			<Text>▋</Text>
		</Box>
	);
}
