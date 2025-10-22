use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use encoding_rs::Encoding;
use log::info;

use super::DictionaryBuilder;
use crate::dictionary::types::{
    CharCategory, CharDefinitions, CodePointRange, ConnectionMatrix, DictEntry, UnknownEntries,
    UnknownEntry,
};

pub fn build_dictionary(builder: &DictionaryBuilder) -> Result<()> {
    info!("Starting dictionary build process");

    // Create output directory
    fs::create_dir_all(&builder.output_dir).context("Failed to create output directory")?;

    // 1. Parse CSV files into dictionary entries
    info!("Parsing dictionary entries from CSV files");
    let entries = parse_csv_files(&builder.mecab_dir, &builder.encoding)?;
    info!("Parsed {} dictionary entries", entries.len());

    // 2. Build FST mapping surface forms to index IDs and separate morpheme index
    info!("Building FST and morpheme index");
    let (fst_data, morpheme_index) = build_fst(&entries)?;

    // 3. Parse connection matrix
    info!("Parsing connection matrix");
    let connection_matrix = parse_matrix_def(&builder.mecab_dir, &builder.encoding)?;

    // 4. Parse character definitions
    info!("Parsing character definitions");
    let char_defs = parse_char_def(&builder.mecab_dir, &builder.encoding)?;

    // 5. Parse unknown word definitions
    info!("Parsing unknown word definitions");
    let unknowns = parse_unk_def(&builder.mecab_dir, &builder.encoding)?;

    // 6. Serialize all data to output directory
    info!("Serializing dictionary data");
    save_dictionary(
        &builder.output_dir,
        &fst_data,
        &morpheme_index,
        &entries,
        &connection_matrix,
        &char_defs,
        &unknowns,
    )?;

    info!("Dictionary build completed successfully");
    Ok(())
}

fn parse_csv_files(mecab_dir: &Path, encoding: &str) -> Result<Vec<DictEntry>> {
    let mut entries = Vec::new();

    // Find all CSV files in the directory
    let csv_pattern = mecab_dir.join("*.csv");
    let csv_files =
        glob::glob(csv_pattern.to_str().unwrap()).context("Failed to read CSV file pattern")?;

    // Get the encoding
    let encoding = Encoding::for_label(encoding.as_bytes()).context("Unknown encoding")?;

    for csv_file in csv_files {
        let csv_file = csv_file.context("Failed to get CSV file path")?;
        info!("Processing file: {:?}", csv_file);

        let file_content =
            fs::read(&csv_file).with_context(|| format!("Failed to read file: {:?}", csv_file))?;

        let (decoded, _, _) = encoding.decode(&file_content);

        for line in decoded.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() < 13 {
                continue; // Skip malformed lines
            }

            let entry = DictEntry {
                surface: fields[0].to_string(),
                left_id: fields[1].parse().context("Failed to parse left_id")?,
                right_id: fields[2].parse().context("Failed to parse right_id")?,
                cost: fields[3].parse().context("Failed to parse cost")?,
                part_of_speech: format!("{},{},{},{}", fields[4], fields[5], fields[6], fields[7]),
                inflection_type: fields[8].to_string(),
                inflection_form: fields[9].to_string(),
                base_form: fields[10].to_string(),
                reading: fields[11].to_string(),
                phonetic: fields[12].to_string(),
                morph_id: entries.len(), // Use current position as dictionary entry index
            };

            entries.push(entry);
        }
    }

    Ok(entries)
}

