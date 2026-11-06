//! GPU context: the Vulkan loader, instance, debug messenger, window surface,
//! logical device and swapchain.
//!
//! Creation order: loader → instance → debug messenger → surface → physical device
//! (chosen) → logical device → queue (fetched) → swapchain → image views.
//! `Drop` destroys in the mirror order.

use std::ffi::{CStr, c_char, c_void};
use std::fmt;

use ash::{Device, Entry, Instance, ext, khr, vk};
use raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};

use crate::swapchain::Swapchain;

const VALIDATION_LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";

/// Everything that can go wrong while setting up the GPU.
#[derive(Debug)]
pub enum GpuError {
    /// The Vulkan loader library (`libvulkan`) could not be opened.
    Load(ash::LoadingError),
    /// A Vulkan call returned an error code.
    Vulkan(vk::Result),
    /// No GPU has a queue family that can draw *and* present to the window.
    NoSuitableGpu,
    /// The window couldn't hand over its platform handles.
    WindowHandle(HandleError),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuError::Load(e) => write!(f, "could not load the Vulkan library: {e}"),
            GpuError::Vulkan(e) => write!(f, "Vulkan error: {e}"),
            GpuError::NoSuitableGpu => {
                write!(f, "no GPU can both draw and present to this window")
            }
            GpuError::WindowHandle(e) => write!(f, "could not get the window's handles: {e}"),
        }
    }
}

impl std::error::Error for GpuError {}

impl From<ash::LoadingError> for GpuError {
    fn from(e: ash::LoadingError) -> Self {
        GpuError::Load(e)
    }
}

impl From<HandleError> for GpuError {
    fn from(e: HandleError) -> Self {
        GpuError::WindowHandle(e)
    }
}

impl From<vk::Result> for GpuError {
    fn from(e: vk::Result) -> Self {
        GpuError::Vulkan(e)
    }
}

/// Owns every Vulkan object needed before anything can be drawn.
///
/// Deliberately not `Clone` or `Copy`: one owner, so one destroy.
/// **Must be dropped before the window it was created from** (the surface points at it).
pub struct GpuContext {
    _entry: Entry, // keeps the loader library open for as long as the instance lives
    instance: Instance,
    debug: Option<DebugMessenger>,
    surface_loader: khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: Device,
    queue: vk::Queue,
    queue_family_index: u32,
    swapchain_loader: khr::swapchain::Device,
    swapchain: Swapchain,
}

