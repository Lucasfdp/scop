//! GPU context: the Vulkan loader, instance, debug messenger, window surface,
//! logical device and swapchain.
//!
//! Creation order: loader → instance → debug messenger → surface → physical device
//! (chosen) → logical device → queue (fetched) → swapchain → image views → frame slots.
//! Pipelines and meshes are added later, by the app, through `Gfx` (§ 8).
//! `Drop` waits for the GPU to go idle, then destroys in the mirror order.

use std::ffi::{CStr, c_char, c_void};
use std::fmt;

use ash::{Device, Entry, Instance, ext, khr, vk};
use raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};

use crate::app::{Frame, GpuInfo, MeshId, PipelineId};
use crate::buffer::Uploader;
use crate::frames::{self, FrameTarget, Frames};
use crate::mesh::{Mesh, MeshData};
use crate::pipeline::{Pipeline, Shaders};
use crate::spirv::SpirvError;
use crate::swapchain::Swapchain;

const VALIDATION_LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";

/// Everything that can go wrong while setting up the GPU.
#[derive(Debug)]
pub(crate) enum GpuError {
    /// The Vulkan loader library (`libvulkan`) could not be opened.
    Load(ash::LoadingError),
    /// A Vulkan call returned an error code.
    Vulkan(vk::Result),
    /// No GPU has a queue family that can draw *and* present to the window.
    NoSuitableGpu,
    /// The window couldn't hand over its platform handles.
    WindowHandle(HandleError),
    /// The GPU or surface lacks something scop needs.
    Unsupported(&'static str),
    /// Shader bytes that aren't valid SPIR-V.
    Spirv(SpirvError),
    /// A draw request named a pipeline or mesh this context never created.
    UnknownHandle,
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
            GpuError::Unsupported(what) => write!(f, "unsupported: {what}"),
            GpuError::Spirv(e) => write!(f, "bad shader: {e}"),
            GpuError::UnknownHandle => write!(f, "draw request with an unknown pipeline or mesh"),
        }
    }
}

impl std::error::Error for GpuError {}

impl From<ash::LoadingError> for GpuError {
    fn from(e: ash::LoadingError) -> Self {
        GpuError::Load(e)
    }
}