fn build_fst(entries: &[DictEntry]) -> Result<(Vec<u8>, Vec<Vec<u32>>)> {
    use std::collections::HashMap;

    // Group entries by surface form to handle duplicates
    let mut surface_groups: HashMap<String, Vec<u32>> = HashMap::new();
    for (id, entry) in entries.iter().enumerate() {
        surface_groups
            .entry(entry.surface.clone())
            .or_default()
            .push(id as u32);
    }

    // Create separate morpheme index for storing multiple morpheme IDs
    let mut morpheme_index: Vec<Vec<u32>> = Vec::new();

    // Create surface form to index ID mappings (instead of encoded morpheme IDs)
    let mut surface_to_index: Vec<(String, u64)> = surface_groups
        .iter()
        .map(|(surface, ids)| {
            // Store morpheme IDs in separate index, FST stores only the index ID
            let index_id = morpheme_index.len() as u64;
            morpheme_index.push(ids.clone());

            info!(
                "Surface '{}' → index {} with {} morpheme IDs: {:?}",
                surface,
                index_id,
                ids.len(),
                if ids.len() <= 10 {
                    ids.clone()
                } else {
                    ids[..5].to_vec()
                }
            );

            (surface.clone(), index_id)
        })
        .collect();

    // Sort by surface form (required for FST building)
    surface_to_index.sort_by(|a, b| a.0.cmp(&b.0));

    info!(
        "Building FST with {} unique surface forms, total entries: {}, morpheme index size: {}",
        surface_to_index.len(),
        entries.len(),
        morpheme_index.len()
    );

    // Build FST
    let mut builder = fst::MapBuilder::memory();
    for (surface, index_id) in surface_to_index {
        builder
            .insert(surface.as_bytes(), index_id)
            .context("Failed to insert into FST")?;
    }

    let fst_bytes = builder.into_inner().context("Failed to build FST")?;
    Ok((fst_bytes, morpheme_index))
}

fn parse_matrix_def(mecab_dir: &Path, encoding: &str) -> Result<ConnectionMatrix> {
    let matrix_file = mecab_dir.join("matrix.def");
    let encoding = Encoding::for_label(encoding.as_bytes()).context("Unknown encoding")?;

    let file_content = fs::read(&matrix_file).context("Failed to read matrix.def")?;

    let (decoded, _, _) = encoding.decode(&file_content);
    let mut lines = decoded.lines();

    // Read matrix dimensions
    let first_line = lines.next().context("Empty matrix.def file")?;
    let dims: Vec<&str> = first_line.split_whitespace().collect();
    if dims.len() != 2 {
        anyhow::bail!("Invalid matrix.def format: expected dimensions on first line");
    }

    let rows: usize = dims[0].parse().context("Failed to parse matrix rows")?;
    let cols: usize = dims[1].parse().context("Failed to parse matrix cols")?;

    // Initialize matrix
    let mut matrix = vec![vec![0i16; cols]; rows];

    // Parse connection costs
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            continue; // Skip malformed lines
        }

        let row: usize = parts[0].parse().context("Failed to parse row index")?;
        let col: usize = parts[1].parse().context("Failed to parse col index")?;
        let cost: i16 = parts[2].parse().context("Failed to parse cost")?;

        if row < rows && col < cols {
            matrix[row][col] = cost;
        }
    }

    Ok(matrix)
}

fn parse_char_def(mecab_dir: &Path, encoding: &str) -> Result<CharDefinitions> {
    let char_file = mecab_dir.join("char.def");
    let encoding = Encoding::for_label(encoding.as_bytes()).context("Unknown encoding")?;

    let file_content = fs::read(&char_file).context("Failed to read char.def")?;

    let (decoded, _, _) = encoding.decode(&file_content);

    let mut categories = HashMap::new();
    let mut code_ranges = Vec::new();

    for line in decoded.lines() {
        let line = line.trim().replace('\t', " ");
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("0x") {
            // Parse code point range
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let range_parts: Vec<&str> = parts[0].split("..").collect();
            let from_code = u32::from_str_radix(&range_parts[0][2..], 16)
                .context("Failed to parse from codepoint")?;
            let to_code = if range_parts.len() == 2 {
                u32::from_str_radix(&range_parts[1][2..], 16)
                    .context("Failed to parse to codepoint")?
            } else {
                from_code
            };

            let from_char = char::from_u32(from_code).context("Invalid from codepoint")?;
            let to_char = char::from_u32(to_code).context("Invalid to codepoint")?;

            let category = parts[1].to_string();
            let mut compat_categories = Vec::new();

            // Parse compatible categories
            for part in parts.iter().skip(2) {
                if part.starts_with('#') {
                    break;
                }
                if !part.is_empty() {
                    compat_categories.push(part.to_string());
                }
            }

            code_ranges.push(CodePointRange {
                from: from_char,
                to: to_char,
                category,
                compat_categories,
            });
        } else {
            // Parse category definition
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            let name = parts[0].to_string();
            let invoke = parts[1] == "1";
            let group = parts[2] == "1";
            let length = parts[3].parse().context("Failed to parse length")?;

            categories.insert(
                name,
                CharCategory {
                    invoke,
                    group,
                    length,
                },
            );
        }
    }

    Ok(CharDefinitions {
        categories,
        code_ranges,
    })
}

