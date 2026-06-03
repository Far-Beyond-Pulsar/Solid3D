use std::io::Write;

use solid_rs::prelude::*;

use crate::X_FORMAT;

pub struct XSaver;

impl Saver for XSaver {
    fn format_info(&self) -> &'static FormatInfo {
        &X_FORMAT
    }

    fn save(&self, scene: &Scene, writer: &mut dyn Write, _options: &SaveOptions) -> Result<()> {
        if scene.meshes.is_empty() {
            return Err(SolidError::unsupported(
                "DirectX .x saver requires at least one mesh",
            ));
        }
        if scene.meshes.len() > 1 {
            return Err(SolidError::unsupported(
                "DirectX .x saver currently supports saving one mesh per scene",
            ));
        }

        let mesh = &scene.meshes[0];
        let faces = mesh_to_faces(mesh)?;
        let normals = mesh_vertex_normals(mesh, &faces);

        writeln!(writer, "xof 0303txt 0032").map_err(SolidError::Io)?;
        writeln!(writer, "Mesh {{").map_err(SolidError::Io)?;

        writeln!(writer, "{};", mesh.vertices.len()).map_err(SolidError::Io)?;
        for (i, v) in mesh.vertices.iter().enumerate() {
            let end = if i + 1 == mesh.vertices.len() {
                ";"
            } else {
                ","
            };
            writeln!(
                writer,
                "{};{};{};{end}",
                v.position.x, v.position.y, v.position.z
            )
            .map_err(SolidError::Io)?;
        }

        writeln!(writer, "{};", faces.len()).map_err(SolidError::Io)?;
        for (i, face) in faces.iter().enumerate() {
            let end = if i + 1 == faces.len() { ";" } else { "," };
            write!(writer, "{};", face.len()).map_err(SolidError::Io)?;
            for (j, index) in face.iter().enumerate() {
                if j > 0 {
                    write!(writer, ",").map_err(SolidError::Io)?;
                }
                write!(writer, "{index}").map_err(SolidError::Io)?;
            }
            writeln!(writer, ";{end}").map_err(SolidError::Io)?;
        }

        writeln!(writer, "MeshNormals {{").map_err(SolidError::Io)?;
        writeln!(writer, "{};", normals.len()).map_err(SolidError::Io)?;
        for (i, n) in normals.iter().enumerate() {
            let end = if i + 1 == normals.len() { ";" } else { "," };
            writeln!(writer, "{};{};{};{end}", n.x, n.y, n.z).map_err(SolidError::Io)?;
        }

        writeln!(writer, "{};", faces.len()).map_err(SolidError::Io)?;
        for (i, face) in faces.iter().enumerate() {
            let end = if i + 1 == faces.len() { ";" } else { "," };
            write!(writer, "{};", face.len()).map_err(SolidError::Io)?;
            for (j, index) in face.iter().enumerate() {
                if j > 0 {
                    write!(writer, ",").map_err(SolidError::Io)?;
                }
                write!(writer, "{index}").map_err(SolidError::Io)?;
            }
            writeln!(writer, ";{end}").map_err(SolidError::Io)?;
        }

        writeln!(writer, "}}").map_err(SolidError::Io)?;
        writeln!(writer, "}}").map_err(SolidError::Io)?;
        Ok(())
    }
}

fn mesh_to_faces(mesh: &Mesh) -> Result<Vec<Vec<u32>>> {
    let mut faces = Vec::new();
    for primitive in &mesh.primitives {
        match primitive.topology {
            Topology::TriangleList => {
                if primitive.indices.len() % 3 != 0 {
                    return Err(SolidError::invalid_ref(
                        "triangle-list primitive index count is not divisible by 3",
                    ));
                }
                for tri in primitive.indices.chunks(3) {
                    faces.push(vec![tri[0], tri[1], tri[2]]);
                }
            }
            Topology::QuadList => {
                if primitive.indices.len() % 4 != 0 {
                    return Err(SolidError::invalid_ref(
                        "quad-list primitive index count is not divisible by 4",
                    ));
                }
                for q in primitive.indices.chunks(4) {
                    faces.push(vec![q[0], q[1], q[2], q[3]]);
                }
            }
            _ => {
                return Err(SolidError::unsupported(format!(
                    "DirectX .x saver does not support topology {}",
                    primitive.topology.name()
                )));
            }
        }
    }
    if faces.is_empty() {
        return Err(SolidError::unsupported(
            "DirectX .x saver requires at least one face",
        ));
    }
    Ok(faces)
}

fn mesh_vertex_normals(mesh: &Mesh, faces: &[Vec<u32>]) -> Vec<glam::Vec3> {
    let mut normals = vec![glam::Vec3::ZERO; mesh.vertices.len()];
    let mut counts = vec![0u32; mesh.vertices.len()];

    let mut has_any = false;
    for (i, v) in mesh.vertices.iter().enumerate() {
        if let Some(n) = v.normal {
            normals[i] = n.normalize_or_zero();
            if normals[i] != glam::Vec3::ZERO {
                has_any = true;
            }
        }
    }
    if has_any {
        return normals;
    }

    for face in faces {
        if face.len() < 3 {
            continue;
        }
        let i0 = face[0] as usize;
        let i1 = face[1] as usize;
        let i2 = face[2] as usize;
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }
        let p0 = mesh.vertices[i0].position;
        let p1 = mesh.vertices[i1].position;
        let p2 = mesh.vertices[i2].position;
        let n = (p1 - p0).cross(p2 - p0).normalize_or_zero();
        if n == glam::Vec3::ZERO {
            continue;
        }
        for &idx in face {
            let vi = idx as usize;
            if vi < normals.len() {
                normals[vi] += n;
                counts[vi] += 1;
            }
        }
    }

    for (n, count) in normals.iter_mut().zip(counts.iter()) {
        if *count > 0 {
            *n = (*n / *count as f32).normalize_or_zero();
        } else if *n == glam::Vec3::ZERO {
            *n = glam::Vec3::Z;
        }
    }
    normals
}
