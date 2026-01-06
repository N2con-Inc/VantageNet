<script lang="ts">
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { DockerDiscovery, NetworkDiscovery, SelfReportDiscovery } from '../../types/api';
	import type { Discovery } from '../../types/base';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { discoveryTypes, subnetTypes } from '$lib/shared/stores/metadata';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { generateCronSchedule } from '../../queries';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';

	// Props
	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; state: { values: Record<string, any> } };
		formData: Discovery;
		readOnly?: boolean;
		daemonHostId: string | null;
		daemon: Daemon;
	}

	let { form, formData = $bindable(), readOnly = false, daemonHostId, daemon }: Props = $props();

	// Queries
	const subnetsQuery = useSubnetsQuery();

	// Derived data
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Discovery type options
	let discoveryTypeOptions = $derived([
		{ value: 'Network', label: 'Network Scan', disabled: false },
		{
			value: 'Docker',
			label: 'Docker Scan',
			disabled: daemonHostId == null || !daemon.capabilities.has_docker_socket
		},
		{ value: 'SelfReport', label: 'Self Report', disabled: daemonHostId == null }
	]);

	const hostNameFallbackOptions = [
		{ value: 'Ip', label: 'IP Address' },
		{ value: 'BestService', label: 'Best Service' }
	];

	const runTypeOptions = [
		{ value: 'AdHoc', label: 'AdHoc (Run on Demand)' },
		{ value: 'Scheduled', label: 'Scheduled (Automatic)' }
	];

	// Handle run type changes - update formData when form field changes
	function handleRunTypeChange(value: string) {
		if (value === 'AdHoc' && formData.run_type.type !== 'AdHoc') {
			formData.run_type = {
				type: 'AdHoc',
				last_run: null
			};
		} else if (value === 'Scheduled' && formData.run_type.type !== 'Scheduled') {
			formData.run_type = {
				type: 'Scheduled',
				cron_schedule: '0 0 */1 * * *', // Default: every hour
				last_run: null,
				enabled: true
			};
		}
	}

	// Handle discovery type changes - update formData when form field changes
	function handleDiscoveryTypeChange(value: string) {
		if (value === 'Network' && formData.discovery_type.type !== 'Network') {
			formData.discovery_type = {
				type: 'Network',
				subnet_ids: daemon.capabilities.interfaced_subnet_ids,
				host_naming_fallback: 'BestService'
			} as NetworkDiscovery;
		} else if (value === 'Docker' && formData.discovery_type.type !== 'Docker') {
			formData.discovery_type = {
				type: 'Docker',
				host_id: daemonHostId,
				host_naming_fallback: 'BestService'
			} as DockerDiscovery;
		} else if (value === 'SelfReport' && formData.discovery_type.type !== 'SelfReport') {
			formData.discovery_type = {
				type: 'SelfReport',
				host_id: daemonHostId
			} as SelfReportDiscovery;
		}
	}

	// Handle host naming fallback changes
	function handleHostNameFallbackChange(value: string) {
		if (formData.discovery_type.type == 'Docker' || formData.discovery_type.type == 'Network') {
			if (formData.discovery_type.host_naming_fallback !== value) {
				formData.discovery_type = {
					...formData.discovery_type,
					host_naming_fallback: value as 'BestService' | 'Ip'
				};
			}
		}
	}

	// Handle schedule changes - update cron from days/hours
	function handleScheduleChange(days: number, hours: number) {
		if (formData.run_type.type === 'Scheduled') {
			const totalHours = days * 24 + hours;
			formData.run_type = {
				...formData.run_type,
				cron_schedule: generateCronSchedule(totalHours)
			};
		}
	}

	// Subnet management for Network
	let availableSubnets = $derived(
		subnetsData.filter(
			(s) =>
				formData.discovery_type.type === 'Network' &&
				s.network_id == formData.network_id &&
				!formData.discovery_type.subnet_ids?.includes(s.id) &&
				subnetTypes.getMetadata(s.subnet_type).network_scan_discovery_eligible
		)
	);

	let selectedSubnets = $derived(
		formData.discovery_type.type === 'Network' && formData.discovery_type.subnet_ids
			? formData.discovery_type.subnet_ids
					.map((id) => subnetsData.find((s) => s.id === id))
					.filter(Boolean)
			: []
	);

	let nonInterfacedSubnets = $derived(
		formData.discovery_type.type == 'Network' &&
			formData.discovery_type.subnet_ids &&
			formData.discovery_type.subnet_ids.length > 0
			? formData.discovery_type.subnet_ids
					.filter((s) => !daemon.capabilities.interfaced_subnet_ids.includes(s))
					.map((s) => subnetsData.find((subnet) => subnet.id == s))
					.filter((s) => s != undefined)
					.map((s) => s.name + ` (${s.cidr})`)
			: []
	);

	function handleAddSubnet(subnetId: string) {
		if (formData.discovery_type.type === 'Network') {
			const currentIds = formData.discovery_type.subnet_ids || [];
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: [...currentIds, subnetId]
			};
		}
	}

	function handleRemoveSubnet(index: number) {
		if (formData.discovery_type.type === 'Network' && formData.discovery_type.subnet_ids) {
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: formData.discovery_type.subnet_ids.filter((_, i) => i !== index)
			};
		}
	}

	// Day and hour options for schedule
	const dayOptions = Array.from({ length: 31 }, (_, i) => ({
		value: String(i),
		label: i === 0 ? 'No days' : i === 1 ? '1 day' : `${i} days`
	}));

	const hourOptions = Array.from({ length: 24 }, (_, i) => ({
		value: String(i),
		label: i === 0 ? 'No hours' : i === 1 ? '1 hour' : `${i} hours`
	}));