fn parse_unk_def(mecab_dir: &Path, encoding: &str) -> Result<UnknownEntries> {
    let unk_file = mecab_dir.join("unk.def");
    let encoding = Encoding::for_label(encoding.as_bytes()).context("Unknown encoding")?;

    let file_content = fs::read(&unk_file).context("Failed to read unk.def")?;

    let (decoded, _, _) = encoding.decode(&file_content);

    let mut unknowns = HashMap::new();

    for line in decoded.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 11 {
            continue; // Skip malformed lines
        }

        let category = fields[0].to_string();
        let left_id = fields[1].parse().context("Failed to parse left_id")?;
        let right_id = fields[2].parse().context("Failed to parse right_id")?;
        let cost = fields[3].parse().context("Failed to parse cost")?;
        let part_of_speech = format!("{},{},{},{}", fields[4], fields[5], fields[6], fields[7]);

        let entry = UnknownEntry {
            left_id,
            right_id,
            cost,
            part_of_speech,
        };

        unknowns
            .entry(category)
            .or_insert_with(Vec::new)
            .push(entry);
    }

    Ok(unknowns)
}

fn save_dictionary(
    output_dir: &Path,
    fst_data: &[u8],
    morpheme_index: &[Vec<u32>],
    entries: &[DictEntry],
    connection_matrix: &ConnectionMatrix,
    char_defs: &CharDefinitions,
    unknowns: &UnknownEntries,
) -> Result<()> {
    // Save FST
    let fst_path = output_dir.join("dic.fst");
    fs::write(&fst_path, fst_data).context("Failed to write FST file")?;

    // Save morpheme index (maps FST index IDs to vectors of morpheme IDs)
    let morpheme_index_path = output_dir.join("morpheme_index.bin");
    let encoded =
        bincode::serialize(morpheme_index).context("Failed to serialize morpheme index")?;
    fs::write(&morpheme_index_path, encoded).context("Failed to write morpheme index file")?;

    // Save dictionary entries
    let entries_path = output_dir.join("entries.bin");
    let encoded = bincode::serialize(entries).context("Failed to serialize entries")?;
    fs::write(&entries_path, encoded).context("Failed to write entries file")?;

    // Save connection matrix
    let connections_path = output_dir.join("connections.bin");
    let encoded =
        bincode::serialize(connection_matrix).context("Failed to serialize connection matrix")?;
    fs::write(&connections_path, encoded).context("Failed to write connections file")?;

    // Save packed connection matrix for zero-copy loading
    let connections_pack_path = output_dir.join(crate::dictionary::loader::CONNECTIONS_PACK_FILE);
    write_connections_pack(&connections_pack_path, connection_matrix)
        .context("Failed to write connections pack file")?;

    // Save packed morpheme index for zero-copy loading
    let morpheme_pack_path = output_dir.join(crate::dictionary::loader::MORPHEME_PACK_FILE);
    write_morpheme_index_pack(&morpheme_pack_path, morpheme_index)
        .context("Failed to write morpheme index pack file")?;

    // Save character definitions
    let char_defs_path = output_dir.join("char_defs.bin");
    let encoded = bincode::serialize(char_defs).context("Failed to serialize char definitions")?;
    fs::write(&char_defs_path, encoded).context("Failed to write char definitions file")?;

    // Save unknown word definitions
    let unknowns_path = output_dir.join("unknowns.bin");
    let encoded = bincode::serialize(unknowns).context("Failed to serialize unknown entries")?;
    fs::write(&unknowns_path, encoded).context("Failed to write unknowns file")?;

    info!("Dictionary files saved to: {:?}", output_dir);
    Ok(())
}

