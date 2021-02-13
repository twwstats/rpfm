use super::*;
use serde_json::to_string_pretty;

use std::fs::File;
use std::io::{BufReader, BufWriter, SeekFrom, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rpfm_error::{ErrorKind, Result};

use crate::SETTINGS;
use crate::common::{decoder::Decoder, encoder::Encoder};
use crate::packedfile::PackedFileType;

const PATH_FILE_INDEX_PATH_OFFSET: usize = 4;
const TIMESTAMP_SIZE: usize = 8;
const HEADER_SIZE: usize = 32;

impl PackFile {

    /// This function reads the content of a PackFile into a `PackFile` struct.
    pub fn read_pfh3(
        &mut self,
        mut pack_file: BufReader<File>,
        types_to_load: &Option<Vec<PackedFileType>>,
        use_lazy_loading: bool
    ) -> Result<()> {

        // Read the rest of the header, skipping already read data.
        let mut buffer = vec![0; HEADER_SIZE];
        let pack_file_len = pack_file.get_ref().metadata()?.len();
        if (pack_file_len as usize) < buffer.capacity() {
            return Err(ErrorKind::PackFileHeaderNotComplete.into())
        }

        pack_file.seek(SeekFrom::Start(0))?;
        pack_file.read_exact(&mut buffer)?;

        let pack_file_count = buffer.decode_integer_u32(8)?;
        let pack_file_index_size = buffer.decode_integer_u32(12)?;
        let packed_file_count = buffer.decode_integer_u32(16)?;
        let packed_file_index_size = buffer.decode_integer_u32(20)?;

        self.timestamp = (buffer.decode_integer_i64(24)? / WINDOWS_TICK) - SEC_TO_UNIX_EPOCH;

        // Ensure the PackFile has all the data needed for the index. If the PackFile's data is encrypted
        // and the PackFile is PFH5, due to how the encryption works, the data should start in a multiple of 8.
        let mut data_position = u64::from(buffer.len() as u32 + pack_file_index_size + packed_file_index_size);

        if pack_file_len < data_position { return Err(ErrorKind::PackFileIndexesNotComplete.into()) }

        // Create the buffers for the indexes data.
        let mut pack_file_index = vec![0; pack_file_index_size as usize];
        let mut packed_file_index = vec![0; packed_file_index_size as usize];

        // Get the data from both indexes to their buffers.
        pack_file.read_exact(&mut pack_file_index)?;
        pack_file.read_exact(&mut packed_file_index)?;

        // Read the PackFile Index.
        let mut pack_file_index_position: usize = 0;

        // First, we decode every entry in the PackFile index and store it. It's encoded in StringU8 terminated in 00,
        // so we just read them char by char until hitting 0, then decode the next one and so on.
        // NOTE: This doesn't deal with encryption, as we haven't seen any encrypted PackFile with data in this index.
        for _ in 0..pack_file_count {
            let pack_file_name = pack_file_index.decode_packedfile_string_u8_0terminated(pack_file_index_position, &mut pack_file_index_position)?;
            self.pack_files.push(pack_file_name);
        }

        // Prepare the needed stuff to read the PackedFiles.
        let mut index_position: usize = 0;
        let pack_file = Arc::new(Mutex::new(pack_file));
        for _ in 0..packed_file_count {

            // Get his size. If it's encrypted, decrypt it first.
            let size = packed_file_index.decode_integer_u32(index_position)?;

            let timestamp = if self.bitmask.contains(PFHFlags::HAS_INDEX_WITH_TIMESTAMPS) {
                (packed_file_index.decode_integer_i64(index_position + 4)? / WINDOWS_TICK) - SEC_TO_UNIX_EPOCH
            } else { 0 };

            // Update his offset, and get his compression data if it has it.
            index_position += if self.bitmask.contains(PFHFlags::HAS_INDEX_WITH_TIMESTAMPS) { PATH_FILE_INDEX_PATH_OFFSET + TIMESTAMP_SIZE } else { PATH_FILE_INDEX_PATH_OFFSET };

            // Get his path. Like the PackFile index, it's a StringU8 terminated in 00. We get it and split it in folders for easy use.
            let path = packed_file_index.decode_packedfile_string_u8_0terminated(index_position, &mut index_position)?;
            let path = path.split('\\').map(|x| x.to_owned()).collect::<Vec<String>>();
            let packed_file_type = PackedFileType::get_packed_file_type(&path);

            let load = match types_to_load {
                Some(ref types_to_load) => types_to_load.contains(&packed_file_type),
                None => true,
            };

            if load {
                let is_compressed = false;
                let is_encrypted = None;

                // Once we are done, we create the PackedFile and add it to the PackedFile list.
                let raw_data = RawPackedFile::read_from_data(
                    path,
                    self.get_file_name().to_string(),
                    timestamp,
                    is_compressed,
                    is_encrypted,
                    PackedFileData::OnDisk(RawOnDisk::new(
                        pack_file.clone(),
                        data_position,
                        size,
                        is_compressed,
                        is_encrypted
                    ))
                );

                let mut packed_file = PackedFile::new_from_raw(&raw_data);

                // If this is a notes PackedFile, save the notes and forget about the PackedFile. Otherwise, save the PackedFile.
                if packed_file.get_path() == [RESERVED_NAME_NOTES] {
                    if let Ok(data) = packed_file.get_raw_data_and_keep_it() {
                        if let Ok(data) = data.decode_string_u8(0, data.len()) {
                            self.notes = Some(data);
                        }
                    }
                }

                else if packed_file.get_path() == [RESERVED_NAME_SETTINGS] {
                    if let Ok(data) = packed_file.get_raw_data_and_keep_it() {
                        self.settings = if let Ok(settings) = PackFileSettings::load(&data) {
                            settings
                        } else {
                            PackFileSettings::default()
                        };
                    }
                }
                else {
                    self.packed_files.push(packed_file);
                }
            }

            data_position += u64::from(size);
        }

        // If at this point we have not reached the end of the PackFile, there is something wrong with it.
        if data_position != pack_file_len {
            return Err(ErrorKind::PackFileSizeIsNotWhatWeExpect(pack_file_len, data_position).into())
        }

        // If we disabled lazy-loading, load every PackedFile to memory.
        if !use_lazy_loading {
            self.packed_files.par_iter_mut().try_for_each(|packed_file| packed_file.get_ref_mut_raw().load_data())?
        }

        // Return our PackFile.
        Ok(())
    }

    /// This function tries to save a `PackFile` to a file in the filesystem.
    ///
    /// If no path is passed, the `PackFile` will be saved in his current path.
    /// If a path is passed as `new_path` the `PackFile` will be saved in that path.
    pub fn save_pfh3(&mut self, new_path: Option<PathBuf>) -> Result<()> {

        // If any of the problematic masks in the header is set or is one of CA's, return an error.
        if !self.is_editable(*SETTINGS.read().unwrap().settings_bool.get("allow_editing_of_ca_packfiles").unwrap()) { return Err(ErrorKind::PackFileIsNonEditable.into()) }

        // If we receive a new path, update it. Otherwise, ensure the file actually exists on disk.
        if let Some(path) = new_path { self.set_file_path(&path)?; }
        else if !self.get_file_path().is_file() { return Err(ErrorKind::PackFileIsNotAFile.into()) }

        // Before everything else, add the file for the notes if we have them. We'll remove it later, after the file has been saved.
        if let Some(note) = &self.notes {
            let mut data = vec![];
            data.encode_string_u8(&note);
            let raw_data = RawPackedFile::read_from_vec(vec![RESERVED_NAME_NOTES.to_owned()], self.get_file_name(), 0, false, data);
            let packed_file = PackedFile::new_from_raw(&raw_data);
            self.packed_files.push(packed_file);
        }

        // Saving PackFile settings.
        let mut data = vec![];
        data.write_all(&to_string_pretty(&self.settings)?.as_bytes())?;
        let raw_data = RawPackedFile::read_from_vec(vec![RESERVED_NAME_SETTINGS.to_owned()], self.get_file_name(), 0, false, data);
        let packed_file = PackedFile::new_from_raw(&raw_data);
        self.packed_files.push(packed_file);

        // For some bizarre reason, if the PackedFiles are not alphabetically sorted they may or may not crash the game for particular people.
        // So, to fix it, we have to sort all the PackedFiles here by path.
        // NOTE: This sorting has to be CASE INSENSITIVE. This means for "ac", "Ab" and "aa" it'll be "aa", "Ab", "ac".
        self.packed_files.sort_unstable_by_key(|a| a.get_path().join("\\").to_lowercase());

        // We ensure that all the data is loaded and in his right form (compressed/encrypted) before attempting to save.
        // We need to do this here because we need later on their compressed size.
        self.packed_files.par_iter_mut().try_for_each(|x| x.encode())?;

        // First we encode the indexes and the data (just in case we compressed it).
        let mut pack_file_index = vec![];
        let mut packed_file_index = vec![];

        for pack_file in &self.pack_files {
            pack_file_index.extend_from_slice(pack_file.as_bytes());
            pack_file_index.push(0);
        }

        for packed_file in &self.packed_files {
            packed_file_index.encode_integer_u32(packed_file.get_ref_raw().get_size());

            if self.bitmask.contains(PFHFlags::HAS_INDEX_WITH_TIMESTAMPS) {
                packed_file_index.encode_integer_i64(packed_file.get_ref_raw().get_timestamp());
            }

            packed_file_index.append(&mut packed_file.get_path().join("\\").as_bytes().to_vec());
            packed_file_index.push(0);
        }

        // Create the file to save to, and save the header and the indexes.
        let mut file = BufWriter::new(File::create(&self.file_path)?);

        // Write the entire header.
        let mut header = vec![];
        header.encode_string_u8(&self.pfh_version.get_value());
        header.encode_integer_u32(self.bitmask.bits | self.pfh_file_type.get_value());
        header.encode_integer_u32(self.pack_files.len() as u32);
        header.encode_integer_u32(pack_file_index.len() as u32);
        header.encode_integer_u32(self.packed_files.len() as u32);
        header.encode_integer_u32(packed_file_index.len() as u32);

        self.timestamp = get_current_time();
        header.encode_integer_i64((self.timestamp + SEC_TO_UNIX_EPOCH) * WINDOWS_TICK);

        // Write the indexes and the data of the PackedFiles. No need to keep the data, as it has been preloaded before.
        file.write_all(&header)?;
        file.write_all(&pack_file_index)?;
        file.write_all(&packed_file_index)?;
        for packed_file in &self.packed_files {
            let data = packed_file.get_ref_raw().get_raw_data()?;
            file.write_all(&data)?;
        }

        // Remove again the reserved PackedFiles.
        self.remove_packed_file_by_path(&[RESERVED_NAME_NOTES.to_owned()]);
        self.remove_packed_file_by_path(&[RESERVED_NAME_SETTINGS.to_owned()]);

        // If nothing has failed, return success.
        Ok(())
    }
}
