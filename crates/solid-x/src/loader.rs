use std::io::Read;

use directx_mesh::read_directx_mesh;
use solid_rs::prelude::*;

use crate::X_FORMAT;

pub struct XLoader;

impl Loader for XLoader {
    fn format_info(&self) -> &'static FormatInfo {
        &X_FORMAT
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 16];
        let n = reader.read(&mut buf).unwrap_or(0);
        let header = &buf[..n];
        if header.starts_with(b"xof ") {
            if header.len() >= 12 && &header[8..12] == b"txt " {
                return 0.95;
            }
            return 0.65;
        }
        0.0
    }

    fn load(&self, reader: &mut dyn ReadSeek, _options: &LoadOptions) -> Result<Scene> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(SolidError::Io)?;

        validate_x_header(&data)?;

        let text = std::str::from_utf8(&data)
            .map_err(|e| SolidError::parse(format!("DirectX .x text decode failed: {e}")))?;
        let x = read_directx_mesh(text)
            .map_err(|e| SolidError::format("x", format!("DirectX parse failed: {e}")))?;

        let mut mesh = Mesh::new("DirectXMesh");
        mesh.vertices = x
            .vertices
            .iter()
            .map(|&(x, y, z)| Vertex::new(glam::Vec3::new(x, y, z)))
            .collect();

        if x.normals.len() == mesh.vertices.len() {
            for (v, &(nx, ny, nz)) in mesh.vertices.iter_mut().zip(x.normals.iter()) {
                let n = glam::Vec3::new(nx, ny, nz).normalize_or_zero();
                if n != glam::Vec3::ZERO {
                    v.normal = Some(n);
                }
            }
        }

        let mut tri_indices = Vec::new();
        for face in &x.faces {
            if face.len() < 3 {
                return Err(SolidError::parse(
                    "DirectX .x face has fewer than 3 indices",
                ));
            }
            let i0 = face[0];
            for i in 1..face.len() - 1 {
                tri_indices.push(i0);
                tri_indices.push(face[i]);
                tri_indices.push(face[i + 1]);
            }
        }
        mesh.primitives
            .push(Primitive::triangles(tri_indices, None));
        mesh.compute_bounds();

        let mut builder = SceneBuilder::named("DirectX Scene");
        let mesh_idx = builder.push_mesh(mesh);
        let root = builder.add_root_node("Root");
        builder.attach_mesh(root, mesh_idx);
        Ok(builder.build())
    }
}

fn validate_x_header(data: &[u8]) -> Result<()> {
    if data.len() < 16 {
        return Err(SolidError::parse("DirectX .x file is too short"));
    }
    if &data[..4] != b"xof " {
        return Err(SolidError::parse("DirectX .x missing 'xof ' header"));
    }

    let format = &data[8..12];
    match format {
        b"txt " => Ok(()),
        b"bin " => Err(SolidError::unsupported(
            "DirectX .x binary flavor (xof ....bin ....) is not supported",
        )),
        b"tzip" | b"bzip" => Err(SolidError::unsupported(
            "DirectX .x compressed flavors (tzip/bzip) are not supported",
        )),
        _ => Err(SolidError::parse("DirectX .x has unknown encoding flavor")),
    }
}
