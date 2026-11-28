use super::*;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub(super) fn save_scene(
    scene: &Scene,
    path: &std::path::Path,
    project: &std::path::Path,
) -> Result<(), String> {
    let directory = project.join("Assets/Meshes");

    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;

    let mut scene = scene.clone();

    for object in &mut scene.objects {
        if object.meshes.is_empty() {
            continue;
        }

        if object.mesh_asset.is_none() {
            let bytes = serde_json::to_vec(&object.meshes).map_err(|e| e.to_string())?;

            let mut hasher = std::collections::hash_map::DefaultHasher::new();

            bytes.hash(&mut hasher);

            let filename = format!("{:016x}.gxmesh", hasher.finish());

            let relative = format!("Assets/Meshes/{}", filename);

            let asset_path = project.join(&relative);

            if !asset_path.exists() {
                std::fs::write(asset_path, bytes).map_err(|e| e.to_string())?;
            }

            object.mesh_asset = Some(relative);
        }

        // Store only the asset reference in scene files.
        object.meshes = Arc::new(Vec::new());
    }

    let data = serde_json::to_vec_pretty(&scene).map_err(|e| e.to_string())?;

    std::fs::write(path, data).map_err(|e| e.to_string())
}

pub(super) fn hydrate_scene(scene: &mut Scene, project: &std::path::Path) -> Result<(), String> {
    let mut cache: HashMap<String, Arc<Vec<mesh::MeshData>>> = HashMap::new();

    for object in &mut scene.objects {
        let Some(asset) = &object.mesh_asset else {
            continue;
        };

        if let Some(meshes) = cache.get(asset) {
            object.meshes = meshes.clone();
            continue;
        }

        let data = std::fs::read(project.join(asset)).map_err(|e| e.to_string())?;

        let meshes: Vec<mesh::MeshData> =
            serde_json::from_slice(&data).map_err(|e| e.to_string())?;

        let meshes = Arc::new(meshes);

        cache.insert(asset.clone(), meshes.clone());

        object.meshes = meshes;
    }

    Ok(())
}