impl GpuContext {
    /// Sets up Vulkan for drawing into `window`.
    ///
    /// `size` is the window's size in **physical** pixels (not logical points).
    /// The caller must drop the returned context before dropping the window.
    pub fn new(
        app_name: &CStr,
        window: &(impl HasDisplayHandle + HasWindowHandle),
        size: (u32, u32),
    ) -> Result<Self, GpuError> {
        let display_handle = window.display_handle()?.as_raw();
        let window_handle = window.window_handle()?.as_raw();
        // The instance extensions this platform's windows need
        // (on macOS: VK_KHR_surface + VK_EXT_metal_surface).
        let window_extensions = ash_window::enumerate_required_extensions(display_handle)?;

        let entry = load_entry()?;

        let mut validation = cfg!(debug_assertions) && has_layer(&entry, VALIDATION_LAYER)?;
        if cfg!(debug_assertions) && !validation {
            eprintln!("warning: validation layer not found, running without it");
        }
        let portability = has_instance_extension(&entry, khr::portability_enumeration::NAME)?;

        // The layer can be listed (its JSON description was found) yet fail to load
        // (its library wasn't). Run without it rather than refusing to start.
        let instance =
            match create_instance(&entry, app_name, window_extensions, validation, portability) {
                Err(GpuError::Vulkan(vk::Result::ERROR_LAYER_NOT_PRESENT)) if validation => {
                    eprintln!("warning: validation layer failed to load, running without it");
                    validation = false;
                    create_instance(&entry, app_name, window_extensions, validation, portability)?
                }
                result => result?,
            };

        // From here on, `Drop` can't clean up if a step fails (there's no struct yet),
        // so each failure destroys what already exists, children first, by hand.
        let debug = if validation {
            match create_debug_messenger(&entry, &instance) {
                Ok(d) => Some(d),
                Err(e) => {
                    destroy_instance_level(&instance, &None);
                    return Err(e);
                }
            }
        } else {
            None
        };

        let surface_loader = khr::surface::Instance::new(&entry, &instance);
        // SAFETY: the handles come from a live window that the caller keeps alive
        // for longer than this context.
        let surface = match unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)
        } {
            Ok(s) => s,
            Err(e) => {
                destroy_instance_level(&instance, &debug);
                return Err(e.into());
            }
        };
        let fail = |e: GpuError| -> GpuError {
            // SAFETY: nothing created from the surface exists yet.
            unsafe { surface_loader.destroy_surface(surface, None) };
            destroy_instance_level(&instance, &debug);
            e
        };

        let (physical_device, queue_family_index) =
            pick_physical_device(&instance, &surface_loader, surface).map_err(&fail)?;
        let device =
            create_device(&instance, physical_device, queue_family_index).map_err(&fail)?;

        // SAFETY: family `queue_family_index` was requested with 1 queue, so index 0 exists.
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let swapchain_loader = khr::swapchain::Device::new(&instance, &device);
        let swapchain = Swapchain::new(
            &device,
            &swapchain_loader,
            &surface_loader,
            surface,
            physical_device,
            size,
        )
        .map_err(|e| {
            // SAFETY: the swapchain cleaned up after itself; nothing else uses the device.
            unsafe { device.destroy_device(None) };
            fail(e)
        })?;

        Ok(GpuContext {
            _entry: entry,
            instance,
            debug,
            surface_loader,
            surface,
            physical_device,
            device,
            queue,
            queue_family_index,
            swapchain_loader,
            swapchain,
        })
    }

    /// Format of the swapchain images (e.g. `B8G8R8A8_SRGB`).
    pub fn swapchain_format(&self) -> vk::Format {
        self.swapchain.format.format
    }

    /// How finished images are handed to the screen (e.g. `FIFO`).
    pub fn present_mode(&self) -> vk::PresentModeKHR {
        self.swapchain.present_mode
    }

    /// Size of the swapchain images in physical pixels.
    pub fn extent(&self) -> (u32, u32) {
        (self.swapchain.extent.width, self.swapchain.extent.height)
    }

    /// How many images the swapchain actually created (may be more than asked for).
    pub fn image_count(&self) -> usize {
        self.swapchain.images.len()
    }

    /// The GPU's name as the driver reports it (e.g. "Apple M2").
    pub fn gpu_name(&self) -> String {
        let props = self.properties();
        props
            .device_name_as_c_str()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("<unnamed GPU>"))
    }

    /// The Vulkan version the GPU supports, as (major, minor, patch).
    pub fn api_version(&self) -> (u32, u32, u32) {
        let v = self.properties().api_version;
        (
            vk::api_version_major(v),
            vk::api_version_minor(v),
            vk::api_version_patch(v),
        )
    }

    /// Index of the queue family the queue came from.
    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    /// The queue that command buffers will be submitted to (§ 5).
    pub fn queue(&self) -> vk::Queue {
        self.queue
    }

    fn properties(&self) -> vk::PhysicalDeviceProperties {
        // SAFETY: `physical_device` came from this instance, which is still alive.
        unsafe {
            self.instance
                .get_physical_device_properties(self.physical_device)
        }
    }
}

