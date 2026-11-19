//! What every shader can read besides its vertices (§ 10): the camera on the
//! "noticeboard" (a uniform buffer per frame slot, reached through a descriptor set)
//! and each draw's model matrix on a "sticky note" (a push constant).
//!
//! One pipeline layout describes both, and every engine pipeline shares it.
//!
//! Shader side (must match):
//! ```glsl
//! layout(set = 0, binding = 0) uniform Camera { mat4 view_proj; } camera;
//! layout(push_constant) uniform Push { mat4 model; vec4 params; } push;
//! ```

use std::ptr;

use ash::{Device, vk};

use crate::buffer::Buffer;
use crate::gpu::GpuError;
use crate::maths::Mat4;

/// One `mat4` in std140: 16 floats, 64 bytes.
pub(crate) const MAT4_SIZE: usize = 64;

/// The sticky note (§ 10, § 15, § 19): model matrix (bytes 0..64) + four numbers whose
/// meaning the app decides (64..80). 80 ≤ 128, the size every GPU must allow.
pub(crate) const PUSH_SIZE: usize = MAT4_SIZE + 16;
/// Both shaders read the push block: the vertex shader the model, the fragment
/// shader the params (Q-15.4: a stage missing here fails at pipeline creation).
pub(crate) const PUSH_STAGES: vk::ShaderStageFlags = vk::ShaderStageFlags::from_raw(
    vk::ShaderStageFlags::VERTEX.as_raw() | vk::ShaderStageFlags::FRAGMENT.as_raw(),
);

/// One draw's push-constant bytes: the model matrix, then the four params,
/// laid out like the GLSL block `{ mat4 model; vec4 params; }`.
pub(crate) fn push_bytes(model: Mat4, params: [f32; 4]) -> [u8; PUSH_SIZE] {
    let mut bytes = [0u8; PUSH_SIZE];
    bytes[..MAT4_SIZE].copy_from_slice(&mat4_bytes(model));
    for (chunk, value) in bytes[MAT4_SIZE..].chunks_exact_mut(4).zip(params) {
        chunk.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

/// A `Mat4` as the 64 bytes the GPU reads, column by column (std140 = `Mat4`'s layout).
pub(crate) fn mat4_bytes(m: Mat4) -> [u8; MAT4_SIZE] {
    let mut bytes = [0u8; MAT4_SIZE];
    let floats = m.to_cols_array().into_iter().flatten();
    for (chunk, value) in bytes.chunks_exact_mut(4).zip(floats) {
        chunk.copy_from_slice(&value.to_ne_bytes()); // the GPU reads the CPU's byte order
    }
    bytes
}

/// One frame slot's noticeboard: the buffer, the CPU pointer into it, and the
/// descriptor set that points the shader at it.
struct CameraSlot {
    buffer: Buffer,
    mapped: *mut u8,
    set: vk::DescriptorSet,
}

pub(crate) struct ShaderInputs {
    /// The shared "function signature" for every pipeline's extra inputs.
    pub(crate) pipeline_layout: vk::PipelineLayout,
    set_layout: vk::DescriptorSetLayout,
    /// Set 1's shape (§ 14): binding 0 = one combined image sampler, fragment stage.
    /// Owned here because the pipeline layout is built from it; textures use it.
    pub(crate) texture_set_layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    cameras: Vec<CameraSlot>,
}

impl ShaderInputs {
    /// Creates the layouts, the descriptor pool, and one camera buffer + descriptor
    /// set per frame slot (`slot_count`). Each camera starts as the identity matrix.
    pub(crate) fn new(
        device: &Device,
        memory_types: &vk::PhysicalDeviceMemoryProperties,
        slot_count: usize,
    ) -> Result<Self, GpuError> {
        // The noticeboard's shape: binding 0 = one uniform buffer, read by the vertex shader.
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)];
        // Set 1 (§ 14): one picture + its sampler, read by the fragment shader.
        let texture_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];
        // The sticky note: model + four params, read by both shaders (§ 15, § 19).
        let push_ranges = [vk::PushConstantRange::default()
            .stage_flags(PUSH_STAGES)
            .offset(0)
            .size(PUSH_SIZE as u32)];
        // Room for one set per slot, each holding one uniform-buffer descriptor.
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(slot_count as u32)];

        // SAFETY: plain creation on a live device. Until `inputs` exists, each failure
        // destroys by hand what was already made; after that, `destroy` does it.
        unsafe {
            let set_layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                None,
            )?;
            let texture_set_layout = match device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&texture_bindings),
                None,
            ) {
                Ok(l) => l,
                Err(e) => {
                    device.destroy_descriptor_set_layout(set_layout, None);
                    return Err(e.into());
                }
            };
            // Set 0 = camera, set 1 = texture: the order here is the `set = N` in GLSL.
            let set_layouts = [set_layout, texture_set_layout];
            let pipeline_layout = match device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&set_layouts)
                    .push_constant_ranges(&push_ranges),
                None,
            ) {
                Ok(l) => l,
                Err(e) => {
                    device.destroy_descriptor_set_layout(texture_set_layout, None);
                    device.destroy_descriptor_set_layout(set_layout, None);
                    return Err(e.into());
                }
            };
            let pool = match device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets(slot_count as u32)
                    .pool_sizes(&pool_sizes),
                None,
            ) {
                Ok(p) => p,
                Err(e) => {
                    device.destroy_pipeline_layout(pipeline_layout, None);
                    device.destroy_descriptor_set_layout(texture_set_layout, None);
                    device.destroy_descriptor_set_layout(set_layout, None);
                    return Err(e.into());
                }
            };

            let mut inputs = ShaderInputs {
                pipeline_layout,
                set_layout,
                texture_set_layout,
                pool,
                cameras: Vec::with_capacity(slot_count),
            };
            for _ in 0..slot_count {
                match inputs.create_camera(device, memory_types) {
                    Ok(camera) => inputs.cameras.push(camera),
                    Err(e) => {
                        // Nothing has been submitted yet, so nothing is in use.
                        inputs.destroy(device);
                        return Err(e);
                    }
                }
            }
            Ok(inputs)
        }
    }

    /// One slot: a persistently mapped uniform buffer, and a descriptor set
    /// written once to point at it.
    fn create_camera(
        &self,
        device: &Device,
        memory_types: &vk::PhysicalDeviceMemoryProperties,
    ) -> Result<CameraSlot, GpuError> {
        // Per-frame data: written by the CPU every frame, so HOST_VISIBLE (no staging),
        // and HOST_COHERENT (writes reach the GPU without a flush).
        let buffer = Buffer::new(
            device,
            memory_types,
            MAT4_SIZE as vk::DeviceSize,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let set_layouts = [self.set_layout];
        let alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(&set_layouts);

        // SAFETY: the memory is HOST_VISIBLE and mapped only here; the set comes from
        // our pool (freed with it); the buffer outlives the set (both go in `destroy`).
        unsafe {
            let setup = || -> Result<(*mut u8, vk::DescriptorSet), GpuError> {
                let mapped = buffer.map(device)?;
                let set = device.allocate_descriptor_sets(&alloc)?[0];
                Ok((mapped, set))
            };
            let (mapped, set) = match setup() {
                Ok(s) => s,
                Err(e) => {
                    buffer.destroy(device);
                    return Err(e);
                }
            };

            // The descriptor write: "binding 0 of this set = this buffer, bytes 0..64".
            // Done once; afterwards only the buffer's contents change (Q-10.4).
            let buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(buffer.handle)
                .offset(0)
                .range(MAT4_SIZE as vk::DeviceSize)];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_info);
            device.update_descriptor_sets(&[write], &[]);

            let camera = CameraSlot {
                buffer,
                mapped,
                set,
            };
            write_mat4(camera.mapped, Mat4::IDENTITY); // valid even if the app never sets one
            Ok(camera)
        }
    }

    /// Writes `view_proj` into slot `slot`'s camera buffer.
    ///
    /// # Safety
    /// The caller must have waited on that slot's fence: the GPU may still be
    /// reading the buffer otherwise (Q-10.2).
    pub(crate) unsafe fn write_camera(&self, slot: usize, view_proj: Mat4) {
        // SAFETY: `mapped` points at 64 writable bytes; the caller guarantees the GPU isn't reading them.
        unsafe { write_mat4(self.cameras[slot].mapped, view_proj) };
    }

    /// The descriptor set pointing at slot `slot`'s camera buffer.
    pub(crate) fn camera_set(&self, slot: usize) -> vk::DescriptorSet {
        self.cameras[slot].set
    }

    /// Destroys the buffers, then the pool (which frees its sets), then the layouts.
    ///
    /// # Safety
    /// Call exactly once, after the GPU has finished all work using these objects
    /// (after `device_wait_idle`), and after every pipeline using the layout is gone.
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe {
            for camera in &self.cameras {
                camera.buffer.destroy(device); // freeing the memory also unmaps it
            }
            device.destroy_descriptor_pool(self.pool, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.texture_set_layout, None);
            device.destroy_descriptor_set_layout(self.set_layout, None);
        }
    }
}