impl From<SpirvError> for GpuError {
    fn from(e: SpirvError) -> Self {
        GpuError::Spirv(e)
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
pub(crate) struct GpuContext {
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
    rendering_loader: khr::dynamic_rendering::Device,
    memory_types: vk::PhysicalDeviceMemoryProperties,
    frames: Frames,
    // Owned here, handed out to the app as `PipelineId(i)` / `MeshId(i)` (§ 8).
    pipelines: Vec<Pipeline>,
    meshes: Vec<Mesh>,
}

/// What happened when a frame was drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameStatus {
    /// Drawn and shown.
    Presented,
    /// Drawn and shown, but the swapchain no longer matches the window exactly.
    Suboptimal,
    /// Skipped: the swapchain no longer matches the window. Rebuilding comes in § 16.
    OutOfDate,
}

impl GpuContext {
    /// Sets up Vulkan for drawing into `window`.
    ///
    /// `size` is the window's size in **physical** pixels (not logical points).
    /// The caller must drop the returned context before dropping the window.
    pub(crate) fn new(
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
            pick_physical_device(&instance, &surface_loader, surface).map_err(fail)?;
        let device =
            create_device(&instance, physical_device, queue_family_index).map_err(fail)?;

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

        // On Vulkan 1.2 the dynamic rendering functions come from the extension's
        // own function table, not from `Device`.
        let rendering_loader = khr::dynamic_rendering::Device::new(&instance, &device);
        let frames = Frames::new(&device, queue_family_index).map_err(|e| {
            // SAFETY: Frames cleaned up after itself; nothing has been submitted yet.
            unsafe {
                swapchain.destroy(&device, &swapchain_loader);
                device.destroy_device(None);
            }
            fail(e)
        })?;

        // Which kinds of memory this GPU has (§ 7: buffers pick from these).
        // SAFETY: plain query on a live instance.
        let memory_types =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

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
            rendering_loader,
            memory_types,
            frames,
            pipelines: Vec::new(),
            meshes: Vec::new(),
        })
    }

    /// Builds a pipeline for the swapchain's format and returns its ticket.
    pub(crate) fn create_pipeline(&mut self, shaders: Shaders) -> Result<PipelineId, GpuError> {
        let pipeline = Pipeline::new(&self.device, self.swapchain.format.format, shaders)?;
        self.pipelines.push(pipeline);
        Ok(PipelineId(self.pipelines.len() - 1))
    }

    /// Uploads a mesh to device-local memory and returns its ticket.
    pub(crate) fn create_mesh(&mut self, data: &MeshData) -> Result<MeshId, GpuError> {
        let uploader = Uploader {
            device: &self.device,
            memory_types: &self.memory_types,
            pool: self.frames.pool(),
            queue: self.queue,
        };
        let mesh = Mesh::upload(&uploader, data)?;
        self.meshes.push(mesh);
        Ok(MeshId(self.meshes.len() - 1))
    }

    /// Draws one frame from the app's draw list: clears the next swapchain image,
    /// draws each requested mesh, and shows it. The § 5 loop, in order:
    /// wait fence → acquire → reset fence → record → submit → present.
    pub(crate) fn draw_frame(&mut self, frame: &Frame) -> Result<FrameStatus, GpuError> {
        // Turn tickets into the real objects before touching the GPU, so a bad
        // ticket fails cleanly instead of half-way through a frame.
        let mut draws = Vec::with_capacity(frame.draws.len());
        for &(pipeline, mesh) in &frame.draws {
            match (self.pipelines.get(pipeline.0), self.meshes.get(mesh.0)) {
                (Some(p), Some(m)) => draws.push((p.handle, m)),
                _ => return Err(GpuError::UnknownHandle),
            }
        }

        let slot = self.frames.current();

        // SAFETY: every handle below belongs to this context and is alive; the fence
        // wait guarantees the GPU has finished with this slot's command buffer.
        unsafe {
            // 1. Wait until the GPU has finished the work last submitted from this slot.
            self.device.wait_for_fences(&[slot.fence], true, u64::MAX)?;

            // 2. Which image do we draw into? `image_free` is raised once it's off screen.
            let acquired = self.swapchain_loader.acquire_next_image(
                self.swapchain.handle,
                u64::MAX,
                slot.image_free,
                vk::Fence::null(),
            );
            let (image_index, acquire_suboptimal) = match acquired {
                Ok(result) => result,
                // Return *before* resetting the fence, or this slot would wait forever.
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(FrameStatus::OutOfDate),
                Err(e) => return Err(e.into()),
            };
            let target = FrameTarget {
                image: self.swapchain.images[image_index as usize],
                view: self.swapchain.views[image_index as usize],
                extent: self.swapchain.extent,
            };
            let render_done = self.swapchain.render_done[image_index as usize];

            // 3. Acquire succeeded, so we're definitely submitting: switch the light off.
            self.device.reset_fences(&[slot.fence])?;

            // 4. Write the instruction sheet.
            frames::record_frame(
                &self.device,
                &self.rendering_loader,
                slot.command_buffer,
                &target,
                &draws,
                frame.clear_colour,
            )?;

            // 5. Hand it over: wait for `image_free` before the first write to the
            //    image (COLOR_ATTACHMENT_OUTPUT stage); when finished, raise
            //    `render_done` and switch the fence on.
            let wait_semaphores = [slot.image_free];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [slot.command_buffer];
            let signal_semaphores = [render_done];
            let submit = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);
            self.device
                .queue_submit(self.queue, &[submit], slot.fence)?;

            // 6. Ask for it to be shown once `render_done` is raised.
            let swapchains = [self.swapchain.handle];
            let image_indices = [image_index];
            let present = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            let presented = self.swapchain_loader.queue_present(self.queue, &present);

            // The work is submitted whatever present said, so move on to the next slot.
            self.frames.advance();

            match presented {
                Ok(false) if !acquire_suboptimal => Ok(FrameStatus::Presented),
                Ok(_) => Ok(FrameStatus::Suboptimal),
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(FrameStatus::OutOfDate),
                Err(e) => Err(e.into()),
            }
        }
    }
    /// Facts about the GPU and swapchain, as plain values for the app to print.
    pub(crate) fn info(&self) -> GpuInfo {
        let props = self.properties();
        let v = props.api_version;
        GpuInfo {
            name: props
                .device_name_as_c_str()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|_| String::from("<unnamed GPU>")),
            vulkan_version: (
                vk::api_version_major(v),
                vk::api_version_minor(v),
                vk::api_version_patch(v),
            ),
            queue_family: self.queue_family_index,
            swapchain_format: format!("{:?}", self.swapchain.format.format),
            present_mode: format!("{:?}", self.swapchain.present_mode),
            extent: (self.swapchain.extent.width, self.swapchain.extent.height),
            image_count: self.swapchain.images.len(),
        }
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
            // Nothing may be destroyed while the GPU might still be using it. If the
            // wait itself fails, the device is lost and there's nothing left to wait for.
            let _ = self.device.device_wait_idle();
            for mesh in &self.meshes {
                mesh.destroy(&self.device);
            }
            for pipeline in &self.pipelines {
                pipeline.destroy(&self.device);
            }
            self.frames.destroy(&self.device);
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

