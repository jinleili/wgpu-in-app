use std::ffi::CStr;

use ash::vk;
use hal::api::Vulkan;

pub(super) async fn request_device(
    instance: &wgpu::Instance,
    backend: wgpu::Backends,
    surface: &wgpu::Surface,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(instance, backend, Some(surface))
            .await
            .expect("No suitable GPU adapters found on the system!");

    let desc = wgpu::DeviceDescriptor {
        label: None,
        features: adapter.features(),
        limits: adapter.limits(),
    };
    // We have a suitable adapter, we need to manually create the device
    let hal_device = unsafe {
        adapter.as_hal::<Vulkan, _, _>(|adapter| {
            // We only asked for Vulkan adapters
            let adapter = adapter.unwrap();
            let mut enabled_extensions = adapter.required_device_extensions(desc.features);
            enabled_extensions.extend(EXTRA_REQUIRED_EXTENSIONS);

            let phd_limits = &adapter.physical_device_capabilities().properties().limits;
            let uab_types = hal::UpdateAfterBindTypes::from_limits(&desc.limits, phd_limits);
            let mut phd_features =
                adapter.physical_device_features(&enabled_extensions, desc.features, uab_types);

            // Find a queue.
            let family_index = 0; //TODO
            let family_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(family_index)
                .queue_priorities(&[1.0])
                .build();
            let family_infos = [family_info];

            let str_pointers = enabled_extensions
                .iter()
                .map(|&s| s.as_ptr())
                .collect::<Vec<_>>();

            let pre_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&family_infos)
                .enabled_extension_names(&str_pointers);
            let info = phd_features.add_to_device_create_builder(pre_info).build();

            let raw_device = adapter
                .shared_instance()
                .raw_instance()
                .create_device(adapter.raw_physical_device(), &info, None)
                .expect("Failed to create Vulkan device");

            adapter.device_from_raw(
                raw_device,
                true,
                &enabled_extensions,
                desc.features,
                uab_types,
                family_index,
                0,
            )
        })
    }
    .expect("Failed to create hal device");
    let (device, queue) = unsafe { adapter.create_device_from_hal(hal_device, &desc, None) }
        .expect("Failed to create hal device");

    (adapter, device, queue)
}

const EXTRA_REQUIRED_EXTENSIONS: &[&CStr] = &[
    vk::KhrBindMemory2Fn::name(),
    vk::KhrExternalMemoryFn::name(),
    vk::KhrGetMemoryRequirements2Fn::name(),
    vk::ExtQueueFamilyForeignFn::name(),
    vk::AndroidExternalMemoryAndroidHardwareBufferFn::name(),
];