</script>

<div class="space-y-6">
	<div class="border-t border-gray-700 pt-6">
		<h3 class="text-primary mb-4 text-lg font-medium">Discovery Configuration</h3>

		<div class="space-y-4">
			<!-- Run Type Selection -->
			<form.Field
				name="run_type_type"
				listeners={{
					onChange: ({ value }: { value: string }) => handleRunTypeChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<SelectInput
						label="Run Type"
						id="run_type"
						options={runTypeOptions}
						{field}
						disabled={readOnly}
					/>
					<p class="text-tertiary mt-1 text-xs">
						{field.state.value === 'AdHoc'
							? 'This discovery will only run when manually triggered'
							: 'This discovery will run automatically on a schedule'}
					</p>
				{/snippet}
			</form.Field>

			<!-- Discovery Type Selection -->
			<form.Field
				name="discovery_type_type"
				listeners={{
					onChange: ({ value }: { value: string }) => handleDiscoveryTypeChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<SelectInput
						label="Discovery Type"
						id="discovery_type"
						options={discoveryTypeOptions}
						{field}
						disabled={readOnly}
					/>
					<p class="text-tertiary mt-1 text-xs">
						{discoveryTypes.getDescription(field.state.value)}
					</p>
				{/snippet}
			</form.Field>

			{#if daemonHostId == null}
				<InlineWarning
					title="Daemon host is missing"
					body="Could not find a host associated to the selected daemon. It may have been deleted or corrupted. Please recreate the daemon."
				/>
			{/if}

			<!-- Type-specific configuration -->
			{#if formData.discovery_type.type == 'Docker' || formData.discovery_type.type == 'Network'}
				<form.Field
					name="host_naming_fallback"
					listeners={{
						onChange: ({ value }: { value: string }) => handleHostNameFallbackChange(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<SelectInput
							label="Host Name Fallback"
							id="host_name_fallback"
							options={hostNameFallbackOptions}
							{field}
							disabled={readOnly}
							helpText="In the event that hostname can't be resolved, what name should be set for discovered hosts? IP Address, or best service (the highest confidence service match)?"
						/>
					{/snippet}
				</form.Field>
			{/if}

			{#if formData.discovery_type.type === 'Network'}
				<div class="rounded-lg bg-gray-800/50 p-4">
					<ListManager
						label="Target Subnets"
						helpText="Select specific subnets to scan, or leave empty to scan all subnets that the daemon has an interface with."
						placeholder="Select a subnet..."
						emptyMessage="All subnets in network will be scanned"
						allowReorder={false}
						allowItemEdit={() => false}
						showSearch={true}
						options={availableSubnets}
						items={selectedSubnets}
						optionDisplayComponent={SubnetDisplay}
						itemDisplayComponent={SubnetDisplay}
						onAdd={handleAddSubnet}
						onRemove={handleRemoveSubnet}
					/>
				</div>
				{#if nonInterfacedSubnets.length > 0}
					<InlineWarning
						title="Non-Interfaced Subnet Added"
						body={`The selected daemon does not have a direct network interface with the following subnets: \n${nonInterfacedSubnets.join('\n')}. \nYou can still include them, but hostnames and MAC addresses will not be available for any discovered hosts.`}
					/>
				{/if}
			{/if}
		</div>
	</div>

	<!-- Frequency Configuration (only for scheduled runs) -->
	{#if formData.run_type.type === 'Scheduled'}
		<div class="border-t border-gray-700 pt-6">
			<h3 class="text-primary mb-4 text-lg font-medium">Schedule Configuration</h3>

			<div class="space-y-4">
				<p class="text-tertiary text-sm">
					Configure how often this discovery should run automatically
				</p>

				<div class="grid grid-cols-2 gap-4">
					<form.Field
						name="schedule_days"
						listeners={{
							onChange: ({ value }: { value: string }) => {
								const hours = form.state.values.schedule_hours || '0';
								handleScheduleChange(parseInt(value), parseInt(hours));
							}
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<SelectInput
								label="Days"
								id="frequency_days"
								options={dayOptions}
								{field}
								disabled={readOnly}
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="schedule_hours"
						listeners={{
							onChange: ({ value }: { value: string }) => {
								const days = form.state.values.schedule_days || '1';
								handleScheduleChange(parseInt(days), parseInt(value));
							}
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<SelectInput
								label="Hours"
								id="frequency_hours"
								options={hourOptions}
								{field}
								disabled={readOnly}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		</div>
	{:else if formData.run_type.type === 'AdHoc'}
		<div class="border-t border-gray-700 pt-6">
			<div class="rounded-lg bg-gray-800/50 p-4">
				<div class="flex items-start gap-3">
					<svg
						class="mt-0.5 h-5 w-5 flex-shrink-0 text-gray-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
						/>
					</svg>
					<div>
						<h4 class="mb-1 text-sm font-medium text-gray-300">Manual Discovery</h4>
						<p class="text-sm text-gray-400">
							This discovery will only run when you manually trigger it from the discoveries page.
							No automatic scheduling is configured.
						</p>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
