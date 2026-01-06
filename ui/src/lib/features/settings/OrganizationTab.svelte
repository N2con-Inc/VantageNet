<script lang="ts">
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import {
		useOrganizationQuery,
		useUpdateOrganizationMutation,
		useResetOrganizationDataMutation,
		usePopulateDemoDataMutation
	} from '$lib/features/organizations/queries';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { createForm } from '@tanstack/svelte-form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	let {
		subView = $bindable<'main' | 'edit'>('main'),
		onClose
	}: {
		subView?: 'main' | 'edit';
		onClose: () => void;
	} = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	const updateOrganizationMutation = useUpdateOrganizationMutation();
	const resetOrganizationDataMutation = useResetOrganizationDataMutation();
	const populateDemoDataMutation = usePopulateDemoDataMutation();

	let saving = $derived(updateOrganizationMutation.isPending);
	let resetting = $derived(resetOrganizationDataMutation.isPending);
	let populating = $derived(populateDemoDataMutation.isPending);

	let org = $derived(organizationQuery.data);
	let isOwner = $derived(currentUser?.permissions === 'Owner');
	let isDemoOrg = $derived(org?.plan?.type === 'Demo');

	// TanStack Form
	const form = createForm(() => ({
		defaultValues: {
			name: org?.name ?? ''
		},
		onSubmit: async ({ value }) => {
			await handleSave(value.name);
		}
	}));

	// Reset form when switching to edit view
	export function resetForm() {
		if (org) {
			form.reset();
			form.setFieldValue('name', org.name);
		}
	}

	async function handleSave(name: string) {
		if (!org) return;

		try {
			await updateOrganizationMutation.mutateAsync({ id: org.id, name });
			pushSuccess('Organization updated successfully');
			subView = 'main';
		} catch {
			pushError('Failed to update organization');
		}
	}

	function handleCancel() {
		if (subView === 'edit') {
			subView = 'main';
			if (org) {
				form.setFieldValue('name', org.name);
			}
		} else {
			onClose();
		}
	}

	async function handleReset() {
		if (!org) return;

		if (
			!confirm(
				'Are you sure you want to reset all organization data? This will delete all entities and users from your organization. This action cannot be undone.'
			)
		) {
			return;
		}

		try {
			await resetOrganizationDataMutation.mutateAsync(org.id);
			pushSuccess('Organization data has been reset');
		} catch {
			pushError('Failed to reset organization data');
		}
	}

	async function handlePopulateDemo() {
		if (!org) return;

		if (
			!confirm(
				'Are you sure you want to populate demo data? This will add sample networks, hosts, and services to your organization.'
			)
		) {
			return;
		}

		try {
			await populateDemoDataMutation.mutateAsync(org.id);
			pushSuccess('Demo data has been populated');
		} catch {
			pushError('Failed to populate demo data');
		}
	}

	let showSave = $derived(subView === 'edit');
	let cancelLabel = $derived(subView === 'main' ? 'Close' : 'Back');
</script>

<div class="flex min-h-0 flex-1 flex-col">
	{#if org}
		{#if subView === 'main'}
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-6">
					<!-- Organization Info -->
					<InfoCard title="Organization Information">
						<InfoRow label="Name">{org.name}</InfoRow>
						{#if org.plan}
							<InfoRow label="Plan">{org.plan.type}</InfoRow>
						{/if}
						<InfoRow label="Created">
							{formatTimestamp(org.created_at)}
						</InfoRow>
						<InfoRow label="ID" mono={true}>{org.id}</InfoRow>
					</InfoCard>

					<!-- Actions -->
					<InfoCard>
						<div class="flex items-center justify-between">
							<div>
								<p class="text-primary text-sm font-medium">Organization Name</p>
								<p class="text-secondary text-xs">Update your organization's display name</p>
							</div>
							<button
								onclick={() => {
									subView = 'edit';
									form.setFieldValue('name', org.name);
								}}
								class="btn-primary"
							>
								Edit
							</button>
						</div>
					</InfoCard>

					{#if isOwner}
						<!-- Reset Organization Data (available to all org owners) -->
						<InfoCard>
							<div class="flex items-center justify-between">
								<div>
									<p class="text-primary text-sm font-medium">Reset Organization Data</p>
									<p class="text-secondary text-xs">
										Delete everything except for any organization owner user account. This cannot be
										undone.
									</p>
								</div>
								<button onclick={handleReset} disabled={resetting} class="btn-danger">
									{resetting ? 'Resetting...' : 'Reset'}
								</button>
							</div>
						</InfoCard>

						{#if isDemoOrg}
							<!-- Populate Demo Data (only for Demo orgs) -->
							<InfoCard>
								<div class="flex items-center justify-between">
									<div>
										<p class="text-primary text-sm font-medium">Populate Demo Data</p>
										<p class="text-secondary text-xs">
											Fill the organization with sample data for demonstration purposes.
										</p>
									</div>
									<button onclick={handlePopulateDemo} disabled={populating} class="btn-primary">
										{populating ? 'Populating...' : 'Populate'}
									</button>
								</div>
							</InfoCard>
						{/if}
					{/if}
				</div>
			</div>
		{:else if subView === 'edit'}
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-6">
					<p class="text-secondary text-sm">Update your organization's display name</p>
					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<TextInput
								label="Organization Name"
								id="name"
								placeholder="Enter organization name"
								required={true}
								{field}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		{/if}
	{:else}
		<div class="flex-1 overflow-auto p-6">
			<div class="text-secondary py-8 text-center">
				<p>Unable to load organization information</p>
				<p class="text-tertiary mt-2 text-sm">Please try again later</p>
			</div>
		</div>
	{/if}

	<!-- Footer -->
	<div class="modal-footer">
		<div class="flex items-center justify-end gap-3">
			<button type="button" onclick={handleCancel} class="btn-secondary">
				{cancelLabel}
			</button>
			{#if showSave}
				<button
					type="button"
					onclick={() => form.handleSubmit()}
					disabled={saving}
					class="btn-primary"
				>
					{saving ? 'Saving...' : 'Save Changes'}
				</button>
			{/if}
		</div>
	</div>
</div>