/// Copies a matrix's 64 bytes to `dst`.
///
/// # Safety
/// `dst` must point at 64 writable bytes nobody else is using.
unsafe fn write_mat4(dst: *mut u8, m: Mat4) {
    let bytes = mat4_bytes(m);
    // SAFETY: guaranteed by the caller; `bytes` is a local, so no overlap.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), dst, MAT4_SIZE) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maths::Vec3;

    #[test]
    fn push_bytes_put_params_after_the_matrix() {
        let bytes = push_bytes(Mat4::IDENTITY, [0.25, 2.0, 3.5, 9.0]);
        assert_eq!(bytes.len(), 80);
        assert_eq!(bytes[..64], mat4_bytes(Mat4::IDENTITY));
        let float_at = |i: usize| f32::from_ne_bytes(bytes[i..i + 4].try_into().unwrap());
        assert_eq!(
            [float_at(64), float_at(68), float_at(72), float_at(76)],
            [0.25, 2.0, 3.5, 9.0]
        );
    }

    #[test]
    fn mat4_bytes_are_column_by_column() {
        let bytes = mat4_bytes(Mat4::translation(Vec3::new(5.0, 6.0, 7.0)));
        let float_at = |i: usize| f32::from_ne_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        // Column 3 (floats 12..16) holds the move, as GLSL expects.
        assert_eq!(
            [float_at(12), float_at(13), float_at(14), float_at(15)],
            [5.0, 6.0, 7.0, 1.0]
        );
        assert_eq!(float_at(0), 1.0); // top-left of the identity part
    }
}
