//! What every shader can read besides its vertices (§ 10): the camera on the
//! "noticeboard" (a uniform buffer per frame slot, reached through a descriptor set)
//! and each draw's model matrix on a "sticky note" (a push constant).
//!
//! One pipeline layout describes both, and every engine pipeline shares it.
//!
//! Shader side (must match):
//! ```glsl
//! layout(set = 0, binding = 0) uniform Camera { mat4 view_proj; vec4 light; vec4 eye; } camera;
//! layout(set = 0, binding = 1) readonly buffer Instances { mat4 instance[]; } instances;
//! layout(set = 0, binding = 2) uniform samplerCube environment;   // § 23
//! layout(push_constant) uniform Push { mat4 model; vec4 params; } push;
//! ```

use std::ptr;

use ash::{Device, vk};

use crate::buffer::Buffer;
use crate::gpu::GpuError;
use crate::maths::Mat4;

/// One `mat4` in std140: 16 floats, 64 bytes.
pub(crate) const MAT4_SIZE: usize = 64;

/// The noticeboard (§ 10, § 20, § 23): `view_proj` (bytes 0..64) + the light (64..80):
/// direction it shines in (xyz) and strength (w, 0 = off) + the camera's position
/// (80..96, w = 1), for reflections.
pub(crate) const CAMERA_SIZE: usize = MAT4_SIZE + 32;

