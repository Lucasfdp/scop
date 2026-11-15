//! Meshes: the vertex layout the GPU reads, and vertex + index buffers in GPU memory.

use std::mem::{offset_of, size_of};

use ash::{Device, vk};

use crate::buffer::{Buffer, Uploader};
use crate::gpu::GpuError;

/// One corner, exactly as it sits in the vertex buffer.
///
/// `#[repr(C)]`: fields in the order written, C padding rules — a layout the
/// pipeline's vertex input description can rely on. `Copy` is fine: a vertex is
/// plain numbers and owns nothing.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    /// x, y, z in the model's own space (§ 10: corners are 3D now).
    pub position: [f32; 3],
    /// Linear RGB, 0.0–1.0.
    pub colour: [f32; 3],
    /// Where this corner sits on the texture (§ 14): U across, V down, 0.0–1.0.
    pub uv: [f32; 2],
}

impl Vertex {
    /// Binding 0: one `Vertex` per corner, `size_of::<Vertex>()` bytes apart.
    pub(crate) fn binding() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    /// The packing list: which bytes of a vertex feed which shader `location`.
    pub(crate) fn attributes() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT, // vec3
                offset: offset_of!(Vertex, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT, // vec3
                offset: offset_of!(Vertex, colour) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT, // vec2
                offset: offset_of!(Vertex, uv) as u32,
            },
        ]
    }
}

/// Mesh data on the CPU side, as the app provides it (and § 12's `.obj` loader will).
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    /// Every 3 indices make one triangle; each is an index into `vertices`.
    pub indices: Vec<u32>,
}

/// A mesh in GPU memory, ready to draw.
pub(crate) struct Mesh {
    pub(crate) vertices: Buffer,
    pub(crate) indices: Buffer,
    pub(crate) index_count: u32,
}

impl Mesh {
    /// Uploads both buffers through staging buffers into device-local memory.
    pub(crate) fn upload(uploader: &Uploader, data: &MeshData) -> Result<Self, GpuError> {
        let vertices = uploader.upload(&data.vertices, vk::BufferUsageFlags::VERTEX_BUFFER)?;
        let indices = match uploader.upload(&data.indices, vk::BufferUsageFlags::INDEX_BUFFER) {
            Ok(b) => b,
            Err(e) => {
                // SAFETY: the upload waited, so the vertex buffer isn't in use.
                unsafe { vertices.destroy(uploader.device) };
                return Err(e);
            }
        };
        Ok(Mesh {
            vertices,
            indices,
            index_count: data.indices.len() as u32,
        })
    }

    /// # Safety
    /// Call exactly once, after the GPU has finished all work using the mesh.
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe {
            self.indices.destroy(device);
            self.vertices.destroy(device);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_layout_matches_the_description() {
        // 3 + 3 + 2 floats, no padding: 32 bytes; colour at byte 12, uv at byte 24.
        assert_eq!(Vertex::binding().stride, 32);
        let [position, colour, uv] = Vertex::attributes();
        assert_eq!(position.offset, 0);
        assert_eq!(colour.offset, 12);
        assert_eq!((uv.location, uv.offset), (2, 24));
    }
}
