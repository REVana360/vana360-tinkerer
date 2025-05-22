use dats::formats::zone_data::zone_model::ZoneMesh;

const HEADER_SIZE: u32 = size_of::<u16>() as u32 // Grid width (u16)
        + size_of::<u16>() as u32 // Grid height (u16)
        + 2*size_of::<u32>() as u32 // Placement and block offsets (2x u32)
        + 2*size_of::<u16>() as u32 // Placement and block counts (2x u16)
        + size_of::<u32>() as u32; // Zone meta

pub fn get_ximesh_bytes(zone_mesh: &ZoneMesh) -> Vec<u8> {
    // Grid cell offsets
    let cell_offsets_size =
        size_of::<u32>() as u32 * zone_mesh.grid_width as u32 * zone_mesh.grid_height as u32;

    // Grid cell entries
    let cell_entries_size = zone_mesh
        .mesh
        .grid_cells
        .iter()
        .map(|e| {
            // Cell info (u32) + Entry count (u16) + 2x u32 offsets for each entry
            size_of::<u32>() + size_of::<u16>() + e.indices.len() * size_of::<u32>() * 2
        })
        .sum::<usize>() as u32;

    let blocks_offset = align_size(HEADER_SIZE + cell_offsets_size + cell_entries_size) as u32;

    let mut mesh_data = Vec::with_capacity(blocks_offset as usize);
    mesh_data.resize(blocks_offset as usize, 0u8);

    let mut block_offsets = Vec::with_capacity(zone_mesh.mesh.blocks.len());

    for block in &zone_mesh.mesh.blocks {
        // Store offset of this block
        let offset = mesh_data.len() as u32;
        block_offsets.push(offset);

        // Lengths of each kind of bytes
        mesh_data.extend((block.vertices.len() as u16).to_le_bytes());
        mesh_data.extend((block.triangles.len() as u16).to_le_bytes());
        mesh_data.extend((block.flags as u16).to_le_bytes());
        mesh_data.extend(0u16.to_le_bytes()); // Padding

        // Vertices bytes
        for vertex in &block.vertices {
            mesh_data.extend(vertex.x.to_le_bytes());
            mesh_data.extend(vertex.y.to_le_bytes());
            mesh_data.extend(vertex.z.to_le_bytes());
        }

        // Triangle vertex bytes
        for tri in &block.triangles {
            mesh_data.extend(tri.vertex1_idx.to_le_bytes());
            mesh_data.extend(tri.vertex2_idx.to_le_bytes());
            mesh_data.extend(tri.vertex3_idx.to_le_bytes());
        }
        add_padding(&mut mesh_data);

        // Metadata bytes
        for tri in &block.triangles {
            let mut metadata = tri.material;
            // Material takes up only the 4 lower bits, so we can fill the rest with more metadata
            if tri.is_barrier {
                metadata |= 1 << 4;
            }

            mesh_data.push(metadata);
        }

        add_padding(&mut mesh_data);
    }

    let mut placement_offsets = Vec::with_capacity(zone_mesh.mesh.placements.len());
    for placement in &zone_mesh.mesh.placements {
        // Store offset of this placement
        placement_offsets.push(mesh_data.len() as u32);

        // Data field containing the map ID
        mesh_data.extend(placement.data_field.to_le_bytes());

        // Transformation matrix
        placement.o2w.iter().for_each(|row| {
            row.iter().for_each(|value| {
                mesh_data.extend(value.to_le_bytes());
            });
        });
    }

    // Grid dimensions
    mesh_data[0..2].copy_from_slice(&(zone_mesh.grid_width as u16).to_le_bytes());
    mesh_data[2..4].copy_from_slice(&(zone_mesh.grid_height as u16).to_le_bytes());

    // Data offsets and counts
    mesh_data[4..8].copy_from_slice(&(block_offsets[0] as u32).to_le_bytes());
    mesh_data[8..12].copy_from_slice(&(placement_offsets[0] as u32).to_le_bytes());
    mesh_data[12..14].copy_from_slice(&(block_offsets.len() as u16).to_le_bytes());
    mesh_data[14..16].copy_from_slice(&(placement_offsets.len() as u16).to_le_bytes());

    // Zone meta
    let zone_meta: u32 = if zone_mesh.wide_grid_search() { 1 } else { 0 };
    mesh_data[16..20].copy_from_slice(&zone_meta.to_le_bytes());

    let mut cell_offset_offset = HEADER_SIZE as usize;
    let mut cell_data_offset = cell_offset_offset + cell_offsets_size as usize;

    for cell in &zone_mesh.mesh.grid_cells {
        if cell.indices.is_empty() {
            // Skip this cell if there's no entries, and thus leaving the offset at 0.
            cell_offset_offset += size_of::<u32>();
            continue;
        }

        // Set the offset to this cell data
        mesh_data[cell_offset_offset..cell_offset_offset + size_of::<u32>()]
            .copy_from_slice(&(cell_data_offset as u32).to_le_bytes());
        cell_offset_offset += size_of::<u32>();

        // Info entry
        mesh_data[cell_data_offset..cell_data_offset + size_of::<u32>()]
            .copy_from_slice(&cell.info.to_le_bytes());
        cell_data_offset += size_of::<u32>();

        // Indices count
        mesh_data[cell_data_offset..cell_data_offset + size_of::<u16>()]
            .copy_from_slice(&(cell.indices.len() as u16).to_le_bytes());
        cell_data_offset += size_of::<u16>();

        for indices in &cell.indices {
            // Block offset
            let block_offset = block_offsets[indices.block_idx as usize];
            mesh_data[cell_data_offset..cell_data_offset + size_of::<u32>()]
                .copy_from_slice(&block_offset.to_le_bytes());
            cell_data_offset += size_of::<u32>();

            // Placement offset
            let placement_offset = placement_offsets[indices.placement_idx as usize];
            mesh_data[cell_data_offset..cell_data_offset + size_of::<u32>()]
                .copy_from_slice(&placement_offset.to_le_bytes());
            cell_data_offset += size_of::<u32>();
        }
    }

    mesh_data.shrink_to_fit();
    mesh_data
}

#[inline]
fn get_padding(len: u32) -> u32 {
    (4 - (len & 3)) & 3
}

#[inline]
fn align_size(size: u32) -> u32 {
    size + get_padding(size) as u32
}

#[inline]
fn add_padding(bytes: &mut Vec<u8>) {
    let padding = get_padding(bytes.len() as u32);
    for _ in 0..padding {
        bytes.push(0);
    }
}
