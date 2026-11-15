//! The behaviour tables' big source files read without keeping them as JSON trees: a
//! modded pack's ran to hundreds of MB (`block-behaviours.json`) and gigabytes
//! (`gameplay-facts.json`).
use super::*;

impl Behaviours {
    /// [`Self::load_with_gameplay`], `block-behaviours.json` read only when the snapshot misses
    /// (the caller need not keep its tree: a modded pack's ran to hundreds of MB).
    pub fn load_with_gameplay_from_file<G: GameplayStateSource>(
        registry: &BlockRegistry,
        resources_dir: &Path,
        gameplay: &[G],
    ) -> io::Result<Self> {
        cached(registry, resources_dir, || {
            let behaviour = read_json(&resources_dir.join("block-behaviours.json"))?;
            build_with_gameplay(registry, resources_dir, &behaviour, gameplay)
        })
    }
}

/// `gameplay-facts.json`'s fields the behaviour tables read ([`GameplayStateSource`]), streamed
/// from the file: the rest of each row (outline edges, the other sounds: most of the file) is
/// skipped as it is read. As a JSON tree the file took ten times its size (3.4 GB for 208k
/// states, a modded pack's), though the tables keep only tens of MB.
pub(super) fn read_gameplay(path: &Path) -> io::Result<GameplayFile> {
    let file = io::BufReader::with_capacity(1 << 20, std::fs::File::open(path)?);
    serde_json::from_reader(file).map_err(|e| invalid(format!("{}: {e}", path.display())))
}

#[derive(serde::Deserialize)]
pub(super) struct GameplayFile {
    pub(super) states: Vec<GameplayRow>,
}

/// One state's gameplay row, as [`GameplayStateSource for Value`] reads it (missing fields:
/// false, 0, "", a sound's 1.0).
#[derive(Default)]
pub(super) struct GameplayRow {
    replaceable: bool,
    random_ticks: bool,
    solid_render: bool,
    liquid: bool,
    block_entity: bool,
    ignited_by_lava: bool,
    can_occlude: bool,
    full_collision: bool,
    skylight_down: bool,
    requires_tool: bool,
    shape_occlusion: bool,
    emission: u64,
    dampening: u64,
    hardness: f64,
    push: Box<str>,
    sound: GameplaySound,
}

#[derive(Default)]
struct GameplaySound {
    volume: Option<f64>,
    pitch: Option<f64>,
    place: Option<SoundId>,
    break_sound: Option<SoundId>,
    fall: Option<SoundId>,
}

struct SoundId {
    id: Box<str>,
}

// A row is discarded as soon as its small projection is built. Deserializing each row
// through Value preserves the existing permissive defaults for nulls, wrong types and
// non-object rows without retaining the entire (potentially multi-gigabyte) JSON tree.
impl<'de> serde::Deserialize<'de> for GameplayRow {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = <Value as serde::Deserialize>::deserialize(deserializer)?;
        let sound_id = |name: &str| {
            let id = value.sound_id(name);
            (!id.is_empty()).then(|| SoundId { id: id.into() })
        };
        Ok(Self {
            replaceable: value.flag("replaceable"),
            random_ticks: value.flag("random_ticks"),
            solid_render: value.flag("solid_render"),
            liquid: value.flag("liquid"),
            block_entity: value.flag("block_entity"),
            ignited_by_lava: value.flag("ignited_by_lava"),
            can_occlude: value.flag("can_occlude"),
            full_collision: value.flag("full_collision"),
            skylight_down: value.flag("skylight_down"),
            requires_tool: value.flag("requires_tool"),
            shape_occlusion: value.flag("shape_occlusion"),
            emission: value.unsigned("emission"),
            dampening: value.unsigned("dampening"),
            hardness: value.number("hardness"),
            push: value.text("push").into(),
            sound: GameplaySound {
                volume: Some(value.sound_number("volume")),
                pitch: Some(value.sound_number("pitch")),
                place: sound_id("place"),
                break_sound: sound_id("break"),
                fall: sound_id("fall"),
            },
        })
    }
}

