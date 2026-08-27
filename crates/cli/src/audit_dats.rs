use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use dats::{
    base::{DatId, DatPath},
    context::DatContext,
    dat_format::DatFormat,
    formats::{
        auto_translate::AutoTranslate, dialog::Dialog, dmsg_table::DmsgTable,
        entity_names::EntityNames, events::Events, furniture_data::FurnitureData,
        item_info::ItemInfoTable, menu_table::MenuTable, merit_category_table::MeritCategoryTable,
        merit_table::MeritTable, status_info::StatusInfoTable, string_table::StringTable,
        xistring_table::XiStringTable, zone_data::ZoneData,
    },
    id_mapping::{DatFormatKind, DatIdMapping},
};
use serde::Serialize;
use walkdir::WalkDir;

const REPORT_SCHEMA_VERSION: u32 = 1;
const XBOX_REPORT_SCHEMA_VERSION: u32 = 4;
const XBOX_BASE_PACKAGE: &str = "0001";
const XBOX_PACKAGE_SLOT_NAMES: &[&str] = &[
    "R000100", "R000101", "R000102", "R000103", "R000104", "R000105", "R000106", "R000107",
    "R000108", "R000109", "R000110", "R000111",
];
const XBOX_PACKAGE_NAMES: &[&str] = &[
    "0001", "R000100", "R000101", "R000102", "R000103", "R000104", "R000105", "R000106", "R000107",
    "R000108", "R000109", "R000110", "R000111",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FormatKind {
    AutoTranslate,
    Dialog,
    DmsgTable,
    EntityNames,
    Events,
    FurnitureData,
    ItemInfoTable,
    MenuTable,
    MeritCategoryTable,
    MeritTable,
    StatusInfoTable,
    StringTable,
    XiStringTable,
    ZoneData,
}

impl FormatKind {
    fn name(self) -> &'static str {
        match self {
            Self::AutoTranslate => "auto_translate",
            Self::Dialog => "dialog",
            Self::DmsgTable => "dmsg_table",
            Self::EntityNames => "entity_names",
            Self::Events => "events",
            Self::FurnitureData => "furniture_data",
            Self::ItemInfoTable => "item_info_table",
            Self::MenuTable => "menu_table",
            Self::MeritCategoryTable => "merit_category_table",
            Self::MeritTable => "merit_table",
            Self::StatusInfoTable => "status_info_table",
            Self::StringTable => "string_table",
            Self::XiStringTable => "xi_string_table",
            Self::ZoneData => "zone_data",
        }
    }

    fn check_path(self, path: &PathBuf) -> bool {
        panic::catch_unwind(AssertUnwindSafe(|| match self {
            Self::AutoTranslate => AutoTranslate::check_path(path).is_ok(),
            Self::Dialog => Dialog::check_path(path).is_ok(),
            Self::DmsgTable => DmsgTable::check_path(path).is_ok(),
            Self::EntityNames => EntityNames::check_path(path).is_ok(),
            Self::Events => Events::check_path(path).is_ok(),
            Self::FurnitureData => FurnitureData::check_path(path).is_ok(),
            Self::ItemInfoTable => ItemInfoTable::check_path(path).is_ok(),
            Self::MenuTable => MenuTable::check_path(path).is_ok(),
            Self::MeritCategoryTable => MeritCategoryTable::check_path(path).is_ok(),
            Self::MeritTable => MeritTable::check_path(path).is_ok(),
            Self::StatusInfoTable => StatusInfoTable::check_path(path).is_ok(),
            Self::StringTable => StringTable::check_path(path).is_ok(),
            Self::XiStringTable => XiStringTable::check_path(path).is_ok(),
            Self::ZoneData => ZoneData::check_path(path).is_ok(),
        }))
        .unwrap_or(false)
    }

    fn round_trip(self, bytes: &[u8]) -> std::result::Result<Vec<u8>, RoundTripError> {
        macro_rules! parse_and_write {
            ($format:ty) => {{
                let parsed = <$format>::from_bytes(bytes)
                    .map_err(|error| RoundTripError::new(FailureKind::Parse, error.to_string()))?;
                parsed
                    .to_bytes()
                    .map_err(|error| RoundTripError::new(FailureKind::Write, error.to_string()))
            }};
        }

        match self {
            Self::AutoTranslate => parse_and_write!(AutoTranslate),
            Self::Dialog => parse_and_write!(Dialog),
            Self::DmsgTable => parse_and_write!(DmsgTable),
            Self::EntityNames => parse_and_write!(EntityNames),
            Self::Events => parse_and_write!(Events),
            Self::FurnitureData => parse_and_write!(FurnitureData),
            Self::ItemInfoTable => parse_and_write!(ItemInfoTable),
            Self::MenuTable => parse_and_write!(MenuTable),
            Self::MeritCategoryTable => parse_and_write!(MeritCategoryTable),
            Self::MeritTable => parse_and_write!(MeritTable),
            Self::StatusInfoTable => parse_and_write!(StatusInfoTable),
            Self::StringTable => parse_and_write!(StringTable),
            Self::XiStringTable => parse_and_write!(XiStringTable),
            Self::ZoneData => parse_and_write!(ZoneData),
        }
    }
}

impl From<DatFormatKind> for FormatKind {
    fn from(format: DatFormatKind) -> Self {
        match format {
            DatFormatKind::AutoTranslate => Self::AutoTranslate,
            DatFormatKind::Dialog => Self::Dialog,
            DatFormatKind::DmsgTable => Self::DmsgTable,
            DatFormatKind::EntityNames => Self::EntityNames,
            DatFormatKind::Events => Self::Events,
            DatFormatKind::FurnitureData => Self::FurnitureData,
            DatFormatKind::ItemInfoTable => Self::ItemInfoTable,
            DatFormatKind::MenuTable => Self::MenuTable,
            DatFormatKind::MeritCategoryTable => Self::MeritCategoryTable,
            DatFormatKind::MeritTable => Self::MeritTable,
            DatFormatKind::StatusInfoTable => Self::StatusInfoTable,
            DatFormatKind::XiStringTable => Self::XiStringTable,
            DatFormatKind::ZoneData => Self::ZoneData,
        }
    }
}

fn known_format_mappings() -> Vec<(u32, FormatKind)> {
    DatIdMapping::get()
        .format_mappings()
        .into_iter()
        .map(|mapping| (mapping.id.get_inner(), mapping.format.into()))
        .collect()
}