/// The camera buffer's bytes, laid out like `{ mat4 view_proj; vec4 light; vec4 eye; }`.
pub(crate) fn camera_bytes(view_proj: Mat4, light: [f32; 4], eye: [f32; 4]) -> [u8; CAMERA_SIZE] {
    let mut bytes = [0u8; CAMERA_SIZE];
    bytes[..MAT4_SIZE].copy_from_slice(&mat4_bytes(view_proj));
    let floats = light.into_iter().chain(eye);
    for (chunk, value) in bytes[MAT4_SIZE..].chunks_exact_mut(4).zip(floats) {
        chunk.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

/// How many matrices one frame's instance list holds (§ 22). Position 0 is always the
/// identity, so instanced draws get `INSTANCE_CAPACITY − 1` between them.
pub(crate) const INSTANCE_CAPACITY: usize = 4096;
const INSTANCE_BYTES: usize = INSTANCE_CAPACITY * MAT4_SIZE;

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
    /// § 22: this slot's instance list (a storage buffer), persistently mapped.
    instances: Buffer,
    instances_mapped: *mut u8,
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
        // The noticeboard's shape: binding 0 = one uniform buffer, read by the vertex
        // shader (camera) and, since § 20, the fragment shader (light).
        // § 22: binding 1 = this slot's instance list, read by the vertex shader.
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(PUSH_STAGES),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
            // § 23: binding 2 = the environment cube map, read by fragment shaders
            // (the sky, chrome). Pointed at a cube by `set_environment`.
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];
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
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(slot_count as u32),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(slot_count as u32),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(slot_count as u32),
        ];

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
            CAMERA_SIZE as vk::DeviceSize,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        // § 22: the instance list — also rewritten every frame, so also no staging.
        let instances = match Buffer::new(
            device,
            memory_types,
            INSTANCE_BYTES as vk::DeviceSize,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        ) {
            Ok(b) => b,
            Err(e) => {
                // SAFETY: just created, never used by the GPU.
                unsafe { buffer.destroy(device) };
                return Err(e);
            }
        };
        let set_layouts = [self.set_layout];
        let alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(&set_layouts);

        // SAFETY: the memory is HOST_VISIBLE and mapped only here; the set comes from
        // our pool (freed with it); the buffer outlives the set (both go in `destroy`).
        unsafe {
            let setup = || -> Result<(*mut u8, *mut u8, vk::DescriptorSet), GpuError> {
                let mapped = buffer.map(device)?;
                let instances_mapped = instances.map(device)?;
                let set = device.allocate_descriptor_sets(&alloc)?[0];
                Ok((mapped, instances_mapped, set))
            };
            let (mapped, instances_mapped, set) = match setup() {
                Ok(s) => s,
                Err(e) => {
                    buffer.destroy(device);
                    instances.destroy(device);
                    return Err(e);
                }
            };

            // The descriptor write: "binding 0 of this set = this buffer, bytes 0..96".
            // Done once; afterwards only the buffer's contents change (Q-10.4).
            let buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(buffer.handle)
                .offset(0)
                .range(CAMERA_SIZE as vk::DeviceSize)];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_info);
            // § 22: binding 1 = the instance list, the whole buffer.
            let instances_info = [vk::DescriptorBufferInfo::default()
                .buffer(instances.handle)
                .offset(0)
                .range(INSTANCE_BYTES as vk::DeviceSize)];
            let instances_write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&instances_info);
            device.update_descriptor_sets(&[write, instances_write], &[]);

            let camera = CameraSlot {
                buffer,
                mapped,
                instances,
                instances_mapped,
                set,
            };
            // Position 0: the identity, read by every ordinary draw — written once,
            // never overwritten (instanced draws start at 1, Q-22.3).
            let identity = mat4_bytes(Mat4::IDENTITY);
            ptr::copy_nonoverlapping(identity.as_ptr(), camera.instances_mapped, MAT4_SIZE);
            // Valid even if the app never sets one: identity camera, light off.
            write_camera_bytes(
                camera.mapped,
                Mat4::IDENTITY,
                [0.0; 4],
                [0.0, 0.0, 0.0, 1.0],
            );
            Ok(camera)
        }
    }

    /// Writes `view_proj` and the light into slot `slot`'s camera buffer.
    ///
    /// # Safety
    /// The caller must have waited on that slot's fence: the GPU may still be
    /// reading the buffer otherwise (Q-10.2).
    pub(crate) unsafe fn write_camera(
        &self,
        slot: usize,
        view_proj: Mat4,
        light: [f32; 4],
        eye: [f32; 4],
    ) {
        // SAFETY: `mapped` points at 96 writable bytes; the caller guarantees the GPU isn't reading them.
        unsafe { write_camera_bytes(self.cameras[slot].mapped, view_proj, light, eye) };
    }

    /// § 23: points binding 2 of every slot's set at the cube `view` (the environment).
    ///
    /// # Safety
    /// No submitted frame may be using the sets: call after `device_wait_idle`, or
    /// before the first frame.
    pub(crate) unsafe fn set_environment(
        &self,
        device: &Device,
        view: vk::ImageView,
        sampler: vk::Sampler,
    ) {
        let image_info = [vk::DescriptorImageInfo::default()
            .sampler(sampler)
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let writes: Vec<_> = self
            .cameras
            .iter()
            .map(|camera| {
                vk::WriteDescriptorSet::default()
                    .dst_set(camera.set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&image_info)
            })
            .collect();
        // SAFETY: guaranteed by the caller.
        unsafe { device.update_descriptor_sets(&writes, &[]) };
    }

    /// Writes this frame's instance matrices into slot `slot`'s list, from position 1
    /// on (position 0 stays the identity). At most `INSTANCE_CAPACITY − 1`: `Frame`
    /// already stops there.
    ///
    /// # Safety
    /// As `write_camera`: the caller must have waited on that slot's fence.
    pub(crate) unsafe fn write_instances(&self, slot: usize, matrices: &[Mat4]) {
        let count = matrices.len().min(INSTANCE_CAPACITY - 1);
        let base = self.cameras[slot].instances_mapped;
        for (i, m) in matrices[..count].iter().enumerate() {
            let bytes = mat4_bytes(*m);
            // SAFETY: (1 + i) < INSTANCE_CAPACITY, so the 64 bytes are inside the mapped
            // buffer; the caller guarantees the GPU isn't reading it.
            unsafe {
                ptr::copy_nonoverlapping(bytes.as_ptr(), base.add((1 + i) * MAT4_SIZE), MAT4_SIZE)
            };
        }
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
                camera.instances.destroy(device);
            }
            device.destroy_descriptor_pool(self.pool, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.texture_set_layout, None);
            device.destroy_descriptor_set_layout(self.set_layout, None);
        }
    }
}

/// Copies the camera's 96 bytes to `dst`.
///
/// # Safety
/// `dst` must point at 96 writable bytes nobody else is using.
unsafe fn write_camera_bytes(dst: *mut u8, view_proj: Mat4, light: [f32; 4], eye: [f32; 4]) {
    let bytes = camera_bytes(view_proj, light, eye);
    // SAFETY: guaranteed by the caller; `bytes` is a local, so no overlap.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), dst, CAMERA_SIZE) };
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
    fn camera_bytes_put_the_light_then_the_eye_after_the_matrix() {
        let bytes = camera_bytes(Mat4::IDENTITY, [0.5, -1.0, 0.25, 1.0], [3.0, 4.0, 5.0, 1.0]);
        assert_eq!(bytes.len(), 96);
        assert_eq!(bytes[..64], mat4_bytes(Mat4::IDENTITY));
        let float_at = |i: usize| f32::from_ne_bytes(bytes[i..i + 4].try_into().unwrap());
        assert_eq!(
            [float_at(64), float_at(68), float_at(72), float_at(76)],
            [0.5, -1.0, 0.25, 1.0]
        );
        // § 23: the camera's position, after the light.
        assert_eq!(
            [float_at(80), float_at(84), float_at(88), float_at(92)],
            [3.0, 4.0, 5.0, 1.0]
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