impl Drop for GpuContext {
    fn drop(&mut self) {
        // SAFETY: mirror of creation order; children before parents; each object
        // is owned only by this struct, so each is destroyed exactly once.
        unsafe {
            self.swapchain.destroy(&self.device, &self.swapchain_loader);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
        destroy_instance_level(&self.instance, &self.debug);
    }
}

type DebugMessenger = (ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT);

/// Destroys the debug messenger (if any), then the instance.
/// Everything created from the device must already be gone.
fn destroy_instance_level(instance: &Instance, debug: &Option<DebugMessenger>) {
    // SAFETY: called once, after the device (if any) is destroyed; messenger before instance.
    unsafe {
        if let Some((loader, messenger)) = debug {
            loader.destroy_debug_utils_messenger(*messenger, None);
        }
        instance.destroy_instance(None);
    }
}

// --------------------------------------------------------------------- creation steps

/// Opens the Vulkan loader library (`libvulkan`).
///
/// On macOS, Homebrew and the LunarG SDK install it in folders the system doesn't
/// search by default, so if the normal lookup fails, try those folders by full path.
/// (Setting `DYLD_FALLBACK_LIBRARY_PATH` in `.cargo/config.toml` doesn't work:
/// `cargo run` overwrites that variable with its own value.)
fn load_entry() -> Result<Entry, GpuError> {
    // SAFETY: opens the system loader; the caller keeps the `Entry` alive
    // for as long as anything created through it.
    let default = unsafe { Entry::load() };

    #[cfg(target_os = "macos")]
    if default.is_err() {
        use std::path::PathBuf;
        let mut candidates = Vec::new();
        if let Some(sdk) = std::env::var_os("VULKAN_SDK") {
            candidates.push(PathBuf::from(sdk).join("lib/libvulkan.dylib"));
        }
        candidates.push(PathBuf::from("/opt/homebrew/lib/libvulkan.dylib"));
        for path in candidates {
            // SAFETY: as above, from an explicit path.
            if let Ok(entry) = unsafe { Entry::load_from(&path) } {
                return Ok(entry);
            }
        }
    }

    Ok(default?)
}

fn has_layer(entry: &Entry, name: &CStr) -> Result<bool, GpuError> {
    // SAFETY: plain query on a loaded entry.
    let layers = unsafe { entry.enumerate_instance_layer_properties()? };
    Ok(layers.iter().any(|l| l.layer_name_as_c_str() == Ok(name)))
}

fn has_instance_extension(entry: &Entry, name: &CStr) -> Result<bool, GpuError> {
    // SAFETY: plain query on a loaded entry.
    let exts = unsafe { entry.enumerate_instance_extension_properties(None)? };
    Ok(exts.iter().any(|e| e.extension_name_as_c_str() == Ok(name)))
}

fn create_instance(
    entry: &Entry,
    app_name: &CStr,
    window_extensions: &[*const c_char],
    validation: bool,
    portability: bool,
) -> Result<Instance, GpuError> {
    let app_info = vk::ApplicationInfo::default()
        .application_name(app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(c"engine")
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_2);

    // Pointers into 'static data (constants and c"" literals), so they never dangle.
    let mut extensions = window_extensions.to_vec();
    let mut flags = vk::InstanceCreateFlags::empty();
    if portability {
        extensions.push(khr::portability_enumeration::NAME.as_ptr());
        flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
    }
    if validation {
        extensions.push(ext::debug_utils::NAME.as_ptr());
    }
    let layers = if validation {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    // The real messenger only exists between instance creation and destruction.
    // Chaining this copy onto the create info also reports problems found *during*
    // `create_instance` and `destroy_instance` (e.g. "a child object was not destroyed").
    let mut debug_info = debug_messenger_info();
    let mut create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extensions)
        .enabled_layer_names(&layers)
        .flags(flags);
    if validation {
        create_info = create_info.push_next(&mut debug_info);
    }

    // SAFETY: every pointer in `create_info` points to data alive until this call returns.
    Ok(unsafe { entry.create_instance(&create_info, None)? })
}

fn create_debug_messenger(entry: &Entry, instance: &Instance) -> Result<DebugMessenger, GpuError> {
    let loader = ext::debug_utils::Instance::new(entry, instance);
    let info = debug_messenger_info();
    // SAFETY: the instance was created with VK_EXT_debug_utils enabled.
    let messenger = unsafe { loader.create_debug_utils_messenger(&info, None)? };
    Ok((loader, messenger))
}

/// What the messenger reports (warnings and errors only: MoltenVK's info is chatty)
/// and which function it calls.
fn debug_messenger_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(debug_callback))
}

/// Called by the validation layer (C code) whenever it has something to report.
unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _types: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let level = if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        "ERROR"
    } else {
        "WARNING"
    };
    // SAFETY: the layer passes a valid pointer for the duration of this call.
    let message = unsafe { data.as_ref() }
        .and_then(|d| unsafe { d.message_as_c_str() })
        .map(|m| m.to_string_lossy())
        .unwrap_or_default();
    eprintln!("[vulkan {level}] {message}");
    vk::FALSE // "don't abort the call that triggered this"
}

/// Returns the first GPU that supports the swapchain extension and has one queue
/// family that can both draw and present to `surface`, plus that family's index.
fn pick_physical_device(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, u32), GpuError> {
    // SAFETY: plain queries on a live instance and surface.
    let devices = unsafe { instance.enumerate_physical_devices()? };
    for device in devices {
        if !has_device_extension(instance, device, khr::swapchain::NAME)? {
            continue;
        }
        let families = unsafe { instance.get_physical_device_queue_family_properties(device) };
        for (index, family) in families.iter().enumerate() {
            let index = index as u32;
            let draws = family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let presents = unsafe {
                surface_loader.get_physical_device_surface_support(device, index, surface)?
            };
            if draws && presents {
                return Ok((device, index));
            }
        }
    }
    Err(GpuError::NoSuitableGpu)
}

fn has_device_extension(
    instance: &Instance,
    device: vk::PhysicalDevice,
    name: &CStr,
) -> Result<bool, GpuError> {
    // SAFETY: plain query on a live instance.
    let exts = unsafe { instance.enumerate_device_extension_properties(device)? };
    Ok(exts.iter().any(|e| e.extension_name_as_c_str() == Ok(name)))
}

fn create_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
) -> Result<Device, GpuError> {
    // Swapchain: checked by `pick_physical_device`. Portability subset: MoltenVK
    // lists it and requires it to be enabled if listed.
    let mut extensions = vec![khr::swapchain::NAME.as_ptr()];
    if has_device_extension(instance, physical_device, khr::portability_subset::NAME)? {
        extensions.push(khr::portability_subset::NAME.as_ptr());
    }

    let priorities = [1.0_f32];
    let queue_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities);
    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(std::slice::from_ref(&queue_info))
        .enabled_extension_names(&extensions);

    // SAFETY: every pointer in `create_info` points to data alive until this call returns.
    Ok(unsafe { instance.create_device(physical_device, &create_info, None)? })
}
