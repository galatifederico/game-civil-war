use std::path::{Path, PathBuf};

use super::defs::ContentPack;

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("impossibile leggere {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("errore di sintassi in {path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("contenuti non validi:\n  - {}", .0.join("\n  - "))]
    Invalid(Vec<String>),
}

/// Parses one file (RON or, with a `.json` extension, JSON).
pub fn parse_pack(text: &str, path: &Path) -> Result<ContentPack, ContentError> {
    let is_json = path.extension().is_some_and(|e| e == "json");
    let res = if is_json {
        serde_json::from_str(text).map_err(|e| e.to_string())
    } else {
        ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|e| e.to_string())
    };
    res.map_err(|message| ContentError::Parse { path: path.to_path_buf(), message })
}

/// Loads every `.ron`/`.json` file of a folder (sorted by name) as partial packs, merged in order.
pub fn load_pack_dir(dir: impl AsRef<Path>) -> Result<Vec<ContentPack>, ContentError> {
    let dir = dir.as_ref();
    let rd = std::fs::read_dir(dir).map_err(|source| ContentError::Io { path: dir.into(), source })?;
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "ron" || e == "json"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(p)
                .map_err(|source| ContentError::Io { path: p.clone(), source })?;
            let mut pack = parse_pack(&text, p)?;
            // Tile maps kept in their own text files.
            if let Some(map) = pack.map.as_mut() {
                for layer in map.layers.iter_mut() {
                    if let Some(f) = &layer.tiles_file {
                        let path = dir.join(f);
                        let rows = std::fs::read_to_string(&path).map_err(|source| ContentError::Io { path: path.clone(), source })?;
                        layer.tiles = rows.lines().map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect();
                    }
                }
            }
            Ok(pack)
        })
        .collect()
}