fn write_connections_pack(path: &Path, matrix: &ConnectionMatrix) -> Result<()> {
    let rows = matrix.len();
    let cols = if rows > 0 { matrix[0].len() } else { 0 };

    ensure!(
        rows <= u32::MAX as usize && cols <= u32::MAX as usize,
        "Connection matrix dimensions exceed supported range: {}x{}",
        rows,
        cols
    );

    let mut file = fs::File::create(path).context("Failed to create connections pack file")?;
    file.write_all(crate::dictionary::loader::CONNECTIONS_PACK_MAGIC)
        .context("Failed to write connections pack magic")?;
    file.write_all(&crate::dictionary::loader::CONNECTIONS_PACK_VERSION.to_le_bytes())
        .context("Failed to write connections pack version")?;
    file.write_all(&(rows as u32).to_le_bytes())
        .context("Failed to write connections pack rows")?;
    file.write_all(&(cols as u32).to_le_bytes())
        .context("Failed to write connections pack cols")?;

    for row in matrix {
        ensure!(
            row.len() == cols,
            "Connection matrix row length mismatch: expected {}, got {}",
            cols,
            row.len()
        );
        for &cost in row {
            file.write_all(&cost.to_le_bytes())
                .context("Failed to write connection cost")?;
        }
    }

    Ok(())
}

fn write_morpheme_index_pack(path: &Path, index: &[Vec<u32>]) -> Result<()> {
    let entry_count = index.len();

    ensure!(
        entry_count <= u32::MAX as usize,
        "Morpheme index entry count exceeds supported range: {}",
        entry_count
    );

    let mut offsets = Vec::with_capacity(entry_count + 1);
    offsets.push(0u32);
    let mut values = Vec::new();

    for row in index {
        ensure!(
            row.len() <= u32::MAX as usize,
            "Morpheme index row length exceeds supported range: {}",
            row.len()
        );
        let next_offset = offsets
            .last()
            .copied()
            .and_then(|prev| prev.checked_add(row.len() as u32))
            .ok_or_else(|| anyhow::anyhow!("Morpheme index offsets overflow"))?;
        offsets.push(next_offset);
        values.extend(row.iter().copied());
    }

    ensure!(
        values.len() <= u32::MAX as usize,
        "Morpheme index value count exceeds supported range: {}",
        values.len()
    );

    let final_offset = *offsets.last().unwrap_or(&0);
    ensure!(
        final_offset == values.len() as u32,
        "Morpheme index offsets mismatch: final offset {} != value count {}",
        final_offset,
        values.len()
    );

    let mut file = fs::File::create(path).context("Failed to create morpheme index pack file")?;
    file.write_all(crate::dictionary::loader::MORPHEME_PACK_MAGIC)
        .context("Failed to write morpheme index pack magic")?;
    file.write_all(&crate::dictionary::loader::MORPHEME_PACK_VERSION.to_le_bytes())
        .context("Failed to write morpheme index pack version")?;
    file.write_all(&(entry_count as u32).to_le_bytes())
        .context("Failed to write morpheme index entry count")?;
    file.write_all(&(values.len() as u32).to_le_bytes())
        .context("Failed to write morpheme index value count")?;

    for offset in offsets {
        file.write_all(&offset.to_le_bytes())
            .context("Failed to write morpheme index offset")?;
    }

    for value in values {
        file.write_all(&value.to_le_bytes())
            .context("Failed to write morpheme index value")?;
    }

    Ok(())
}
