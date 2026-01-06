<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max, min } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { Topology } from '../types/base';
	import {
		createEmptyTopologyFormData,
		useCreateTopologyMutation,
		useTopologiesQuery,
		useUpdateTopologyMutation,
		selectedTopologyId,
		topologyOptions
	} from '../queries';
	import { entities } from '$lib/shared/stores/metadata';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import { TopologyDisplay } from '$lib/shared/components/forms/selection/display/TopologyDisplay.svelte';

	// TanStack Query hooks
	const networksQuery = useNetworksQuery();
	const topologiesQuery = useTopologiesQuery();
	const createTopologyMutation = useCreateTopologyMutation();
	const updateTopologyMutation = useUpdateTopologyMutation();

	let networksData = $derived(networksQuery.data ?? []);
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	let {
		isOpen = $bindable(false),
		onSubmit,
		onClose,
		topo = null
	}: {
		isOpen: boolean;
		onSubmit: () => Promise<void> | void;
		onClose: () => void;
		topo: Topology | null;
	} = $props();

	let isEditing = $derived(topo != null);
	let title = $derived(isEditing ? `Edit ${topo?.name}` : 'Create Topology');

	let loading = $state(false);

	function getDefaultValues(): Topology {
		if (topo) {
			return { ...topo };
		}
		// For new topologies, pre-select the currently viewed topology as parent
		const currentTopology = $selectedTopologyId
			? topologiesData.find((t) => t.id === $selectedTopologyId)
			: null;
		const networkId = currentTopology?.network_id ?? defaultNetworkId;
		const defaults = createEmptyTopologyFormData(networkId);
		// Default to current topology, or first available on this network
		if (currentTopology) {
			defaults.parent_id = currentTopology.id;
		} else {
			const firstAvailable = topologiesData.find((t) => t.network_id === networkId);
			if (firstAvailable) {
				defaults.parent_id = firstAvailable.id;
			}
		}
		return defaults;
	}

	// Create form with additional creation_mode field for UI
	const form = createForm(() => ({
		defaultValues: { ...createEmptyTopologyFormData(''), creation_mode: 'branch' },
		onSubmit: async ({ value }) => {
			// eslint-disable-next-line @typescript-eslint/no-unused-vars
			const { creation_mode, ...topologyFields } = value as Topology & { creation_mode: string };
			const topologyData: Topology = {
				...topologyFields,
				name: topologyFields.name.trim(),
				options: $topologyOptions
			};

			loading = true;
			try {
				if (isEditing) {
					await updateTopologyMutation.mutateAsync(topologyData);
				} else {
					const created = await createTopologyMutation.mutateAsync(topologyData);
					// Select the newly created topology
					selectedTopologyId.set(created.id);
				}
				await onSubmit();
			} finally {
				loading = false;
			}
		}
	}));

	// Local state for network_id to enable Svelte 5 reactivity
	// (form.state.values is NOT tracked by $derived)
	let selectedNetworkId = $state<string>('');

	// Sync form values to local state on store changes
	$effect(() => {
		return form.store.subscribe(() => {
			selectedNetworkId = form.state.values.network_id;
		});
	});

	// Local state for creation mode to enable Svelte 5 reactivity
	let creationMode = $state<'branch' | 'fresh'>('branch');
	let previousCreationMode = $state<'branch' | 'fresh'>('branch');

	// Sync creation mode from form store and handle changes
	$effect(() => {
		return form.store.subscribe(() => {
			const newMode = (form.state.values as { creation_mode?: string }).creation_mode as
				| 'branch'
				| 'fresh';
			if (newMode !== previousCreationMode) {
				previousCreationMode = newMode;
				creationMode = newMode;
				// Update parent_id based on mode change
				if (newMode === 'fresh') {
					form.setFieldValue('parent_id', null);
				} else if (availableTopologies.length > 0 && !form.state.values.parent_id) {
					form.setFieldValue('parent_id', availableTopologies[0].id);
				}
			}
		});
	});

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		const hasParent = defaults.parent_id !== null;
		const mode = hasParent ? 'branch' : 'fresh';
		form.reset({
			...defaults,
			creation_mode: mode
		});
		selectedNetworkId = defaults.network_id;
		creationMode = mode;
		previousCreationMode = mode;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	// Creation mode options
	const creationModeOptions = [
		{ value: 'branch', label: 'Branch from existing' },
		{ value: 'fresh', label: 'Start fresh' }
	];

	// Available topologies for parent selection (exclude current and filter by network)
	let availableTopologies = $derived(
		topologiesData.filter(
			(t) => t.id !== form.state.values.id && t.network_id === selectedNetworkId
		)
	);

	let colorHelper = entities.getColorHelper('Topology');
	let Icon = entities.getIconComponent('Topology');
</script>

<GenericModal {isOpen} {title} size="md" {onClose} onOpen={handleOpen} showCloseButton={true}>
	{#snippet headerIcon()}
		<ModalHeaderIcon {Icon} color={colorHelper.color} />
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
			<div class="space-y-4">
				<form.Field name="network_id">
					{#snippet children(field)}
						<SelectNetwork
							selectedNetworkId={field.state.value}
							onNetworkChange={(id) => field.handleChange(id)}
							disabled={isEditing}
						/>
					{/snippet}
				</form.Field>

				{#if !isEditing && availableTopologies.length > 0}
					<form.Field name="creation_mode">
						{#snippet children(field)}
							<RadioGroup
								label="Creation Mode"
								id="creation_mode"
								{field}
								options={creationModeOptions}
								disabled={isEditing}
							/>
						{/snippet}
					</form.Field>
				{/if}

				{#if creationMode === 'branch' && availableTopologies.length > 0}
					<form.Field name="parent_id">
						{#snippet children(field)}
							<div>
								<RichSelect
									label={isEditing ? 'Parent' : 'Select a parent to branch off of'}
									displayComponent={TopologyDisplay}
									required={false}
									disabled={isEditing}
									selectedValue={field.state.value}
									onSelect={(id) => field.handleChange(id)}
									options={availableTopologies}
								/>
							</div>
						{/snippet}
					</form.Field>
				{/if}

				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }) => required(value) || max(100)(value) || min(3)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput label="Name" id="name" {field} placeholder="Enter topology name" required />
					{/snippet}
				</form.Field>
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" disabled={loading} onclick={onClose} class="btn-secondary">
					Cancel
				</button>
				<button type="submit" disabled={loading} class="btn-primary">
					{loading ? 'Saving...' : 'Save'}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
