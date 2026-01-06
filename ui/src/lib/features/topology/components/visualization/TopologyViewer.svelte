<script lang="ts">
	import { type Node, type Edge, type Connection } from '@xyflow/svelte';
	import {
		optionsPanelExpanded,
		selectedEdge,
		selectedNode,
		selectedTopologyId,
		useTopologiesQuery,
		useUpdateTopologyMutation
	} from '../../queries';
	import { type EdgeHandle, type TopologyEdge } from '../../types/base';
	import BaseTopologyViewer from './BaseTopologyViewer.svelte';

	// TanStack Query hooks
	const topologiesQuery = useTopologiesQuery();
	const updateTopologyMutation = useUpdateTopologyMutation();

	// Derived topology from query data
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let topology = $derived(topologiesData.find((t) => t.id === $selectedTopologyId));

	let baseViewer: BaseTopologyViewer | null = $state(null);

	// Selection state synced with stores
	let localSelectedNode: Node | null = $state(null);
	let localSelectedEdge: Edge | null = $state(null);

	export function triggerFitView() {
		baseViewer?.triggerFitView();
	}

	async function handleNodeDragStop(targetNode: Node) {
		if (!topology) return;
		let movedNode = topology.nodes.find((node) => node.id == targetNode?.id);
		if (movedNode && targetNode && targetNode.position) {
			movedNode.position.x = targetNode.position.x;
			movedNode.position.y = targetNode.position.y;
			await updateTopologyMutation.mutateAsync(topology);
		}
	}

	async function handleReconnect(edge: Edge, newConnection: Connection) {
		if (!topology) return;
		const edgeData = edge.data as TopologyEdge;

		if ($selectedEdge && edge.id === $selectedEdge.id) {
			let topologyEdge = topology.edges.find((e) => e.id == edgeData.id);
			if (
				topologyEdge &&
				newConnection.source == topologyEdge.source &&
				newConnection.target == topologyEdge.target &&
				newConnection.sourceHandle &&
				newConnection.targetHandle
			) {
				topologyEdge.source_handle = newConnection.sourceHandle as EdgeHandle;
				topologyEdge.target_handle = newConnection.targetHandle as EdgeHandle;
				const updatedTopology = {
					...topology,
					edges: [...topology.edges]
				};
				await updateTopologyMutation.mutateAsync(updatedTopology);
			}
		}
	}

	function handleNodeSelect(node: Node | null) {
		selectedNode.set(node);
		selectedEdge.set(null);
		optionsPanelExpanded.set(true);
	}

	function handleEdgeSelect(edge: Edge | null) {
		selectedEdge.set(edge);
		selectedNode.set(null);
		optionsPanelExpanded.set(true);
	}

	function handlePaneSelect() {
		selectedNode.set(null);
		selectedEdge.set(null);
	}
</script>

{#if topology}
	<div class="h-[calc(100vh-150px)] w-full">
		<BaseTopologyViewer
			bind:this={baseViewer}
			{topology}
			readonly={false}
			showControls={true}
			bind:selectedNode={localSelectedNode}
			bind:selectedEdge={localSelectedEdge}
			onNodeDragStop={handleNodeDragStop}
			onReconnect={handleReconnect}
			onNodeSelect={handleNodeSelect}
			onEdgeSelect={handleEdgeSelect}
			onPaneSelect={handlePaneSelect}
		/>
	</div>
{/if}
