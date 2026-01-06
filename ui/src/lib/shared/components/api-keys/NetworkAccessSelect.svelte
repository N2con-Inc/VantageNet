<script lang="ts">
	/**
	 * Shared component for selecting network access
	 * Used by user API keys, user invites, and user management
	 *
	 * Filters available networks based on the current user's network access
	 * (users can only grant access to networks they have access to)
	 */
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { Network } from '$lib/features/networks/types';
	import type { UserOrgPermissions } from '$lib/features/users/types';

	interface Props {
		/** Currently selected network IDs */
		selectedNetworkIds: string[];
		/** Callback when selection changes */
		onChange: (networkIds: string[]) => void;
		/** The permission level being granted (affects whether network selection is shown) */
		permissionLevel?: UserOrgPermissions;
		/** Label for the list manager */
		label?: string;
		/** Help text to display */
		helpText?: string;
		/** Whether selection is required */
		required?: boolean;
		/** Always show network selection regardless of permission level (for API keys) */
		alwaysShowSelection?: boolean;
	}

	let {
		selectedNetworkIds,
		onChange,
		permissionLevel = 'Viewer',
		label = 'Networks',
		helpText = 'Select networks this entity will have access to',
		required = false,
		alwaysShowSelection = false
	}: Props = $props();

	// Get current user and networks
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// Permissions that have access to all networks (don't need explicit selection)
	let networksNotNeeded = $derived(
		permissions
			.getItems()
			.filter((p) => p.metadata.manage_org_entities)
			.map((p) => p.id)
	);

	// Check if network selection is needed for this permission level
	// Always show if alwaysShowSelection is true (e.g., for API keys)
	let needsNetworkSelection = $derived(
		alwaysShowSelection || !networksNotNeeded.includes(permissionLevel)
	);

	// Convert selected IDs to Network objects
	let selectedNetworks = $derived(
		selectedNetworkIds
			.map((id) => networksData.find((n) => n.id === id))
			.filter((n): n is Network => n !== undefined)
	);

	// Available networks (filtered by current user's access, excluding already selected)
	let networkOptions = $derived(
		networksData
			.filter((n) => {
				// If current user is Owner/Admin, show all networks
				if (currentUser && networksNotNeeded.includes(currentUser.permissions)) {
					return true;
				}
				// Otherwise, only show networks the current user has access to
				return currentUser ? currentUser.network_ids.includes(n.id) : false;
			})
			.filter((n) => !selectedNetworkIds.includes(n.id))
	);

	function handleAddNetwork(id: string) {
		onChange([...selectedNetworkIds, id]);
	}

	function handleRemoveNetwork(index: number) {
		const newIds = [...selectedNetworkIds];
		newIds.splice(index, 1);
		onChange(newIds);
	}
</script>

{#if needsNetworkSelection}
	<ListManager
		{label}
		{helpText}
		{required}
		allowReorder={false}
		allowAddFromOptions={true}
		allowCreateNew={false}
		allowItemEdit={() => false}
		disableCreateNewButton={false}
		onAdd={handleAddNetwork}
		onRemove={handleRemoveNetwork}
		options={networkOptions}
		optionDisplayComponent={NetworkDisplay}
		items={selectedNetworks}
		itemDisplayComponent={NetworkDisplay}
	/>
{:else}
	<div class="card card-static">
		<p class="text-secondary text-sm">
			Users with {permissionLevel} permissions have access to all networks.
		</p>
	</div>
{/if}