/// Returns the first GPU that supports the swapchain and dynamic rendering, and has
/// one queue family that can both draw and present to `surface`, plus that family's index.
fn pick_physical_device(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, u32), GpuError> {
    // SAFETY: plain queries on a live instance and surface.
    let devices = unsafe { instance.enumerate_physical_devices()? };
    let mut lacks_dynamic_rendering = false;
    for device in devices {
        if !has_device_extension(instance, device, khr::swapchain::NAME)? {
            continue;
        }
        if !supports_dynamic_rendering(instance, device)? {
            lacks_dynamic_rendering = true;
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
    if lacks_dynamic_rendering {
        return Err(GpuError::Unsupported(
            "VK_KHR_dynamic_rendering (the GPU driver may be too old)",
        ));
    }
    Err(GpuError::NoSuitableGpu)
}

/// Dynamic rendering needs both: the extension listed, and its feature switch supported.
fn supports_dynamic_rendering(
    instance: &Instance,
    device: vk::PhysicalDevice,
) -> Result<bool, GpuError> {
    if !has_device_extension(instance, device, khr::dynamic_rendering::NAME)? {
        return Ok(false);
    }
    // Ask for the feature by chaining its struct onto the query (the pNext idea from § 3).
    let mut dynamic_rendering = vk::PhysicalDeviceDynamicRenderingFeatures::default();
    let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut dynamic_rendering);
    // SAFETY: plain query on a live instance; the chained struct outlives the call.
    unsafe { instance.get_physical_device_features2(device, &mut features) };
    Ok(dynamic_rendering.dynamic_rendering == vk::TRUE)
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
    // Swapchain and dynamic rendering: checked by `pick_physical_device`.
    // Portability subset: MoltenVK lists it and requires it to be enabled if listed.
    let mut extensions = vec![
        khr::swapchain::NAME.as_ptr(),
        khr::dynamic_rendering::NAME.as_ptr(),
    ];
    if has_device_extension(instance, physical_device, khr::portability_subset::NAME)? {
        extensions.push(khr::portability_subset::NAME.as_ptr());
    }

    let priorities = [1.0_f32];
    let queue_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities);
    // Enabling the extension adds its functions; the feature switch must also be on.
    let mut dynamic_rendering =
        vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);
    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(std::slice::from_ref(&queue_info))
        .enabled_extension_names(&extensions)
        .push_next(&mut dynamic_rendering);

    // SAFETY: every pointer in `create_info` points to data alive until this call returns.
    Ok(unsafe { instance.create_device(physical_device, &create_info, None)? })
}
