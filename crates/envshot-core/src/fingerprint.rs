use serde_json::{Map, Value};

use envshot_model::{Snapshot, VersionValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FingerprintProfile {
    ReproV1,
}

impl FingerprintProfile {
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::ReproV1 => "repro-v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    pub value: String,

    pub projection: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FingerprintError {
    #[error("unsupported schema major {found} (reader supports major {supported})")]
    UnsupportedSchemaMajor { found: u32, supported: u32 },
}

pub fn fingerprint(
    snapshot: &Snapshot,
    profile: &FingerprintProfile,
) -> Result<Fingerprint, FingerprintError> {
    let supported = envshot_model::SchemaIdentifier::current().major;
    if snapshot.schema.major != supported {
        return Err(FingerprintError::UnsupportedSchemaMajor {
            found: snapshot.schema.major,
            supported,
        });
    }

    let facts = projection_facts(snapshot, *profile);
    let document = serde_json::json!({
        "profile": profile.id(),
        "schema": {
            "name": snapshot.schema.name,
            "major": snapshot.schema.major,
            "minor": snapshot.schema.minor,
        },
        "facts": facts,
    });

    let projection = serde_json::to_string(&document)
        .expect("canonical JSON serialization of sorted maps cannot fail");
    let value = envelope(&projection, *profile);
    Ok(Fingerprint { value, projection })
}

fn envelope(projection: &str, profile: FingerprintProfile) -> String {
    use sha2::Digest as _;

    let mut hasher = sha2::Sha256::new();
    hasher.update(b"envshot-fingerprint\0");
    hasher.update(profile.id().as_bytes());
    hasher.update(b"\0");
    hasher.update(envshot_model::SchemaIdentifier::current().major.to_string());
    hasher.update(b"\0");
    hasher.update(projection.as_bytes());
    let digest = hasher.finalize();
    format!("{}:sha256:{}", profile.id(), hex(&digest))
}

fn hex(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[char; 16] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX_DIGITS[(byte >> 4) as usize]);
        out.push(HEX_DIGITS[(byte & 0xF) as usize]);
    }
    out
}

fn projection_facts(snapshot: &Snapshot, profile: FingerprintProfile) -> Value {
    let _ = profile;
    let mut facts = Map::new();

    for (id, section) in &snapshot.sections {
        let Some(data) = section.data() else {
            continue;
        };
        match data {
            envshot_model::SectionData::Os(os) => {
                facts.insert(format!("{id}.family"), enum_code(&os.family));
                insert_version(&mut facts, id, "version", &os.version);
                insert_version_option(&mut facts, id, "build", os.build.as_ref());
            }
            envshot_model::SectionData::Architecture(arch) => {
                facts.insert(format!("{id}.process"), enum_code(&arch.process));
                facts.insert(format!("{id}.os"), enum_code(&arch.os));
            }
            envshot_model::SectionData::Cpu(cpu) => {
                facts.insert(
                    format!("{id}.features"),
                    Value::Array(
                        cpu.features
                            .iter()
                            .map(|feature| Value::String(feature.clone()))
                            .collect(),
                    ),
                );
            }
            envshot_model::SectionData::Memory(memory) => {
                facts.insert(
                    format!("{id}.installed_bytes"),
                    Value::Number(memory.installed_bytes.into()),
                );
            }
            envshot_model::SectionData::Virtualization(virt) => {
                facts.insert(format!("{id}.category"), enum_code(&virt.category));
            }

            envshot_model::SectionData::Locale(_)
            | envshot_model::SectionData::Windows(_)
            | envshot_model::SectionData::Gpu(_)
            | envshot_model::SectionData::Distro(_) => {}
        }
    }

    Value::Object(facts)
}

fn enum_code<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("closed-vocabulary serialization cannot fail")
}

fn insert_version(facts: &mut Map<String, Value>, id: &str, field: &str, version: &VersionValue) {
    facts.insert(format!("{id}.{field}"), version_fact(version));
}

fn insert_version_option(
    facts: &mut Map<String, Value>,
    id: &str,
    field: &str,
    version: Option<&VersionValue>,
) {
    if let Some(version) = version {
        facts.insert(format!("{id}.{field}"), version_fact(version));
    }
}

fn version_fact(version: &VersionValue) -> Value {
    match version.normalized() {
        Some(normalized) => serde_json::json!({
            "major": normalized.major,
            "minor": normalized.minor,
            "patch": normalized.patch,
        }),
        None => Value::String(version.raw().to_owned()),
    }
}
