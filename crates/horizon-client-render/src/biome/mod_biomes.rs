//! Mods' biomes (a content mod's `data/<ns>/worldgen/biome`, as Java mods carry them).
use super::{BiomeInfo, Colormaps};
use crate::Result;
use serde_json::Value;
use std::{fs, path::Path};

/// The biome files of every namespace but vanilla's under `root/data`, by namespace then name.
pub fn mod_biomes(root: &Path) -> Vec<(String, std::path::PathBuf)> {
    let mut out = Vec::new();
    let Ok(namespaces) = fs::read_dir(root.join("data")) else {
        return out;
    };
    let mut namespaces: Vec<String> = namespaces
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n != "minecraft")
        .collect();
    namespaces.sort();
    for namespace in namespaces {
        let Ok(files) = fs::read_dir(root.join("data").join(&namespace).join("worldgen/biome"))
        else {
            continue;
        };
        let mut files: Vec<_> = files
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        files.sort();
        out.extend(files.into_iter().map(|p| (namespace.clone(), p)));
    }
    out
}

/// Adds the mods' biomes of `root` to `list`, read with `maps`.
pub(super) fn push_mod_biomes(
    root: &Path,
    maps: &Colormaps,
    list: &mut Vec<BiomeInfo>,
) -> Result<()> {
    // Mods' biomes (a content mod's data/<ns>/worldgen/biome, as Java mods' carry them)
    // after vanilla's, so vanilla's keep their ids.
    for (namespace, path) in mod_biomes(root) {
        let name = format!(
            "{namespace}:{}",
            path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or("bad biome")?
        );
        let v: Value = serde_json::from_slice(&fs::read(&path)?)?;
        list.push(BiomeInfo::from_json(name, &v, maps));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_mod_biomes_in_stable_order_without_vanilla() {
        let root = std::env::temp_dir().join(format!("horizon-biomes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for ns in ["zeta", "alpha", "minecraft"] {
            let dir = root.join("data").join(ns).join("worldgen/biome");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("b.json"), "{}").unwrap();
            fs::write(dir.join("a.json"), "{}").unwrap();
            fs::write(dir.join("ignore.txt"), "{}").unwrap();
        }
        let actual: Vec<_> = mod_biomes(&root).into_iter().map(|(ns, path)| {
            (ns, path.file_name().unwrap().to_string_lossy().into_owned())
        }).collect();
        assert_eq!(actual, vec![("alpha".into(), "a.json".into()),
            ("alpha".into(), "b.json".into()), ("zeta".into(), "a.json".into()),
            ("zeta".into(), "b.json".into())]);
        fs::remove_dir_all(root).unwrap();
    }
}
