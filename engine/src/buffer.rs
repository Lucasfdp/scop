//! GPU buffers: a `VkBuffer` plus the `VkDeviceMemory` it lives in, and uploading
//! data into device-local memory through a staging buffer (§ 7).

use std::mem::size_of_val;
use std::ptr;

use ash::{Device, vk};

use crate::gpu::GpuError;

/// A buffer and its memory, owned together and destroyed together.
pub(crate) struct Buffer {
    pub(crate) handle: vk::Buffer,
    memory: vk::DeviceMemory,
}

impl Buffer {
    /// Creates a buffer of `size` bytes for `usage`, in memory with `properties`.
    pub(crate) fn new(
        device: &Device,
        memory_types: &vk::PhysicalDeviceMemoryProperties,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<Self, GpuError> {
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        // SAFETY: plain creation on a live device; each failure undoes what exists.
        unsafe {
            let handle = device.create_buffer(&info, None)?;

            // What does this buffer really need? (size may be more than we asked for)
            let needs = device.get_buffer_memory_requirements(handle);
            let Some(type_index) =
                find_memory_type(memory_types, needs.memory_type_bits, properties)
            else {
                device.destroy_buffer(handle, None);
                return Err(GpuError::Unsupported("no suitable GPU memory type"));
            };

            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size) // the *required* size, not `size`
                .memory_type_index(type_index);
            let memory = match device.allocate_memory(&alloc, None) {
                Ok(m) => m,
                Err(e) => {
                    device.destroy_buffer(handle, None);
                    return Err(e.into());
                }
            };

            if let Err(e) = device.bind_buffer_memory(handle, memory, 0) {
                device.destroy_buffer(handle, None);
                device.free_memory(memory, None);
                return Err(e.into());
            }
            Ok(Buffer { handle, memory })
        }
    }

    /// Maps the whole buffer and returns a CPU pointer to its first byte.
    /// Never unmapped: freeing the memory (in `destroy`) unmaps it (§ 10:
    /// *persistently mapped*).
    ///
    /// # Safety
    /// The buffer's memory must be `HOST_VISIBLE`; call at most once per buffer.
    pub(crate) unsafe fn map(&self, device: &Device) -> Result<*mut u8, GpuError> {
        // SAFETY: the caller guarantees HOST_VISIBLE memory, mapped only once.
        let ptr = unsafe {
            device.map_memory(self.memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())?
        };
        Ok(ptr.cast::<u8>())
    }

    /// Destroys the buffer, then frees its memory.
    ///
    /// # Safety
    /// Call exactly once, when the GPU has finished all work using the buffer.
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.handle, None);
            device.free_memory(self.memory, None);
        }
    }
}

/// Index of the first memory type this buffer may use (bit set in `allowed`)
/// that has every flag in `wanted`.
pub(crate) fn find_memory_type(
    memory_types: &vk::PhysicalDeviceMemoryProperties,
    allowed: u32,
    wanted: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let count = memory_types.memory_type_count as usize;
    memory_types.memory_types[..count]
        .iter()
        .enumerate()
        .find(|(i, kind)| allowed & (1 << i) != 0 && kind.property_flags.contains(wanted))
        .map(|(i, _)| i as u32)
}

/// What an upload needs from the GPU context.
pub(crate) struct Uploader<'a> {
    pub(crate) device: &'a Device,
    pub(crate) memory_types: &'a vk::PhysicalDeviceMemoryProperties,
    pub(crate) pool: vk::CommandPool,
    pub(crate) queue: vk::Queue,
}

impl Uploader<'_> {
    /// Copies `data` into a new device-local buffer for `usage`, through a staging
    /// buffer. Waits until the copy has finished: load-time only, never per frame.
    pub(crate) fn upload<T: Copy>(
        &self,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer, GpuError> {
        let size = size_of_val(data) as vk::DeviceSize;
        if size == 0 {
            return Err(GpuError::Unsupported(
                "empty buffer (no vertices or indices)",
            ));
        }

        // 1. The loading dock: CPU-writable, writes visible to the GPU without a flush.
        let staging = Buffer::new(
            self.device,
            self.memory_types,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let result = self.fill(&staging, data, size).and_then(|()| {
            // 2. The warehouse: fast for the GPU, filled by a copy.
            let target = Buffer::new(
                self.device,
                self.memory_types,
                size,
                usage | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            // 3. The forklift: copy, and wait for it.
            match self.copy(&staging, &target, size) {
                Ok(()) => Ok(target),
                Err(e) => {
                    // SAFETY: the copy failed or finished; nothing else uses `target`.
                    unsafe { target.destroy(self.device) };
                    Err(e)
                }
            }
        });

        // SAFETY: `copy` waited for the GPU (or never submitted), so the staging
        // buffer is no longer in use.
        unsafe { staging.destroy(self.device) };
        result
    }

    /// Maps the staging buffer and copies `data` into it (the CPU's `memcpy`).
    fn fill<T: Copy>(
        &self,
        staging: &Buffer,
        data: &[T],
        size: vk::DeviceSize,
    ) -> Result<(), GpuError> {
        // SAFETY: the memory is HOST_VISIBLE and at least `size` bytes; the source
        // and destination don't overlap; we unmap before anything else uses it.
        unsafe {
            let mapped =
                self.device
                    .map_memory(staging.memory, 0, size, vk::MemoryMapFlags::empty())?;
            ptr::copy_nonoverlapping(
                data.as_ptr().cast::<u8>(),
                mapped.cast::<u8>(),
                size as usize,
            );
            self.device.unmap_memory(staging.memory);
        }
        Ok(())
    }

    /// Records and submits a one-off copy, then waits for it on a fence.
    fn copy(&self, from: &Buffer, to: &Buffer, size: vk::DeviceSize) -> Result<(), GpuError> {
        let alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        };
        // Make the copy's writes visible to the vertex/index reads of later frames.
        let written_for_drawing = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ | vk::AccessFlags::INDEX_READ);

        // SAFETY: the pool belongs to this queue's family; the command buffer and
        // fence are freed before returning, after the wait.
        unsafe {
            let command_buffer = self.device.allocate_command_buffers(&alloc)?[0];
            let fence = match self
                .device
                .create_fence(&vk::FenceCreateInfo::default(), None)
            {
                Ok(f) => f,
                Err(e) => {
                    self.device
                        .free_command_buffers(self.pool, &[command_buffer]);
                    return Err(e.into());
                }
            };

            let run = || -> Result<(), vk::Result> {
                self.device.begin_command_buffer(command_buffer, &begin)?;
                self.device
                    .cmd_copy_buffer(command_buffer, from.handle, to.handle, &[region]);
                self.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::VERTEX_INPUT,
                    vk::DependencyFlags::empty(),
                    &[written_for_drawing],
                    &[],
                    &[],
                );
                self.device.end_command_buffer(command_buffer)?;
                let command_buffers = [command_buffer];
                let submit = vk::SubmitInfo::default().command_buffers(&command_buffers);
                self.device.queue_submit(self.queue, &[submit], fence)?;
                // The submit returned at once; the copy runs later. Wait for it.
                self.device.wait_for_fences(&[fence], true, u64::MAX)
            };
            let result = run();

            self.device.destroy_fence(fence, None);
            self.device
                .free_command_buffers(self.pool, &[command_buffer]);
            result.map_err(GpuError::from)
        }
    }
}
