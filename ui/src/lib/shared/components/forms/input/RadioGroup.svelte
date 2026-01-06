<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface RadioOption {
		value: string;
		label: string;
	}

	interface Props {
		label: string;
		field: AnyFieldApi;
		id: string;
		options: RadioOption[];
		helpText?: string;
		required?: boolean;
		disabled?: boolean;
	}

	let {
		label,
		field,
		id,
		options,
		helpText = '',
		required = false,
		disabled = false
	}: Props = $props();
</script>

<div class:disabled>
	<FormField {label} {field} {helpText} {id} {required}>
		<div class="flex gap-4">
			{#each options as option (option.value)}
				<label class="flex cursor-pointer items-center gap-2">
					<input
						type="radio"
						name={id}
						value={option.value}
						checked={field.state.value === option.value}
						{disabled}
						onchange={() => field.handleChange(option.value)}
						class="h-4 w-4 border-gray-600 bg-gray-700 text-blue-600 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
					/>
					<span class="text-primary text-sm">{option.label}</span>
				</label>
			{/each}
		</div>
	</FormField>
</div>

<style>
	input[type='radio']:checked {
		background-color: rgb(37, 99, 235);
		border-color: rgb(37, 99, 235);
	}

	.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.disabled :global(label) {
		cursor: not-allowed;
	}
</style>
