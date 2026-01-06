<script lang="ts">
	/**
	 * Shared component for API key generation and rotation
	 * Used by both daemon API keys and user API keys
	 */
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { RotateCcwKey } from 'lucide-svelte';

	interface Props {
		/** The generated key string to display (null if not yet generated) */
		generatedKey: string | null;
		/** Whether this is editing an existing key (shows rotate UI) or creating new */
		isEditing: boolean;
		/** Whether a generation/rotation operation is in progress */
		loading?: boolean;
		/** Callback to generate a new key */
		onGenerate: () => void | Promise<void>;
		/** Callback to rotate an existing key */
		onRotate: () => void | Promise<void>;
	}

	let { generatedKey, isEditing, loading = false, onGenerate, onRotate }: Props = $props();

	function handleClick() {
		if (isEditing) {
			onRotate();
		} else {
			onGenerate();
		}
	}

	let buttonText = $derived(loading ? 'Generating...' : isEditing ? 'Rotate Key' : 'Generate Key');
</script>

<div class="space-y-3">
	{#if !generatedKey && isEditing}
		<InlineWarning
			title="Generating a new key will invalidate your old key"
			body="Click the button below to generate a new API key. You'll only see it once, so make sure to copy it."
		/>
	{/if}

	{#if generatedKey}
		<InlineWarning
			title="Save this key now"
			body="This key will not be shown again. Copy it now and store it securely."
		/>
	{/if}

	<div class="flex items-start gap-2">
		<button
			type="button"
			class="btn-primary flex-shrink-0 self-stretch"
			onclick={handleClick}
			disabled={loading}
		>
			<RotateCcwKey />
			<span>{buttonText}</span>
		</button>

		<div class="flex-1">
			<CodeContainer
				language="bash"
				expandable={false}
				code={generatedKey ? generatedKey : 'Press Generate Key...'}
			/>
		</div>
	</div>
</div>
