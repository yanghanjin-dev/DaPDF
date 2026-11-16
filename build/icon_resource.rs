//! Emit a single-section Windows COFF resource object without an SDK or windres.
//! Resource 1 is an RT_GROUP_ICON referencing the ICO's individual RT_ICON images.
use std::fs;
use std::path::Path;

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn put16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn directory(data: &mut Vec<u8>, ids: &[u32]) -> Vec<usize> {
    let base = data.len();
    data.resize(base + 16 + ids.len() * 8, 0);
    put16(data, base + 14, ids.len() as u16);
    ids.iter().enumerate().map(|(i, id)| {
        let entry = base + 16 + i * 8;
        put32(data, entry, *id);
        entry + 4
    }).collect()
}

pub fn compile(icon: &Path, output: &Path, arch: &str) {
    let ico = fs::read(icon).expect("Read DaPDF icon");
    assert_eq!(u16_at(&ico, 0), 0, "Invalid ICO reserved field");
    assert_eq!(u16_at(&ico, 2), 1, "Expected ICO image");
    let count = u16_at(&ico, 4) as usize;
    assert!(count > 0 && 6 + count * 16 <= ico.len(), "Invalid ICO directory");
    let mut group = ico[..6].to_vec();
    let mut images = Vec::new();
    for i in 0..count {
        let entry = 6 + i * 16;
        let size = u32_at(&ico, entry + 8) as usize;
        let start = u32_at(&ico, entry + 12) as usize;
        images.push(ico[start..start + size].to_vec());
        group.extend_from_slice(&ico[entry..entry + 12]);
        group.extend_from_slice(&((i + 1) as u16).to_le_bytes());
    }

    let mut data = Vec::new();
    let roots = directory(&mut data, &[3, 14]); // RT_ICON, RT_GROUP_ICON
    let mut leaves = Vec::new();
    for (type_index, ids) in [(0, (1..=count as u32).collect::<Vec<_>>()), (1, vec![1])] {
        let offset = data.len() as u32;
        put32(&mut data, roots[type_index], offset | 0x8000_0000);
        let entries = directory(&mut data, &ids);
        for entry in entries {
            let offset = data.len() as u32;
            put32(&mut data, entry, offset | 0x8000_0000);
            leaves.push(directory(&mut data, &[0])[0]); // language-neutral
        }
    }
    images.push(group);
    let mut relocations = Vec::new();
    for (leaf, image) in leaves.into_iter().zip(images) {
        let offset = data.len();
        put32(&mut data, leaf, offset as u32);
        data.resize(offset + 16, 0); // IMAGE_RESOURCE_DATA_ENTRY
        let payload_offset = data.len() as u32;
        put32(&mut data, offset, payload_offset);
        put32(&mut data, offset + 4, image.len() as u32);
        relocations.push(offset as u32);
        data.extend_from_slice(&image);
        while data.len() % 4 != 0 { data.push(0); }
    }

    let (machine, relocation_type) = match arch {
        "x86_64" => (0x8664, 0x0003), // IMAGE_REL_AMD64_ADDR32NB
        "x86" => (0x014c, 0x0007),    // IMAGE_REL_I386_DIR32NB
        "aarch64" => (0xaa64, 0x0002), // IMAGE_REL_ARM64_ADDR32NB
        _ => panic!("Unsupported Windows icon target: {arch}"),
    };
    let reloc_offset = 60 + data.len();
    let symbol_offset = reloc_offset + relocations.len() * 10;
    let mut object = vec![0; 60]; // COFF header + one section header
    put16(&mut object, 0, machine);
    put16(&mut object, 2, 1);
    put32(&mut object, 8, symbol_offset as u32);
    put32(&mut object, 12, 1);
    object[20..25].copy_from_slice(b".rsrc");
    put32(&mut object, 36, data.len() as u32);
    put32(&mut object, 40, 60);
    put32(&mut object, 44, reloc_offset as u32);
    put16(&mut object, 52, relocations.len() as u16);
    put32(&mut object, 56, 0x4030_0040); // initialized, readable, 4-byte alignment
    object.extend_from_slice(&data);
    for offset in relocations {
        object.extend_from_slice(&offset.to_le_bytes());
        object.extend_from_slice(&0u32.to_le_bytes()); // section symbol index
        object.extend_from_slice(&(relocation_type as u16).to_le_bytes());
    }
    let mut symbol = [0; 18];
    symbol[..5].copy_from_slice(b".rsrc");
    put16(&mut symbol, 12, 1);
    symbol[16] = 3; // IMAGE_SYM_CLASS_STATIC
    object.extend_from_slice(&symbol);
    object.extend_from_slice(&4u32.to_le_bytes()); // empty COFF string table
    fs::write(output, object).expect("Write DaPDF icon resource object");
}