impl GameplayStateSource for GameplayRow {
    fn flag(&self, name: &str) -> bool {
        match name {
            "replaceable" => self.replaceable,
            "random_ticks" => self.random_ticks,
            "solid_render" => self.solid_render,
            "liquid" => self.liquid,
            "block_entity" => self.block_entity,
            "ignited_by_lava" => self.ignited_by_lava,
            "can_occlude" => self.can_occlude,
            "full_collision" => self.full_collision,
            "skylight_down" => self.skylight_down,
            "requires_tool" => self.requires_tool,
            "shape_occlusion" => self.shape_occlusion,
            _ => false,
        }
    }
    fn unsigned(&self, name: &str) -> u64 {
        match name {
            "emission" => self.emission,
            "dampening" => self.dampening,
            _ => 0,
        }
    }
    fn number(&self, name: &str) -> f64 {
        if name == "hardness" {
            self.hardness
        } else {
            0.0
        }
    }
    fn text(&self, name: &str) -> &str {
        if name == "push" { &self.push } else { "" }
    }
    fn sound_number(&self, name: &str) -> f64 {
        match name {
            "volume" => self.sound.volume,
            "pitch" => self.sound.pitch,
            _ => None,
        }
        .unwrap_or(1.0)
    }
    fn sound_id(&self, name: &str) -> &str {
        match name {
            "place" => self.sound.place.as_ref(),
            "break" => self.sound.break_sound.as_ref(),
            "fall" => self.sound.fall.as_ref(),
            _ => None,
        }
        .map_or("", |s| &s.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn streamed_rows_match_value_defaults_and_fields() {
        for value in [
            json!({}),
            json!(null),
            json!({"replaceable": true, "random_ticks": true, "solid_render": true,
                "liquid": true, "block_entity": true, "ignited_by_lava": true,
                "can_occlude": true, "full_collision": true, "skylight_down": true,
                "requires_tool": true, "shape_occlusion": true, "emission": 15,
                "dampening": 3, "hardness": -0.5, "push": "block",
                "sound": {"volume": 0.6, "pitch": 1.2, "place": {"id": "a"},
                    "break": {"id": "b"}, "fall": {"id": "c"}}}),
            json!({"replaceable": null, "emission": -2, "dampening": "oops",
                "hardness": [], "push": 8, "sound": {"volume": null, "pitch": "oops",
                "place": {}, "break": {"id": 3}, "fall": null}}),
            json!({"sound": null}),
        ] {
            let row: GameplayRow = serde_json::from_value(value.clone()).unwrap();
            for name in ["replaceable", "random_ticks", "solid_render", "liquid", "block_entity",
                "ignited_by_lava", "can_occlude", "full_collision", "skylight_down",
                "requires_tool", "shape_occlusion", "unknown"] {
                assert_eq!(row.flag(name), value.flag(name), "{name}: {value}");
            }
            for name in ["emission", "dampening", "unknown"] {
                assert_eq!(row.unsigned(name), value.unsigned(name), "{name}: {value}");
            }
            for name in ["hardness", "unknown"] {
                assert_eq!(row.number(name), value.number(name), "{name}: {value}");
            }
            for name in ["push", "unknown"] {
                assert_eq!(row.text(name), value.text(name), "{name}: {value}");
            }
            for name in ["volume", "pitch", "unknown"] {
                assert_eq!(row.sound_number(name), value.sound_number(name), "{name}: {value}");
            }
            for name in ["place", "break", "fall", "unknown"] {
                assert_eq!(row.sound_id(name), value.sound_id(name), "{name}: {value}");
            }
        }
    }

    #[test]
    fn requires_a_states_array() {
        assert!(serde_json::from_str::<GameplayFile>("{}").is_err());
        assert!(serde_json::from_str::<GameplayFile>(r#"{"states": null}"#).is_err());
        assert_eq!(serde_json::from_str::<GameplayFile>(r#"{"states": [null, {}]}"#).unwrap().states.len(), 2);
    }
}
