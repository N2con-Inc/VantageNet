<script lang="ts">
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { ArrowBigUpDash } from 'lucide-svelte';
	import type { Daemon } from '../types/base';
	import { VERSION } from '$lib/version';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		daemon: Daemon;
	}

	let { isOpen = false, onClose, daemon }: Props = $props();

	// Commands for upgrading
	const binaryUpgradeCommand = `bash -c "$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)"`;

	const dockerComposeLatestPull = `docker compose pull
docker compose up -d`;

	const dockerComposeImageLine = `image: ghcr.io/scanopy/scanopy/daemon:latest`;

	let colorHelper = entities.getColorHelper('Daemon');
</script>

<GenericModal {isOpen} title="Upgrade Daemon" size="lg" {onClose}>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={ArrowBigUpDash} color={colorHelper.color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<p class="text-secondary">
					An update is available for daemon <span class="text-primary font-medium"
						>{daemon.name}</span
					>.
					{#if daemon.version_status.version}
						Current version: <span class="font-mono">{daemon.version_status.version}.</span>
					{/if}
					Latest version: <span class="font-mono">{VERSION}.</span>
				</p>

				<!-- Binary Installation -->
				<div class="space-y-3">
					<h3 class="text-primary font-medium">Binary Installation</h3>
					<p class="text-secondary text-sm">
						Run the install script to upgrade to the latest version. This will replace the existing
						binary.
					</p>
					<CodeContainer language="bash" expandable={false} code={binaryUpgradeCommand} />
					<p class="text-secondary text-sm">
						After updating, restart the daemon service for changes to take effect.
					</p>
				</div>

				<!-- Docker Compose Installation -->
				<div class="space-y-3">
					<h3 class="text-primary font-medium">Docker Compose Installation</h3>

					<div class="space-y-2">
						<p class="text-secondary text-sm">
							If you're using the <span class="font-mono">:latest</span> tag, pull the new image and
							restart:
						</p>
						<CodeContainer language="bash" expandable={false} code={dockerComposeLatestPull} />
					</div>

					<div class="space-y-2">
						<p class="text-secondary text-sm">
							If you're pinned to a specific version, update the image line in your
							<span class="font-mono">docker-compose.yml</span>:
						</p>
						<CodeContainer language="yaml" expandable={false} code={dockerComposeImageLine} />
						<p class="text-secondary text-sm">
							Then run <span class="font-mono">docker compose up -d</span> to apply the changes.
						</p>
					</div>
				</div>
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end">
				<button type="button" class="btn-secondary" onclick={onClose}>Close</button>
			</div>
		</div>
	</div>
</GenericModal>