fn known_formats_by_id(format_mappings: &[(u32, FormatKind)]) -> BTreeMap<u32, Vec<FormatKind>> {
    let mut formats_by_id = BTreeMap::<u32, Vec<FormatKind>>::new();
    for (id, format) in format_mappings {
        formats_by_id.entry(*id).or_default().push(*format);
    }
    for formats in formats_by_id.values_mut() {
        formats.sort_unstable();
        formats.dedup();
    }
    formats_by_id
}

fn detect_format(
    path: &PathBuf,
    ids: &[u32],
    formats_by_id: &BTreeMap<u32, Vec<FormatKind>>,
) -> Option<FormatKind> {
    let mapped_formats = ids
        .iter()
        .filter_map(|id| formats_by_id.get(id))
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>();

    if mapped_formats.len() == 1 {
        return mapped_formats.first().copied();
    }
    if mapped_formats.len() > 1 {
        let detected = SUPPORTED_FORMATS
            .iter()
            .copied()
            .filter(|format| mapped_formats.contains(format) && format.check_path(path))
            .collect::<Vec<_>>();
        return if detected.len() == 1 {
            detected.first().copied()
        } else {
            None
        };
    }

    SUPPORTED_FORMATS
        .iter()
        .copied()
        .filter(|format| !matches!(format, FormatKind::ZoneData))
        .find(|format| format.check_path(path))
}

