use std::io::Cursor;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nbt_rust::{
    read_tag, write_tag, CompoundTag, ListTag, LittleEndian, NetworkLittleEndian, RootTag, Tag,
    TagType,
};

fn int_list(values: &[i32]) -> Tag {
    Tag::List(ListTag::new(TagType::Int, values.iter().copied().map(Tag::Int).collect()).unwrap())
}

fn build_small_root() -> RootTag {
    let mut root = CompoundTag::new();
    root.insert("name".to_string(), Tag::String("Steve".to_string()));
    root.insert("health".to_string(), Tag::Int(20));
    root.insert("pos".to_string(), int_list(&[120, 64, -32]));
    root.insert(
        "flags".to_string(),
        Tag::ByteArray(vec![1, 0, 1, 1, 0, 1, 0, 1]),
    );
    RootTag::new("small", Tag::Compound(root))
}

fn build_medium_root() -> RootTag {
    let mut root = CompoundTag::new();
    root.insert("name".to_string(), Tag::String("MediumFixture".to_string()));
    root.insert(
        "blob".to_string(),
        Tag::ByteArray((0..2048usize).map(|i| (i % 251) as u8).collect()),
    );
    root.insert(
        "scores".to_string(),
        Tag::IntArray((0..512i32).map(|v| v * 3 - 7).collect()),
    );
    root.insert(
        "history".to_string(),
        Tag::LongArray((0..256i64).map(|v| v * 13 - 3).collect()),
    );
    let list = (0..64i32)
        .map(|i| {
            let mut entry = CompoundTag::new();
            entry.insert("id".to_string(), Tag::Int(i));
            entry.insert("label".to_string(), Tag::String(format!("entry_{i}")));
            Tag::Compound(entry)
        })
        .collect::<Vec<_>>();
    root.insert(
        "entries".to_string(),
        Tag::List(ListTag::new(TagType::Compound, list).unwrap()),
    );
    RootTag::new("medium", Tag::Compound(root))
}

fn build_large_root() -> RootTag {
    let mut root = CompoundTag::new();
    root.insert("name".to_string(), Tag::String("LargeFixture".to_string()));
    root.insert(
        "blob".to_string(),
        Tag::ByteArray((0..32768usize).map(|i| (i % 255) as u8).collect()),
    );
    root.insert(
        "scores".to_string(),
        Tag::IntArray((0..8192i32).map(|v| v * 5 - 11).collect()),
    );
    root.insert(
        "history".to_string(),
        Tag::LongArray((0..4096i64).map(|v| v * 17 - 5).collect()),
    );
    let list = (0..256i32)
        .map(|i| {
            let mut entry = CompoundTag::new();
            entry.insert("id".to_string(), Tag::Int(i));
            entry.insert("name".to_string(), Tag::String(format!("node_{i}")));
            entry.insert("values".to_string(), int_list(&[i, i + 1, i + 2]));
            Tag::Compound(entry)
        })
        .collect::<Vec<_>>();
    root.insert(
        "entries".to_string(),
        Tag::List(ListTag::new(TagType::Compound, list).unwrap()),
    );
    RootTag::new("large", Tag::Compound(root))
}

fn bench_encode_decode_le(c: &mut Criterion) {
    let fixtures = [
        ("small", build_small_root()),
        ("medium", build_medium_root()),
        ("large", build_large_root()),
    ];

    let mut encode_group = c.benchmark_group("encode_le");
    for (name, root) in &fixtures {
        encode_group.bench_function(BenchmarkId::from_parameter(name), |b| {
            b.iter(|| {
                let mut out = Vec::new();
                write_tag::<LittleEndian, _>(&mut out, black_box(root)).unwrap();
                black_box(out);
            });
        });
    }
    encode_group.finish();

    let mut decode_group = c.benchmark_group("decode_le");
    for (name, root) in &fixtures {
        let mut encoded = Vec::new();
        write_tag::<LittleEndian, _>(&mut encoded, root).unwrap();
        decode_group.throughput(Throughput::Bytes(encoded.len() as u64));
        decode_group.bench_function(BenchmarkId::from_parameter(name), |b| {
            b.iter(|| {
                let mut cursor = Cursor::new(encoded.as_slice());
                let decoded = read_tag::<LittleEndian, _>(&mut cursor).unwrap();
                black_box(decoded);
            });
        });
    }
    decode_group.finish();
}

fn bench_encode_decode_network(c: &mut Criterion) {
    let fixtures = [
        ("small", build_small_root()),
        ("medium", build_medium_root()),
        ("large", build_large_root()),
    ];

    let mut encode_group = c.benchmark_group("encode_network");
    for (name, root) in &fixtures {
        encode_group.bench_function(BenchmarkId::from_parameter(name), |b| {
            b.iter(|| {
                let mut out = Vec::new();
                write_tag::<NetworkLittleEndian, _>(&mut out, black_box(root)).unwrap();
                black_box(out);
            });
        });
    }
    encode_group.finish();

    let mut decode_group = c.benchmark_group("decode_network");
    for (name, root) in &fixtures {
        let mut encoded = Vec::new();
        write_tag::<NetworkLittleEndian, _>(&mut encoded, root).unwrap();
        decode_group.throughput(Throughput::Bytes(encoded.len() as u64));
        decode_group.bench_function(BenchmarkId::from_parameter(name), |b| {
            b.iter(|| {
                let mut cursor = Cursor::new(encoded.as_slice());
                let decoded = read_tag::<NetworkLittleEndian, _>(&mut cursor).unwrap();
                black_box(decoded);
            });
        });
    }
    decode_group.finish();
}

criterion_group!(benches, bench_encode_decode_le, bench_encode_decode_network);
criterion_main!(benches);
