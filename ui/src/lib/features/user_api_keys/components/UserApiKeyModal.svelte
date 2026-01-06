<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import { entities } from '$lib/shared/stores/metadata';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import DateInput from '$lib/shared/components/forms/input/DateInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';

	// Shared components
	import ApiKeyGenerator from '$lib/shared/components/api-keys/ApiKeyGenerator.svelte';
	import PermissionSelect from '$lib/shared/components/api-keys/PermissionSelect.svelte';
	import NetworkAccessSelect from '$lib/shared/components/api-keys/NetworkAccessSelect.svelte';

	import type { UserApiKey } from '../queries';
	import {
		createEmptyUserApiKeyFormData,
		useCreateUserApiKeyMutation,
		useRotateUserApiKeyMutation
	} from '../queries';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onUpdate: (data: UserApiKey) => Promise<void> | void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		apiKey?: UserApiKey | null;
	}

	let { isOpen = false, onClose, onUpdate, onDelete = null, apiKey = null }: Props = $props();

	// Mutations
	const createMutation = useCreateUserApiKeyMutation();
	const rotateMutation = useRotateUserApiKeyMutation();

	let loading = $state(false);
	let deleting = $state(false);
	let generatedKey = $state<string | null>(null);

	let isEditing = $derived(apiKey !== null);
	let title = $derived(isEditing ? `Edit ${apiKey?.name || 'API Key'}` : 'Create API Key');

	// Get minimum date (now) in local time format for datetime-local input
	function getLocalDateTimeMin(): string {
		const now = new Date();
		const year = now.getFullYear();
		const month = String(now.getMonth() + 1).padStart(2, '0');
		const day = String(now.getDate()).padStart(2, '0');
		const hours = String(now.getHours()).padStart(2, '0');
		const minutes = String(now.getMinutes()).padStart(2, '0');
		return `${year}-${month}-${day}T${hours}:${minutes}`;
	}
	const today = getLocalDateTimeMin();

	function getDefaultValues(): UserApiKey {
		return apiKey ? { ...apiKey } : createEmptyUserApiKeyFormData();
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: createEmptyUserApiKeyFormData(),
		onSubmit: async ({ value }) => {
			loading = true;
			try {
				if (isEditing) {
					await onUpdate(value as UserApiKey);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Track permission value for NetworkAccessSelect
	let permissionsValue = $derived(form.state.values.permissions);

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		generatedKey = null;
	}

	function handleOnClose() {
		generatedKey = null;
		onClose();
	}

	async function handleGenerateKey() {
		const formData = form.state.values as UserApiKey;

		// Validate required fields before creating
		if (!formData.name?.trim()) {
			pushError('Name is required');
			return;
		}
		if (!formData.network_ids?.length) {
			pushError('At least one network must be selected');
			return;
		}

		loading = true;
		try {
			const result = await createMutation.mutateAsync(formData);
			generatedKey = result.keyString;
		} catch {
			pushError('Failed to generate API key');
		} finally {
			loading = false;
		}
	}

	async function handleRotateKey() {
		const formData = form.state.values as UserApiKey;
		loading = true;
		try {
			const newKey = await rotateMutation.mutateAsync(formData.id);
			generatedKey = newKey;
		} catch {
			pushError('Failed to rotate API key');
		} finally {
			loading = false;
		}
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && apiKey) {
			deleting = true;
			try {
				await onDelete(apiKey.id);
			} finally {
				deleting = false;
			}
		}
	}

	// Handle network selection changes
	function handleNetworkChange(networkIds: string[]) {
		form.setFieldValue('network_ids', networkIds);
	}

	let colorHelper = entities.getColorHelper('UserApiKey');
</script>

<GenericModal
	{isOpen}
	{title}
	size="xl"
	onClose={handleOnClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('UserApiKey')} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<!-- Access Control Info -->
				<details class="bg-secondary/50 text-secondary rounded-lg text-sm">
					<summary class="text-primary cursor-pointer p-4 font-medium">
						API Key Access Reference
					</summary>
					<div class="px-4 pb-4">
						<table class="w-full text-left text-xs">
							<thead>
								<tr class="border-secondary border-b">
									<th class="text-primary pb-2 pr-3 font-medium">Permission</th>
									<th class="text-primary pb-2 pr-3 font-medium">Network Resources</th>
									<th class="text-primary pb-2 pr-3 font-medium">Tags</th>
									<th class="text-primary pb-2 font-medium">Users</th>
								</tr>
							</thead>
							<tbody class="text-secondary">
								<tr class="border-secondary/50 border-b">
									<td class="py-2 pr-3">Viewer</td>
									<td class="py-2 pr-3">Read</td>
									<td class="py-2 pr-3">Read</td>
									<td class="py-2">—</td>
								</tr>
								<tr class="border-secondary/50 border-b">
									<td class="py-2 pr-3">Member</td>
									<td class="py-2 pr-3">Read/Write</td>
									<td class="py-2 pr-3">Read</td>
									<td class="py-2">—</td>
								</tr>
								<tr class="border-secondary/50 border-b">
									<td class="py-2 pr-3">Admin</td>
									<td class="py-2 pr-3">Read/Write</td>
									<td class="py-2 pr-3">Read/Write</td>
									<td class="py-2">Read/Write (Member, Viewer)</td>
								</tr>
								<tr>
									<td class="py-2 pr-3">Owner</td>
									<td class="py-2 pr-3">Read/Write</td>
									<td class="py-2 pr-3">Read/Write</td>
									<td class="py-2">Read/Write (all levels)</td>
								</tr>
							</tbody>
						</table>
						<p class="mt-3 text-xs italic">
							Network resources: hosts, subnets, services, groups. Org settings (name, billing)
							require user session and are not accessible via API keys.
						</p>
					</div>
				</details>

				<!-- Key Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">Key Details</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Name"
								id="name"
								{field}
								placeholder="e.g., CI/CD Pipeline, Terraform, Local Development"
								helpText="A friendly name to help you identify this key"
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="permissions">
						{#snippet children(field)}
							<PermissionSelect
								{field}
								label="Permissions"
								helpText="The maximum permission level this key can have (cannot exceed your own)"
								context="api_key"
							/>
						{/snippet}
					</form.Field>

					<form.Field name="network_ids">
						{#snippet children(field)}
							<NetworkAccessSelect
								selectedNetworkIds={field.state.value ?? []}
								onChange={handleNetworkChange}
								permissionLevel={permissionsValue}
								helpText="Leave empty for org-scoped resources only (tags, users)"
								alwaysShowSelection={true}
								required={false}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="expires_at">
						{#snippet children(field)}
							<DateInput
								label="Expiration Date (Optional)"
								id="expires_at"
								{field}
								helpText="Leave empty for keys that never expire"
								min={today}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="is_enabled">
						{#snippet children(field)}
							<Checkbox
								{field}
								label="Enable API Key"
								helpText="Manually enable or disable API Key. Will be auto-disabled if used after expiry date."
								id="enableApiKey"
							/>
						{/snippet}
					</form.Field>
				</div>

				<!-- Key generation section -->
				<ApiKeyGenerator
					{generatedKey}
					{isEditing}
					{loading}
					onGenerate={handleGenerateKey}
					onRotate={handleRotateKey}
				/>

				<!-- Metadata section for existing keys -->
				{#if isEditing && apiKey}
					<EntityMetadataSection entities={[apiKey]} />
				{/if}
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? 'Deleting...' : 'Delete'}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button
						type="button"
						disabled={loading || deleting}
						onclick={handleOnClose}
						class="btn-secondary"
					>
						Close
					</button>
					{#if isEditing}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? 'Saving...' : 'Save'}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