const SUPPORTED_FORMATS: &[FormatKind] = &[
    FormatKind::AutoTranslate,
    FormatKind::Dialog,
    FormatKind::DmsgTable,
    FormatKind::EntityNames,
    FormatKind::Events,
    FormatKind::FurnitureData,
    FormatKind::ItemInfoTable,
    FormatKind::MenuTable,
    FormatKind::MeritCategoryTable,
    FormatKind::MeritTable,
    FormatKind::StatusInfoTable,
    FormatKind::StringTable,
    FormatKind::XiStringTable,
    FormatKind::ZoneData,
];

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum AuditStatus {
    Ok,
    Missing,
    Unrecognized,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum FailureKind {
    Missing,
    Read,
    Parse,
    Write,
    RoundTrip,
}

#[derive(Debug)]
struct RoundTripError {
    kind: FailureKind,
    message: String,
}

impl RoundTripError {
    fn new(kind: FailureKind, message: String) -> Self {
        Self { kind, message }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DatAuditFile {
    id: u32,
    path: String,
    status: AuditStatus,
    format: Option<String>,
    failure_kind: Option<FailureKind>,
    error: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct AuditSummary {
    total: usize,
    missing: usize,
    recognized: usize,
    round_trip_ok: usize,
    unrecognized: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
struct AuditReport {
    schema_version: u32,
    summary: AuditSummary,
    files: Vec<DatAuditFile>,
}

#[derive(Debug, Clone)]
struct XboxMapping {
    dat_path: DatPath,
    ids: Vec<u32>,
}

#[derive(Debug, Clone)]
struct XboxPackageCandidate {
    package: String,
    dat_path: DatPath,
    physical_path: String,
}

#[derive(Debug, Clone, Serialize)]
struct XboxPackageFile {
    package: String,
    logical_path: String,
    physical_path: String,
}

#[derive(Debug, Clone, Serialize)]
struct XboxMappedId {
    id: u32,
    logical_path: String,
}

#[derive(Debug, Clone, Serialize)]
struct XboxMappingReference {
    logical_path: String,
    ids: Vec<u32>,
    package: String,
    expected_path: String,
}

#[derive(Debug, Clone, Serialize)]
struct XboxDuplicateMapping {
    logical_path: String,
    ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct XboxMappingResult {
    logical_path: String,
    ids: Vec<u32>,
    package: String,
    selected_path: Option<String>,
    status: AuditStatus,
    format: Option<String>,
    failure_kind: Option<FailureKind>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct XboxFormatResult {
    format: String,
    selected: usize,
    recognized: usize,
    round_trip_ok: usize,
    failed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum XboxClientMappingStatus {
    Selected,
    Missing,
    Absent,
}

#[derive(Debug, Clone, Serialize)]
struct XboxClientFormatMapping {
    id: u32,
    format: String,
    logical_path: Option<String>,
    package: Option<String>,
    selected_path: Option<String>,
    status: XboxClientMappingStatus,
}

#[derive(Debug, Default, Serialize)]
struct XboxAuditSummary {
    mapped_ids: usize,
    unique_mappings: usize,
    duplicate_id_mappings: usize,
    duplicate_id_entries: usize,
    package_candidates: usize,
    selected_dats: usize,
    missing_mapped_paths: usize,
    unreferenced_selected_dats: usize,
    non_selected_mounted_shadow_dats: usize,
    r000100_unmounted_dats: usize,
    package_zero_overrides: usize,
    package_zero_base_fallbacks: usize,
    client_format_mappings: usize,
    selected_client_format_mappings: usize,
    missing_client_format_mappings: usize,
    absent_client_format_mappings: usize,
    recognized: usize,
    round_trip_ok: usize,
    unrecognized: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
struct XboxAuditReport {
    schema_version: u32,
    summary: XboxAuditSummary,
    mapped_ids: Vec<XboxMappedId>,
    unique_mappings: Vec<XboxMappingResult>,
    duplicate_id_mappings: Vec<XboxDuplicateMapping>,
    package_candidates: Vec<XboxPackageFile>,
    selected_dats: Vec<XboxPackageFile>,
    missing_mapped_paths: Vec<XboxMappingReference>,
    unreferenced_selected_dats: Vec<XboxPackageFile>,
    non_selected_mounted_shadow_dats: Vec<XboxPackageFile>,
    r000100_unmounted_dats: Vec<XboxPackageFile>,
    client_format_mappings: Vec<XboxClientFormatMapping>,
    format_results: Vec<XboxFormatResult>,
}

#[derive(Debug, Default)]
struct XboxClassification {
    selected_dats: Vec<XboxPackageFile>,
    missing_mapped_paths: Vec<XboxMappingReference>,
    unreferenced_selected_dats: Vec<XboxPackageFile>,
    non_selected_mounted_shadow_dats: Vec<XboxPackageFile>,
    r000100_unmounted_dats: Vec<XboxPackageFile>,
}

pub fn audit_dats(
    ffxi_path: PathBuf,
    json_output: Option<PathBuf>,
    xbox_packages: bool,
) -> Result<()> {
    if xbox_packages {
        return audit_xbox_packages(ffxi_path, json_output);
    }

    audit_pc_dats(ffxi_path, json_output)
}

fn audit_pc_dats(ffxi_path: PathBuf, json_output: Option<PathBuf>) -> Result<()> {
    let ffxi_path = DatContext::find_ffxi_path(ffxi_path)?;
    let id_map = DatContext::build_rom_id_map(&ffxi_path)?;
    let context = DatContext {
        ffxi_path: ffxi_path.clone(),
        id_map,
        zone_name_to_id_map: Default::default(),
        zone_id_to_name: Default::default(),
    };

    let mut entries: Vec<_> = context.id_map.iter().collect();
    entries.sort_by_key(|(id, _)| id.get_inner());
    let format_mappings = known_format_mappings();
    let formats_by_id = known_formats_by_id(&format_mappings);

    let previous_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let files = entries
        .into_iter()
        .map(|(id, dat_path)| {
            let relative_path = dat_path.to_path();
            let id = id.get_inner();
            audit_file(
                &ffxi_path,
                id,
                std::slice::from_ref(&id),
                &formats_by_id,
                &relative_path,
                &ffxi_path.join(&relative_path),
            )
        })
        .collect::<Vec<_>>();
    panic::set_hook(previous_panic_hook);

    let report = AuditReport {
        schema_version: REPORT_SCHEMA_VERSION,
        summary: summarize(&files),
        files,
    };
    if let Some(json_output) = json_output {
        let json = serde_json::to_string_pretty(&report)?;
        fs::write(&json_output, json.as_bytes()).with_context(|| {
            format!("Could not write audit report to {}", json_output.display())
        })?;
        println!("{}", serde_json::to_string(&report.summary)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn audit_xbox_packages(runtime_root: PathBuf, json_output: Option<PathBuf>) -> Result<()> {
    let table_root = runtime_root.join("0001");
    let id_map = DatContext::build_rom_id_map(&table_root).with_context(|| {
        format!(
            "Could not build Xbox DAT mapping from package {}",
            stable_relative_path(Path::new("0001"))
        )
    })?;
    let mappings = group_xbox_mappings(&id_map);
    let format_mappings = known_format_mappings();
    let formats_by_id = known_formats_by_id(&format_mappings);
    let mapped_ids = mappings
        .iter()
        .flat_map(|mapping| {
            mapping.ids.iter().copied().map(|id| XboxMappedId {
                id,
                logical_path: stable_relative_path(&mapping.dat_path.to_path()),
            })
        })
        .collect::<Vec<_>>();
    let mut mapped_ids = mapped_ids;
    mapped_ids.sort_by_key(|mapped_id| mapped_id.id);
    let candidates = collect_xbox_candidates(&runtime_root)?;
    let classification = classify_xbox_candidates(&mappings, &candidates);
    let selected_files = classification
        .selected_dats
        .iter()
        .map(|file| (file.logical_path.as_str(), file))
        .collect::<HashMap<_, _>>();
    let client_format_mappings =
        project_client_format_mappings(&format_mappings, &id_map, &selected_files);

    let previous_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let unique_mappings = mappings
        .iter()
        .map(|mapping| {
            let logical_path = stable_relative_path(&mapping.dat_path.to_path());
            let selected_file = selected_files.get(logical_path.as_str());
            let package = selected_file
                .map(|file| file.package.clone())
                .unwrap_or_else(|| xbox_missing_package_name(mapping.dat_path).to_string());
            let selected_path = selected_file.map(|file| file.physical_path.clone());
            let result = if let Some(selected_path) = &selected_path {
                let physical_path = runtime_root.join(selected_path);
                audit_file(
                    &runtime_root,
                    0,
                    &mapping.ids,
                    &formats_by_id,
                    Path::new(&logical_path),
                    &physical_path,
                )
            } else {
                DatAuditFile {
                    id: 0,
                    path: logical_path.clone(),
                    status: AuditStatus::Missing,
                    format: None,
                    failure_kind: Some(FailureKind::Missing),
                    error: Some("selected package DAT is missing".to_string()),
                }
            };

            XboxMappingResult {
                logical_path,
                ids: mapping.ids.clone(),
                package,
                selected_path,
                status: result.status,
                format: result.format,
                failure_kind: result.failure_kind,
                error: result.error,
            }
        })
        .collect::<Vec<_>>();
    panic::set_hook(previous_panic_hook);

    let duplicate_id_mappings = mappings
        .iter()
        .filter(|mapping| mapping.ids.len() > 1)
        .map(|mapping| XboxDuplicateMapping {
            logical_path: stable_relative_path(&mapping.dat_path.to_path()),
            ids: mapping.ids.clone(),
        })
        .collect::<Vec<_>>();
    let format_results = summarize_xbox_formats(&unique_mappings);
    let summary = XboxAuditSummary {
        mapped_ids: mapped_ids.len(),
        unique_mappings: mappings.len(),
        duplicate_id_mappings: duplicate_id_mappings.len(),
        duplicate_id_entries: duplicate_id_mappings
            .iter()
            .map(|mapping| mapping.ids.len().saturating_sub(1))
            .sum(),
        package_candidates: candidates.len(),
        selected_dats: classification.selected_dats.len(),
        missing_mapped_paths: classification.missing_mapped_paths.len(),
        unreferenced_selected_dats: classification.unreferenced_selected_dats.len(),
        non_selected_mounted_shadow_dats: classification.non_selected_mounted_shadow_dats.len(),
        r000100_unmounted_dats: classification.r000100_unmounted_dats.len(),
        package_zero_overrides: classification
            .selected_dats
            .iter()
            .filter(|file| file.package == XBOX_PACKAGE_SLOT_NAMES[0])
            .count(),
        package_zero_base_fallbacks: classification
            .selected_dats
            .iter()
            .filter(|file| file.package == XBOX_BASE_PACKAGE)
            .count(),
        client_format_mappings: client_format_mappings.len(),
        selected_client_format_mappings: client_format_mappings
            .iter()
            .filter(|mapping| matches!(mapping.status, XboxClientMappingStatus::Selected))
            .count(),
        missing_client_format_mappings: client_format_mappings
            .iter()
            .filter(|mapping| matches!(mapping.status, XboxClientMappingStatus::Missing))
            .count(),
        absent_client_format_mappings: client_format_mappings
            .iter()
            .filter(|mapping| matches!(mapping.status, XboxClientMappingStatus::Absent))
            .count(),
        recognized: unique_mappings
            .iter()
            .filter(|mapping| mapping.format.is_some())
            .count(),
        round_trip_ok: unique_mappings
            .iter()
            .filter(|mapping| matches!(mapping.status, AuditStatus::Ok))
            .count(),
        unrecognized: unique_mappings
            .iter()
            .filter(|mapping| matches!(mapping.status, AuditStatus::Unrecognized))
            .count(),
        failed: unique_mappings
            .iter()
            .filter(|mapping| matches!(mapping.status, AuditStatus::Failed))
            .count(),
    };
    let report = XboxAuditReport {
        schema_version: XBOX_REPORT_SCHEMA_VERSION,
        summary,
        mapped_ids,
        unique_mappings,
        duplicate_id_mappings,
        package_candidates: candidates.iter().map(package_file).collect(),
        selected_dats: classification.selected_dats,
        missing_mapped_paths: classification.missing_mapped_paths,
        unreferenced_selected_dats: classification.unreferenced_selected_dats,
        non_selected_mounted_shadow_dats: classification.non_selected_mounted_shadow_dats,
        r000100_unmounted_dats: classification.r000100_unmounted_dats,
        client_format_mappings,
        format_results,
    };

    if let Some(json_output) = json_output {
        let json = serde_json::to_string_pretty(&report)?;
        fs::write(&json_output, json.as_bytes()).with_context(|| {
            format!("Could not write audit report to {}", json_output.display())
        })?;
        println!("{}", serde_json::to_string(&report.summary)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn project_client_format_mappings(
    format_mappings: &[(u32, FormatKind)],
    id_map: &HashMap<DatId, DatPath>,
    selected_files: &HashMap<&str, &XboxPackageFile>,
) -> Vec<XboxClientFormatMapping> {
    format_mappings
        .iter()
        .map(|(id, format)| {
            let Some(dat_path) = id_map.get(&DatId::from(*id)) else {
                return XboxClientFormatMapping {
                    id: *id,
                    format: format.name().to_string(),
                    logical_path: None,
                    package: None,
                    selected_path: None,
                    status: XboxClientMappingStatus::Absent,
                };
            };

            let logical_path = stable_relative_path(&dat_path.to_path());
            let selected_file = selected_files.get(logical_path.as_str());
            let package = selected_file
                .map(|file| file.package.clone())
                .unwrap_or_else(|| xbox_missing_package_name(*dat_path).to_string());
            let selected_path = selected_file.map(|file| file.physical_path.clone());
            let status = if selected_path.is_some() {
                XboxClientMappingStatus::Selected
            } else {
                XboxClientMappingStatus::Missing
            };

            XboxClientFormatMapping {
                id: *id,
                format: format.name().to_string(),
                logical_path: Some(logical_path),
                package: Some(package),
                selected_path,
                status,
            }
        })
        .collect()
}

fn group_xbox_mappings(id_map: &HashMap<DatId, DatPath>) -> Vec<XboxMapping> {
    let mut grouped = BTreeMap::<DatPath, Vec<u32>>::new();
    for (id, dat_path) in id_map {
        grouped.entry(*dat_path).or_default().push(id.get_inner());
    }

    grouped
        .into_iter()
        .map(|(dat_path, mut ids)| {
            ids.sort_unstable();
            XboxMapping { dat_path, ids }
        })
        .collect()
}

fn collect_xbox_candidates(runtime_root: &Path) -> Result<Vec<XboxPackageCandidate>> {
    let mut candidates = Vec::new();
    for package in XBOX_PACKAGE_NAMES {
        let package_root = runtime_root.join(package);
        if !package_root.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&package_root) {
            let entry = entry.with_context(|| {
                format!(
                    "Could not enumerate Xbox package {}",
                    stable_relative_path(Path::new(package))
                )
            })?;
            if !entry.file_type().is_file() {
                continue;
            }

            let Ok(relative_path) = entry.path().strip_prefix(runtime_root) else {
                continue;
            };
            let Some(dat_path) = parse_dat_path(relative_path) else {
                continue;
            };
            candidates.push(XboxPackageCandidate {
                package: (*package).to_string(),
                dat_path,
                physical_path: stable_relative_path(relative_path),
            });
        }
    }
    candidates.sort_by(|left, right| {
        left.physical_path
            .cmp(&right.physical_path)
            .then_with(|| left.package.cmp(&right.package))
    });
    Ok(candidates)
}

fn classify_xbox_candidates(
    mappings: &[XboxMapping],
    candidates: &[XboxPackageCandidate],
) -> XboxClassification {
    let mapped_paths = mappings
        .iter()
        .map(|mapping| mapping.dat_path)
        .collect::<BTreeSet<_>>();
    let candidates_by_path = candidates
        .iter()
        .map(|candidate| (candidate.physical_path.to_ascii_lowercase(), candidate))
        .collect::<HashMap<_, _>>();
    let mut selected_paths = BTreeSet::new();
    let mut classification = XboxClassification::default();

    for mapping in mappings {
        let logical_path = stable_relative_path(&mapping.dat_path.to_path());
        if let Some(candidate) = select_xbox_candidate(mapping.dat_path, &candidates_by_path) {
            selected_paths.insert(candidate.physical_path.clone());
            classification.selected_dats.push(package_file(candidate));
        } else {
            let package = xbox_missing_package_name(mapping.dat_path);
            let expected_path = format!("{}/{}", package, logical_path);
            classification
                .missing_mapped_paths
                .push(XboxMappingReference {
                    logical_path,
                    ids: mapping.ids.clone(),
                    package: package.to_string(),
                    expected_path,
                });
        }
    }

    for candidate in candidates {
        if selected_paths.contains(&candidate.physical_path) {
            continue;
        }

        if candidate.package == XBOX_PACKAGE_SLOT_NAMES[0] {
            classification
                .r000100_unmounted_dats
                .push(package_file(candidate));
            continue;
        }

        let is_selected_candidate = select_xbox_candidate(candidate.dat_path, &candidates_by_path)
            .is_some_and(|selected| selected.physical_path == candidate.physical_path);
        if !mapped_paths.contains(&candidate.dat_path) && is_selected_candidate {
            classification
                .unreferenced_selected_dats
                .push(package_file(candidate));
        } else {
            classification
                .non_selected_mounted_shadow_dats
                .push(package_file(candidate));
        }
    }

    classification
        .selected_dats
        .sort_by(|left, right| left.physical_path.cmp(&right.physical_path));
    classification
        .missing_mapped_paths
        .sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    classification
        .unreferenced_selected_dats
        .sort_by(|left, right| left.physical_path.cmp(&right.physical_path));
    classification
        .non_selected_mounted_shadow_dats
        .sort_by(|left, right| left.physical_path.cmp(&right.physical_path));
    classification
        .r000100_unmounted_dats
        .sort_by(|left, right| left.physical_path.cmp(&right.physical_path));

    classification
}

fn select_xbox_candidate<'a>(
    dat_path: DatPath,
    candidates_by_path: &HashMap<String, &'a XboxPackageCandidate>,
) -> Option<&'a XboxPackageCandidate> {
    let logical_path = stable_relative_path(&dat_path.to_path());
    let package = xbox_package_name(xbox_package_index(dat_path));
    let preferred_path = format!("{}/{}", package, logical_path).to_ascii_lowercase();
    if let Some(candidate) = candidates_by_path.get(&preferred_path) {
        return Some(*candidate);
    }

    if package == XBOX_PACKAGE_SLOT_NAMES[0] {
        let fallback_path = format!("{}/{}", XBOX_BASE_PACKAGE, logical_path).to_ascii_lowercase();
        return candidates_by_path.get(&fallback_path).copied();
    }

    None
}

fn package_file(candidate: &XboxPackageCandidate) -> XboxPackageFile {
    XboxPackageFile {
        package: candidate.package.clone(),
        logical_path: stable_relative_path(&candidate.dat_path.to_path()),
        physical_path: candidate.physical_path.clone(),
    }
}

fn parse_dat_path(path: &Path) -> Option<DatPath> {
    let components = path.iter().collect::<Vec<_>>();
    let [_, rom, folder, file] = components.as_slice() else {
        return None;
    };
    let file = file.to_str()?;
    let folder = folder.to_str()?;
    let rom = rom.to_str()?;
    let (file_id, extension) = file.rsplit_once('.')?;
    if !extension.eq_ignore_ascii_case("DAT") {
        return None;
    }
    let file_id = file_id.parse().ok()?;
    let folder_id = folder.parse().ok()?;
    let rom = rom.to_ascii_uppercase();
    let rom_id = if rom == "ROM" {
        1
    } else if let Some(suffix) = rom.strip_prefix("ROM") {
        suffix.parse().ok()?
    } else {
        return None;
    };
    Some(DatPath {
        rom_id,
        folder_id,
        file_id,
    })
}

fn xbox_package_index(dat_path: DatPath) -> usize {
    // Xbox publisher packages are selected from the logical DAT path, not its table ID.
    (dat_path.rom_id as usize + dat_path.folder_id as usize * 128 + dat_path.file_id as usize) % 12
}

fn xbox_package_name(index: usize) -> &'static str {
    XBOX_PACKAGE_SLOT_NAMES[index]
}

fn xbox_missing_package_name(dat_path: DatPath) -> &'static str {
    let package = xbox_package_name(xbox_package_index(dat_path));
    if package == XBOX_PACKAGE_SLOT_NAMES[0] {
        XBOX_BASE_PACKAGE
    } else {
        package
    }
}

fn summarize_xbox_formats(mappings: &[XboxMappingResult]) -> Vec<XboxFormatResult> {
    let mut results = SUPPORTED_FORMATS
        .iter()
        .copied()
        .map(|format| XboxFormatResult {
            format: format.name().to_string(),
            selected: 0,
            recognized: 0,
            round_trip_ok: 0,
            failed: 0,
        })
        .collect::<Vec<_>>();
    let mut unrecognized = XboxFormatResult {
        format: "unrecognized".to_string(),
        selected: 0,
        recognized: 0,
        round_trip_ok: 0,
        failed: 0,
    };

    for mapping in mappings {
        if mapping.selected_path.is_none() {
            continue;
        }
        let Some(format_name) = mapping.format.as_deref() else {
            if matches!(mapping.status, AuditStatus::Unrecognized) {
                unrecognized.selected += 1;
            }
            continue;
        };
        let Some(result) = results
            .iter_mut()
            .find(|result| result.format == format_name)
        else {
            continue;
        };
        result.selected += 1;
        result.recognized += 1;
        if matches!(mapping.status, AuditStatus::Ok) {
            result.round_trip_ok += 1;
        }
        if matches!(mapping.status, AuditStatus::Failed) {
            result.failed += 1;
        }
    }

    results.push(unrecognized);
    results
}

fn audit_file(
    root: &Path,
    id: u32,
    mapped_ids: &[u32],
    formats_by_id: &BTreeMap<u32, Vec<FormatKind>>,
    relative_path: &Path,
    path: &PathBuf,
) -> DatAuditFile {
    let path_string = stable_relative_path(relative_path);

    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return DatAuditFile {
                id,
                path: path_string,
                status: AuditStatus::Missing,
                format: None,
                failure_kind: Some(FailureKind::Missing),
                error: Some(sanitize_error(&error.to_string(), root, path)),
            };
        }
    };
    if !metadata.is_file() {
        return failed_file(
            id,
            path_string,
            FailureKind::Read,
            "path is not a regular file".to_string(),
        );
    }

    let Some(format) = detect_format(path, mapped_ids, formats_by_id) else {
        return DatAuditFile {
            id,
            path: path_string,
            status: AuditStatus::Unrecognized,
            format: None,
            failure_kind: None,
            error: None,
        };
    };

    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return failed_recognized_file(
                id,
                path_string,
                format,
                FailureKind::Read,
                sanitize_error(&error.to_string(), root, path),
            );
        }
    };

    let encoded = match panic::catch_unwind(AssertUnwindSafe(|| format.round_trip(&bytes))) {
        Ok(Ok(encoded)) => encoded,
        Ok(Err(error)) => {
            return failed_recognized_file(
                id,
                path_string,
                format,
                error.kind,
                sanitize_error(&error.message, root, path),
            );
        }
        Err(payload) => {
            return failed_recognized_file(
                id,
                path_string,
                format,
                FailureKind::Parse,
                sanitize_error(
                    &format!("decoder panicked: {}", panic_message(payload)),
                    root,
                    path,
                ),
            );
        }
    };

    if encoded != bytes {
        let first_difference = bytes
            .iter()
            .zip(&encoded)
            .position(|(original, regenerated)| original != regenerated)
            .unwrap_or(bytes.len().min(encoded.len()));
        return failed_recognized_file(
            id,
            path_string,
            format,
            FailureKind::RoundTrip,
            format!(
                "encoded bytes differ (original length {}, encoded length {}, first difference {})",
                bytes.len(),
                encoded.len(),
                first_difference
            ),
        );
    }

    DatAuditFile {
        id,
        path: path_string,
        status: AuditStatus::Ok,
        format: Some(format.name().to_string()),
        failure_kind: None,
        error: None,
    }
}

fn failed_file(id: u32, path: String, kind: FailureKind, error: String) -> DatAuditFile {
    DatAuditFile {
        id,
        path,
        status: AuditStatus::Failed,
        format: None,
        failure_kind: Some(kind),
        error: Some(error),
    }
}

fn failed_recognized_file(
    id: u32,
    path: String,
    format: FormatKind,
    kind: FailureKind,
    error: String,
) -> DatAuditFile {
    DatAuditFile {
        id,
        path,
        status: AuditStatus::Failed,
        format: Some(format.name().to_string()),
        failure_kind: Some(kind),
        error: Some(error),
    }
}

fn summarize(files: &[DatAuditFile]) -> AuditSummary {
    let mut summary = AuditSummary {
        total: files.len(),
        ..Default::default()
    };

    for file in files {
        match file.status {
            AuditStatus::Ok => {
                summary.recognized += 1;
                summary.round_trip_ok += 1;
            }
            AuditStatus::Missing => summary.missing += 1,
            AuditStatus::Unrecognized => summary.unrecognized += 1,
            AuditStatus::Failed => {
                summary.failed += 1;
                if file.format.is_some() {
                    summary.recognized += 1;
                }
            }
        }
    }

    summary
}

fn stable_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sanitize_error(error: &str, root: &Path, path: &Path) -> String {
    let mut sanitized = error.replace('\\', "/");
    let mut replacements = vec![
        (path.to_string_lossy().replace('\\', "/"), "<dat>"),
        (root.to_string_lossy().replace('\\', "/"), "<ffxi-root>"),
    ];
    if let Ok(canonical_root) = fs::canonicalize(root) {
        replacements.push((
            canonical_root.to_string_lossy().replace('\\', "/"),
            "<ffxi-root>",
        ));
    }
    if let Ok(canonical_path) = fs::canonicalize(path) {
        replacements.push((canonical_path.to_string_lossy().replace('\\', "/"), "<dat>"));
    }

    replacements.sort_by_key(|(value, _)| std::cmp::Reverse(value.len()));
    for (value, replacement) in replacements {
        if !value.is_empty() {
            sanitized = sanitized.replace(&value, replacement);
        }
    }
    sanitized
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_recognized_and_failures() {
        let files = vec![
            DatAuditFile {
                id: 1,
                path: "ROM/1/1.DAT".to_string(),
                status: AuditStatus::Ok,
                format: Some("events".to_string()),
                failure_kind: None,
                error: None,
            },
            DatAuditFile {
                id: 2,
                path: "ROM/1/2.DAT".to_string(),
                status: AuditStatus::Missing,
                format: None,
                failure_kind: Some(FailureKind::Missing),
                error: Some("missing".to_string()),
            },
            DatAuditFile {
                id: 4,
                path: "ROM/1/4.DAT".to_string(),
                status: AuditStatus::Unrecognized,
                format: None,
                failure_kind: None,
                error: None,
            },
            DatAuditFile {
                id: 3,
                path: "ROM/1/3.DAT".to_string(),
                status: AuditStatus::Failed,
                format: Some("zone_data".to_string()),
                failure_kind: Some(FailureKind::Write),
                error: Some("Can't write DAT".to_string()),
            },
        ];

        let summary = summarize(&files);
        assert_eq!(summary.total, 4);
        assert_eq!(summary.missing, 1);
        assert_eq!(summary.recognized, 2);
        assert_eq!(summary.round_trip_ok, 1);
        assert_eq!(summary.unrecognized, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn errors_do_not_embed_absolute_root_or_dat_paths() {
        let root = PathBuf::from(r"C:\retail\FINAL FANTASY XI");
        let path = root.join(r"ROM\1\2.DAT");
        let error = format!("Could not read {} below {}", path.display(), root.display());

        let sanitized = sanitize_error(&error, &root, &path);
        assert!(!sanitized.contains("C:/retail"));
        assert!(sanitized.contains("<dat>"));
        assert!(sanitized.contains("<ffxi-root>"));
    }

    #[test]
    fn report_json_has_stable_schema_fields() {
        let report = AuditReport {
            schema_version: REPORT_SCHEMA_VERSION,
            summary: AuditSummary {
                total: 0,
                ..Default::default()
            },
            files: Vec::new(),
        };

        let json = serde_json::to_string(&report).unwrap();
        assert_eq!(
            json,
            r#"{"schema_version":1,"summary":{"total":0,"missing":0,"recognized":0,"round_trip_ok":0,"unrecognized":0,"failed":0},"files":[]}"#
        );
    }

    fn xbox_mapping(rom_id: u8, folder_id: u16, file_id: u16, ids: &[u32]) -> XboxMapping {
        XboxMapping {
            dat_path: DatPath {
                rom_id,
                folder_id,
                file_id,
            },
            ids: ids.to_vec(),
        }
    }

    fn xbox_candidate(
        package: &str,
        rom_id: u8,
        folder_id: u16,
        file_id: u16,
        physical_path: &str,
    ) -> XboxPackageCandidate {
        XboxPackageCandidate {
            package: package.to_string(),
            dat_path: DatPath {
                rom_id,
                folder_id,
                file_id,
            },
            physical_path: physical_path.to_string(),
        }
    }

    #[test]
    fn xbox_package_formula_covers_edges_and_package_zero() {
        assert_eq!(
            xbox_package_index(DatPath {
                rom_id: 1,
                folder_id: 0,
                file_id: 10
            }),
            11
        );
        assert_eq!(
            xbox_package_index(DatPath {
                rom_id: 1,
                folder_id: 0,
                file_id: 11
            }),
            0
        );
        assert_eq!(
            xbox_package_index(DatPath {
                rom_id: 1,
                folder_id: 1,
                file_id: 0
            }),
            9
        );
        assert_eq!(xbox_package_name(0), "R000100");
        assert_eq!(xbox_package_name(1), "R000101");
        assert_eq!(xbox_package_name(11), "R000111");
        assert_eq!(
            xbox_missing_package_name(DatPath {
                rom_id: 1,
                folder_id: 0,
                file_id: 11
            }),
            "0001"
        );
    }

    #[test]
    fn mapped_format_detection_covers_simple_japanese_zone_and_alias_ids() {
        let format_mappings = known_format_mappings();
        let formats_by_id = known_formats_by_id(&format_mappings);
        let missing_path = PathBuf::from("does-not-exist.DAT");
        assert!(matches!(
            detect_format(&missing_path, &[100], &formats_by_id),
            Some(FormatKind::ZoneData)
        ));
        assert!(matches!(
            detect_format(&missing_path, &[83_891], &formats_by_id),
            Some(FormatKind::ZoneData)
        ));
        assert!(matches!(
            detect_format(&missing_path, &[81], &formats_by_id),
            Some(FormatKind::MenuTable)
        ));
        assert!(matches!(
            detect_format(&missing_path, &[55_581], &formats_by_id),
            Some(FormatKind::DmsgTable)
        ));
        assert!(matches!(
            detect_format(&missing_path, &[100, 101], &formats_by_id),
            Some(FormatKind::ZoneData)
        ));
        assert!(detect_format(&missing_path, &[99], &formats_by_id).is_none());
    }

    #[test]
    fn client_format_mapping_projection_distinguishes_selected_missing_and_absent() {
        let format_mappings = vec![
            (1, FormatKind::Events),
            (2, FormatKind::Dialog),
            (3, FormatKind::ZoneData),
        ];
        let id_map = HashMap::from([
            (
                DatId::from(1),
                DatPath {
                    rom_id: 1,
                    folder_id: 0,
                    file_id: 11,
                },
            ),
            (
                DatId::from(2),
                DatPath {
                    rom_id: 1,
                    folder_id: 0,
                    file_id: 1,
                },
            ),
        ]);
        let selected_file = XboxPackageFile {
            package: "R000100".to_string(),
            logical_path: "ROM/0/11.DAT".to_string(),
            physical_path: "R000100/ROM/0/11.DAT".to_string(),
        };
        let selected_files = HashMap::from([(selected_file.logical_path.as_str(), &selected_file)]);

        let projected = project_client_format_mappings(&format_mappings, &id_map, &selected_files);

        assert_eq!(projected.len(), 3);
        assert_eq!(projected[0].status, XboxClientMappingStatus::Selected);
        assert_eq!(projected[0].package.as_deref(), Some("R000100"));
        assert_eq!(
            projected[0].selected_path.as_deref(),
            Some("R000100/ROM/0/11.DAT")
        );
        assert_eq!(projected[1].status, XboxClientMappingStatus::Missing);
        assert_eq!(projected[1].package.as_deref(), Some("R000102"));
        assert!(projected[1].selected_path.is_none());
        assert_eq!(projected[2].status, XboxClientMappingStatus::Absent);
        assert!(projected[2].logical_path.is_none());
        assert!(projected[2].package.is_none());
    }

    #[test]
    fn conflicting_mapped_formats_are_not_selected_arbitrarily() {
        let formats_by_id = BTreeMap::from([
            (1, vec![FormatKind::Events]),
            (2, vec![FormatKind::ZoneData]),
        ]);
        assert!(
            detect_format(
                &PathBuf::from("does-not-exist.DAT"),
                &[1, 2],
                &formats_by_id
            )
            .is_none()
        );
    }

    #[test]
    fn xbox_mapping_groups_aliases_and_sorts_ids() {
        let mut id_map = HashMap::new();
        id_map.insert(
            DatId::from(9),
            DatPath {
                rom_id: 1,
                folder_id: 0,
                file_id: 1,
            },
        );
        id_map.insert(
            DatId::from(3),
            DatPath {
                rom_id: 1,
                folder_id: 0,
                file_id: 0,
            },
        );
        id_map.insert(
            DatId::from(2),
            DatPath {
                rom_id: 1,
                folder_id: 0,
                file_id: 0,
            },
        );

        let mappings = group_xbox_mappings(&id_map);
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].ids, vec![2, 3]);
        assert_eq!(mappings[1].ids, vec![9]);
    }

    #[test]
    fn xbox_classification_prefers_package_zero_override_and_base_fallback() {
        let mappings = vec![
            xbox_mapping(1, 0, 11, &[20, 21]),
            xbox_mapping(1, 0, 23, &[22]),
            xbox_mapping(1, 0, 35, &[23]),
            xbox_mapping(1, 0, 0, &[30]),
        ];
        let candidates = vec![
            xbox_candidate("R000100", 1, 0, 11, "R000100/ROM/0/11.DAT"),
            xbox_candidate("R000101", 1, 0, 11, "R000101/ROM/0/11.DAT"),
            xbox_candidate("0001", 1, 0, 11, "0001/ROM/0/11.DAT"),
            xbox_candidate("0001", 1, 0, 23, "0001/ROM/0/23.DAT"),
            xbox_candidate("R000100", 1, 0, 0, "R000100/ROM/0/0.DAT"),
            xbox_candidate("R000101", 0, 0, 0, "R000101/ROM0/0/0.DAT"),
            xbox_candidate("0001", 0, 0, 0, "0001/ROM0/0/0.DAT"),
            xbox_candidate("R000101", 1, 0, 0, "R000101/ROM/0/0.DAT"),
        ];

        let classification = classify_xbox_candidates(&mappings, &candidates);
        assert_eq!(classification.selected_dats.len(), 3);
        assert_eq!(classification.missing_mapped_paths.len(), 1);
        assert_eq!(classification.unreferenced_selected_dats.len(), 1);
        assert_eq!(classification.non_selected_mounted_shadow_dats.len(), 3);
        assert_eq!(classification.r000100_unmounted_dats.len(), 1);
        assert_eq!(
            classification.selected_dats[0].physical_path,
            "0001/ROM/0/23.DAT"
        );
        assert_eq!(
            classification.selected_dats[1].physical_path,
            "R000100/ROM/0/11.DAT"
        );
        assert_eq!(
            classification.selected_dats[2].physical_path,
            "R000101/ROM/0/0.DAT"
        );
        assert_eq!(
            classification.missing_mapped_paths[0].expected_path,
            "0001/ROM/0/35.DAT"
        );
        assert_eq!(
            classification.r000100_unmounted_dats[0].physical_path,
            "R000100/ROM/0/0.DAT"
        );
    }

    #[test]
    fn xbox_classification_has_stable_relative_json_and_accounting() {
        let mappings = vec![xbox_mapping(1, 0, 0, &[2]), xbox_mapping(1, 0, 11, &[1, 3])];
        let candidates = vec![
            xbox_candidate("R000101", 1, 0, 0, "R000101/ROM/0/0.DAT"),
            xbox_candidate("0001", 1, 0, 11, "0001/ROM/0/11.DAT"),
        ];
        let classification = classify_xbox_candidates(&mappings, &candidates);
        let json = serde_json::to_string(&classification.selected_dats).unwrap();

        assert_eq!(classification.selected_dats.len(), 2);
        assert_eq!(classification.missing_mapped_paths.len(), 0);
        assert_eq!(
            classification.selected_dats[0].physical_path,
            "0001/ROM/0/11.DAT"
        );
        assert_eq!(
            classification.selected_dats[1].physical_path,
            "R000101/ROM/0/0.DAT"
        );
        assert_eq!(
            candidates.len(),
            classification.selected_dats.len()
                + classification.unreferenced_selected_dats.len()
                + classification.non_selected_mounted_shadow_dats.len()
                + classification.r000100_unmounted_dats.len()
        );
        assert!(!json.contains("C:\\"));
        assert!(!json.contains("<ffxi-root>"));
    }

    #[test]
    fn xbox_format_results_count_selected_unique_files() {
        let mappings = vec![
            XboxMappingResult {
                logical_path: "ROM/0/1.DAT".to_string(),
                ids: vec![1],
                package: "0001".to_string(),
                selected_path: Some("0001/ROM/0/1.DAT".to_string()),
                status: AuditStatus::Ok,
                format: Some("events".to_string()),
                failure_kind: None,
                error: None,
            },
            XboxMappingResult {
                logical_path: "ROM/0/2.DAT".to_string(),
                ids: vec![2, 3],
                package: "0001".to_string(),
                selected_path: Some("0001/ROM/0/2.DAT".to_string()),
                status: AuditStatus::Failed,
                format: Some("events".to_string()),
                failure_kind: Some(FailureKind::Parse),
                error: Some("synthetic failure".to_string()),
            },
            XboxMappingResult {
                logical_path: "ROM/0/4.DAT".to_string(),
                ids: vec![4],
                package: "0001".to_string(),
                selected_path: Some("0001/ROM/0/4.DAT".to_string()),
                status: AuditStatus::Unrecognized,
                format: None,
                failure_kind: None,
                error: None,
            },
            XboxMappingResult {
                logical_path: "ROM/0/5.DAT".to_string(),
                ids: vec![5],
                package: "0001".to_string(),
                selected_path: None,
                status: AuditStatus::Missing,
                format: None,
                failure_kind: Some(FailureKind::Missing),
                error: Some("missing".to_string()),
            },
        ];

        let formats = summarize_xbox_formats(&mappings);
        let events = formats
            .iter()
            .find(|result| result.format == "events")
            .unwrap();
        let unrecognized = formats
            .iter()
            .find(|result| result.format == "unrecognized")
            .unwrap();
        assert_eq!(events.selected, 2);
        assert_eq!(events.recognized, 2);
        assert_eq!(events.round_trip_ok, 1);
        assert_eq!(events.failed, 1);
        assert_eq!(unrecognized.selected, 1);
    }

    #[test]
    fn xbox_path_parser_rejects_tables_and_normalizes_dat_paths() {
        assert_eq!(
            parse_dat_path(Path::new("0001/ROM/12/34.dat")),
            Some(DatPath {
                rom_id: 1,
                folder_id: 12,
                file_id: 34,
            })
        );
        assert!(parse_dat_path(Path::new("0001/VTABLE.DAT")).is_none());
        assert!(parse_dat_path(Path::new("0001/ROM/not-a-folder/1.DAT")).is_none());
        assert!(parse_dat_path(Path::new("0001/extra/ROM/1/2.DAT")).is_none());
    }

    #[test]
    fn xbox_synthetic_report_is_private_stable_and_accounted() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-xbox-audit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("0001")).unwrap();
        fs::write(root.join("0001/VTABLE.DAT"), [1u8, 1, 1]).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), [11u8, 0, 11, 0, 0, 0]).unwrap();

        let write_dat = |relative_path: &str| {
            let path = root.join(relative_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"synthetic").unwrap();
        };
        write_dat("0001/ROM/0/11.DAT");
        write_dat("0001/ROM0/0/0.DAT");
        write_dat("R000101/ROM/0/0.DAT");
        write_dat("R000101/ROM/0/11.DAT");
        write_dat("R000100/ROM/0/11.DAT");

        let report_path = root.join("report.json");
        audit_xbox_packages(root.clone(), Some(report_path.clone())).unwrap();
        let first_report = fs::read_to_string(&report_path).unwrap();
        audit_xbox_packages(root.clone(), Some(report_path.clone())).unwrap();
        let second_report = fs::read_to_string(&report_path).unwrap();
        let report: serde_json::Value = serde_json::from_str(&first_report).unwrap();
        let summary = &report["summary"];

        assert_eq!(report["schema_version"], 4);
        assert_eq!(summary["mapped_ids"], 3);
        assert_eq!(summary["unique_mappings"], 2);
        assert_eq!(summary["duplicate_id_mappings"], 1);
        assert_eq!(summary["duplicate_id_entries"], 1);
        assert_eq!(summary["package_candidates"], 5);
        assert_eq!(summary["selected_dats"], 2);
        assert_eq!(summary["missing_mapped_paths"], 0);
        assert_eq!(summary["unreferenced_selected_dats"], 1);
        assert_eq!(summary["non_selected_mounted_shadow_dats"], 2);
        assert_eq!(summary["r000100_unmounted_dats"], 0);
        assert_eq!(summary["package_zero_overrides"], 1);
        assert_eq!(summary["package_zero_base_fallbacks"], 0);
        assert!(summary["client_format_mappings"].as_u64().unwrap() > 0);
        assert_eq!(summary["selected_client_format_mappings"], 0);
        assert_eq!(summary["missing_client_format_mappings"], 0);
        assert_eq!(
            summary["absent_client_format_mappings"],
            summary["client_format_mappings"]
        );
        assert_eq!(summary["unrecognized"], 2);
        assert_eq!(report["format_results"][14]["format"], "unrecognized");
        assert_eq!(report["format_results"][14]["selected"], 2);
        assert_eq!(report["mapped_ids"][0]["id"], 0);
        assert_eq!(report["mapped_ids"][1]["id"], 1);
        assert_eq!(report["mapped_ids"][2]["id"], 2);
        assert_eq!(
            report["client_format_mappings"].as_array().unwrap().len() as u64,
            summary["client_format_mappings"].as_u64().unwrap()
        );
        assert!(!first_report.contains(&root.to_string_lossy().to_string()));
        assert_eq!(first_report, second_report);

        fs::remove_dir_all(root).unwrap();
    }
}
