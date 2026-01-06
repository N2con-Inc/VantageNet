use serial_test::serial;

use crate::{
    server::{
        auth::middleware::auth::AuthenticatedEntity,
        bindings::r#impl::base::Binding,
        hosts::r#impl::api::{
            BindingInput, InterfaceInput, PortInput, ServiceInput, UpdateHostRequest,
        },
        services::definitions::ServiceDefinitionRegistry,
        shared::{
            services::traits::CrudService,
            storage::{filter::EntityFilter, traits::Storage},
            types::entities::{DiscoveryMetadata, EntitySource},
        },
    },
    tests::*,
};

#[tokio::test]
#[serial]
async fn test_host_deduplication_on_create() {
    let (storage, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create a subnet first for interfaces
    let subnet1 = subnet(&network.id);
    services
        .subnet_service
        .create(subnet1.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    let filter = EntityFilter::unfiltered().network_ids(&[network.id]);

    let start_host_count = storage.hosts.get_all(filter.clone()).await.unwrap().len();

    // Create first host with an interface
    let mut host1 = host(&network.id);
    host1.base.source = EntitySource::Discovery {
        metadata: vec![DiscoveryMetadata::default()],
    };
    let iface1 = interface(&network.id, &subnet1.id);
    let created1 = services
        .host_service
        .discover_host(
            host1.clone(),
            vec![iface1.clone()],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Verify interface was created - use the returned HostResponse directly
    assert!(
        !created1.interfaces.is_empty(),
        "Interface should have been created with host"
    );
    let host1_interface = &created1.interfaces[0];

    // Try to create duplicate (same interfaces - matching by IP+subnet or MAC)
    let mut host2 = host(&network.id);
    host2.base.source = EntitySource::Discovery {
        metadata: vec![DiscoveryMetadata::default()],
    };
    // Use the same interface data to trigger deduplication
    let created2 = services
        .host_service
        .discover_host(
            host2.clone(),
            vec![host1_interface.clone()],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Should return same host (upserted)
    assert_eq!(created1.id, created2.id);

    // Verify only one host in DB
    let end_host_count = storage.hosts.get_all(filter).await.unwrap().len();
    assert_eq!(start_host_count + 1, end_host_count);
}

#[tokio::test]
#[serial]
async fn test_host_upsert_merges_new_data() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create subnets
    let subnet1 = subnet(&network.id);
    services
        .subnet_service
        .create(subnet1.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let subnet2 = subnet(&network.id);
    services
        .subnet_service
        .create(subnet2.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with one interface
    let mut host1 = host(&network.id);
    host1.base.source = EntitySource::Discovery {
        metadata: vec![DiscoveryMetadata::default()],
    };
    let iface1 = interface(&network.id, &subnet1.id);

    let created = services
        .host_service
        .discover_host(
            host1.clone(),
            vec![iface1.clone()],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Verify interface was created
    assert!(
        !created.interfaces.is_empty(),
        "Interface should have been created with host"
    );
    let created_iface1 = &created.interfaces[0];

    // Create "duplicate" with additional interface (matching first interface triggers upsert)
    let mut host2 = host(&network.id);
    host2.base.source = EntitySource::Discovery {
        metadata: vec![DiscoveryMetadata::default()],
    };
    let iface2 = interface(&network.id, &subnet2.id);

    // Use the CREATED interface to ensure deduplication matching works
    let upserted = services
        .host_service
        .discover_host(
            host2.clone(),
            vec![created_iface1.clone(), iface2],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Should have merged - same host ID
    assert_eq!(upserted.id, created.id);

    // Check interface count - use returned HostResponse
    assert_eq!(
        upserted.interfaces.len(),
        2,
        "Upserted host should have 2 interfaces"
    );

    if let EntitySource::Discovery { metadata } = upserted.source {
        assert_eq!(metadata.len(), 2)
    } else {
        panic!("Got a different type of source after upserting")
    }
}

#[tokio::test]
#[serial]
async fn test_host_consolidation() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host1 without interfaces
    let host1 = host(&network.id);
    let created1 = services
        .host_service
        .discover_host(
            host1.clone(),
            vec![],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Create host2 with an interface
    let host2 = host(&network.id);
    let port = port(&network.id, &host2.id);
    let iface = interface(&network.id, &subnet_obj.id);

    let mut svc = service(&network.id, &host2.id);
    svc.base.bindings = vec![Binding::new_port_serviceless(port.id, Some(iface.id))];

    let created2 = services
        .host_service
        .discover_host(
            host2.clone(),
            vec![iface],
            vec![port],
            vec![svc],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_svc = &created2.services[0];

    // Consolidate host2 into host1
    let consolidated = services
        .host_service
        .consolidate_hosts(
            created1.to_host(),
            created2.to_host(),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Host1 should have host2's service
    assert!(consolidated.services.contains(&created_svc));

    // Host2 should be deleted
    let host2_after = services.host_service.get_by_id(&created2.id).await.unwrap();
    assert!(host2_after.is_none());

    // Service should now belong to host1
    let svc_after = services
        .service_service
        .get_by_id(&created_svc.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(svc_after.base.host_id, consolidated.id);
}

/// Test that port bindings can be transferred between services via host update.
/// This is a regression test for the bug where transfers failed with
/// "Port is already bound to X" even when transferring via the Transfer Ports feature.
#[tokio::test]
#[serial]
async fn test_port_transfer_between_services_via_host_update() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with interface and port
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port_obj = port(&network.id, &host_obj.id);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port_obj],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_iface = &created_host.interfaces[0];
    let created_port = &created_host.ports[0];

    // Create two services - service A has the port binding
    let mut svc_a = service(&network.id, &created_host.id);
    svc_a.base.name = "Unclaimed Open Ports".to_string();
    svc_a.base.source = EntitySource::Manual;
    svc_a.base.position = 0;
    svc_a.base.bindings = vec![Binding::new_port_serviceless(
        created_port.id,
        Some(created_iface.id),
    )];

    let created_svc_a = services
        .service_service
        .create(svc_a, AuthenticatedEntity::System)
        .await
        .unwrap();

    let mut svc_b = service(&network.id, &created_host.id);
    svc_b.base.name = "Target Service".to_string();
    svc_b.base.source = EntitySource::Manual;
    svc_b.base.position = 1;
    svc_b.base.bindings = vec![]; // No bindings initially

    let created_svc_b = services
        .service_service
        .create(svc_b, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Get a service definition for the inputs
    let service_def = ServiceDefinitionRegistry::find_by_id("Dns Server")
        .unwrap_or_else(|| ServiceDefinitionRegistry::all_service_definitions()[0].clone());

    // Create an update request that transfers the port binding from service A to service B
    // This simulates what the frontend does with "Transfer Ports"
    let binding_id = created_svc_a.base.bindings[0].id;
    let update_request = UpdateHostRequest {
        id: created_host.id,
        name: created_host.name.clone(),
        hostname: created_host.hostname.clone(),
        description: created_host.description.clone(),
        virtualization: None,
        hidden: false,
        tags: vec![],
        expected_updated_at: None,
        interfaces: vec![InterfaceInput {
            id: created_iface.id,
            subnet_id: created_iface.base.subnet_id,
            ip_address: created_iface.base.ip_address,
            mac_address: created_iface.base.mac_address,
            name: created_iface.base.name.clone(),
            position: Some(0),
        }],
        ports: vec![PortInput {
            id: created_port.id,
            number: created_port.base.port_type.number(),
            protocol: created_port.base.port_type.protocol(),
        }],
        services: vec![
            // Service A: no longer has the binding
            ServiceInput {
                id: created_svc_a.id,
                service_definition: service_def.clone(),
                name: "Unclaimed Open Ports".to_string(),
                bindings: vec![], // Binding removed
                virtualization: None,
                tags: vec![],
                position: Some(0),
            },
            // Service B: now has the binding
            ServiceInput {
                id: created_svc_b.id,
                service_definition: service_def.clone(),
                name: "Target Service".to_string(),
                bindings: vec![BindingInput::Port {
                    id: binding_id, // Reuse the binding ID
                    port_id: created_port.id,
                    interface_id: Some(created_iface.id),
                }],
                virtualization: None,
                tags: vec![],
                position: Some(1),
            },
        ],
    };

    // This should succeed - the fix ensures services losing bindings are processed first
    let result = services
        .host_service
        .update_from_request(update_request, AuthenticatedEntity::System)
        .await;

    assert!(
        result.is_ok(),
        "Port transfer should succeed, but got error: {:?}",
        result.err()
    );

    let updated_host = result.unwrap();

    // Verify service A no longer has the binding
    let svc_a_after = updated_host
        .services
        .iter()
        .find(|s| s.id == created_svc_a.id)
        .expect("Service A should still exist");
    assert!(
        svc_a_after.base.bindings.is_empty(),
        "Service A should have no bindings after transfer"
    );

    // Verify service B now has the binding
    let svc_b_after = updated_host
        .services
        .iter()
        .find(|s| s.id == created_svc_b.id)
        .expect("Service B should still exist");
    assert_eq!(
        svc_b_after.base.bindings.len(),
        1,
        "Service B should have 1 binding after transfer"
    );

    // Verify the binding is for the correct port
    let binding = &svc_b_after.base.bindings[0];
    assert_eq!(
        binding.base.binding_type,
        crate::server::bindings::r#impl::base::BindingType::Port {
            port_id: created_port.id,
            interface_id: Some(created_iface.id),
        },
        "Binding should be for the transferred port"
    );
}
