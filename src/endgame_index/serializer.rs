use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use memmap2::Mmap;

use crate::endgame_index::model::{FeatureIndexHeader, GameFeatureRecord, HEADER_SIZE};

pub struct FeatureIndexWriter {
    writer: BufWriter<File>,
    path: PathBuf,
    header: FeatureIndexHeader,
    written_records: u32,
}

impl FeatureIndexWriter {
    pub fn create(
        path: impl AsRef<Path>,
        catalog_version: u32,
        endgame_bit_count: u16,
        db_mtime_secs: u64,
        db_file_size: u64,
    ) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::create(&path_buf)
            .with_context(|| format!("Failed to create feature index at {:?}", path_buf))?;
        let mut writer = BufWriter::with_capacity(512 * 1024, file);

        let created_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let header = FeatureIndexHeader::new(
            catalog_version,
            0,
            endgame_bit_count,
            db_mtime_secs,
            db_file_size,
            created_timestamp,
        );

        // Write placeholder header
        writer.write_all(&header.to_bytes())?;

        Ok(Self {
            writer,
            path: path_buf,
            header,
            written_records: 0,
        })
    }

    #[inline]
    pub fn write_record(&mut self, record: &GameFeatureRecord) -> Result<()> {
        let bytes = record.endgame_bits.to_le_bytes();
        self.writer.write_all(&bytes)?;
        self.written_records += 1;
        Ok(())
    }

    pub fn write_records_slice(&mut self, records: &[GameFeatureRecord]) -> Result<()> {
        for record in records {
            self.write_record(record)?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<u32> {
        self.writer.flush()?;
        drop(self.writer);

        // Rewrite final header with actual game_count
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)?;

        self.header.game_count = self.written_records;
        file.write_all(&self.header.to_bytes())?;
        file.flush()?;

        Ok(self.written_records)
    }
}

pub fn resolve_companion_feat_path<P: AsRef<Path>>(db_path: P) -> PathBuf {
    let p = db_path.as_ref();
    let path_str = p.to_string_lossy();
    let lower = path_str.to_lowercase();
    if lower.ends_with(".pgn") {
        let direct = PathBuf::from(format!("{}.feat.idx", path_str));
        if direct.exists() {
            return direct;
        }
        let stem = p.with_extension("feat.idx");
        if stem.exists() {
            return stem;
        }
        return direct;
    }
    if lower.ends_with(".si5")
        || lower.ends_with(".si4")
        || lower.ends_with(".sg5")
        || lower.ends_with(".sg4")
    {
        let direct = PathBuf::from(format!("{}.feat.idx", path_str));
        if direct.exists() {
            return direct;
        }
        return p.with_extension("feat.idx");
    }
    PathBuf::from(format!("{}.feat.idx", path_str))
}

pub struct MmapFeatureIndex {
    mmap: Arc<Mmap>,
    header: FeatureIndexHeader,
}

impl MmapFeatureIndex {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)
            .with_context(|| format!("Failed to open feature index at {:?}", path_ref))?;
        let metadata = file.metadata()?;
        let file_len = metadata.len();

        if file_len < HEADER_SIZE as u64 {
            bail!("Feature index file too short: {} bytes", file_len);
        }

        let mmap = unsafe { Mmap::map(&file)? };
        let header = FeatureIndexHeader::from_bytes(&mmap[0..HEADER_SIZE])
            .ok_or_else(|| anyhow::anyhow!("Invalid feature index header magic or format"))?;

        let expected_size =
            HEADER_SIZE as u64 + (header.game_count as u64 * header.record_size as u64);
        if file_len < expected_size {
            bail!(
                "Feature index file size mismatch: got {} bytes, expected at least {} bytes for {} games",
                file_len,
                expected_size,
                header.game_count
            );
        }

        Ok(Self {
            mmap: Arc::new(mmap),
            header,
        })
    }

    #[inline]
    pub fn header(&self) -> &FeatureIndexHeader {
        &self.header
    }

    #[inline]
    pub fn game_count(&self) -> u32 {
        self.header.game_count
    }

    #[inline]
    pub fn get_record(&self, game_id: u32) -> Option<GameFeatureRecord> {
        if game_id >= self.header.game_count {
            return None;
        }

        let offset = HEADER_SIZE + (game_id as usize * self.header.record_size as usize);
        let slice = &self.mmap[offset..offset + 8];
        let endgame_bits = u64::from_le_bytes(slice[0..8].try_into().unwrap());

        Some(GameFeatureRecord { endgame_bits })
    }

    pub fn find_games_with_endgame_mask(&self, mask: u64) -> Vec<u32> {
        let mut matches = Vec::new();
        let count = self.header.game_count;
        let stride = self.header.record_size as usize;

        for gid in 0..count {
            let offset = HEADER_SIZE + (gid as usize * stride);
            let bits = u64::from_le_bytes(self.mmap[offset..offset + 8].try_into().unwrap());
            if (bits & mask) == mask {
                matches.push(gid);
            }
        }

        matches
    }

    pub fn count_all_endgame_features(&self) -> Vec<u32> {
        let bit_count = self.header.endgame_bit_count.min(64) as usize;
        let mut counts = vec![0u32; bit_count];
        let count = self.header.game_count;
        let stride = self.header.record_size as usize;

        for gid in 0..count {
            let offset = HEADER_SIZE + (gid as usize * stride);
            let bits = u64::from_le_bytes(self.mmap[offset..offset + 8].try_into().unwrap());
            for (bit, item) in counts.iter_mut().enumerate() {
                if (bits & (1u64 << bit)) != 0 {
                    *item += 1;
                }
            }
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_feature_index_writer_and_mmap_reader() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let mut writer = FeatureIndexWriter::create(&path, 2, 47, 1000, 2000).unwrap();

        let mut rec1 = GameFeatureRecord::new();
        rec1.set_endgame_bit(0);
        rec1.set_endgame_bit(5);

        let mut rec2 = GameFeatureRecord::new();
        rec2.set_endgame_bit(5);
        rec2.set_endgame_bit(18);

        let mut rec3 = GameFeatureRecord::new();
        rec3.set_endgame_bit(46);

        writer.write_record(&rec1).unwrap();
        writer.write_record(&rec2).unwrap();
        writer.write_record(&rec3).unwrap();

        let written = writer.finish().unwrap();
        assert_eq!(written, 3);

        let mmap_idx = MmapFeatureIndex::open(&path).unwrap();
        assert_eq!(mmap_idx.game_count(), 3);
        assert_eq!(mmap_idx.header().catalog_version, 2);
        assert_eq!(mmap_idx.header().endgame_bit_count, 47);

        assert_eq!(mmap_idx.get_record(0).unwrap(), rec1);
        assert_eq!(mmap_idx.get_record(1).unwrap(), rec2);
        assert_eq!(mmap_idx.get_record(2).unwrap(), rec3);
        assert_eq!(mmap_idx.get_record(3), None);

        let matches_bit5 = mmap_idx.find_games_with_endgame_mask(1u64 << 5);
        assert_eq!(matches_bit5, vec![0, 1]);

        let matches_bit0 = mmap_idx.find_games_with_endgame_mask(1u64 << 0);
        assert_eq!(matches_bit0, vec![0]);

        let counts = mmap_idx.count_all_endgame_features();
        assert_eq!(counts[0], 1);
        assert_eq!(counts[5], 2);
        assert_eq!(counts[18], 1);
        assert_eq!(counts[46], 1);
    }
}
